//! portable-pty 기반 PTY 어댑터
//!
//! alacritty_terminal의 tty + EventLoop를 대체한다.
//! PTY 생성, 읽기 스레드, 쓰기/리사이즈/종료를 담당하며
//! alacritty의 `Term<ZedListener>` 그리드 파서와 연결된다.

use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use alacritty_terminal::event::{Event as AlacTermEvent, EventListener};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::Term;
use futures::channel::mpsc::UnboundedSender;
use portable_pty::{
    native_pty_system, Child, CommandBuilder, MasterPty, PtySize,
};
use alacritty_terminal::vte::ansi;

use crate::shell_integration::{Osc133Scanner, ShellIntegrationEvent};
use crate::ZedListener;

/// PTY 읽기 버퍼 크기 (alacritty READ_BUFFER_SIZE와 동일)
const READ_BUFFER_SIZE: usize = 0x10_0000;

/// Term 잠금 상태에서 처리할 최대 바이트 (alacritty MAX_LOCKED_READ와 동일)
const MAX_LOCKED_READ: usize = u16::MAX as usize;

/// PTY 핸들 — 쓰기/리사이즈/프로세스 관리
pub struct PtyHandle {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    child: Mutex<Box<dyn Child + Send + Sync>>,
}

impl PtyHandle {
    /// PTY에 데이터를 쓴다.
    pub fn write_bytes(&self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        if let Ok(mut writer) = self.writer.lock() {
            writer.write_all(data).ok();
        }
    }

    /// PTY 크기를 변경한다.
    pub fn resize(&self, size: PtySize) -> anyhow::Result<()> {
        if let Ok(master) = self.master.lock() {
            master
                .resize(size)
                .map_err(|e| anyhow::anyhow!("PTY resize 실패: {}", e))
        } else {
            Err(anyhow::anyhow!("PTY master lock 실패"))
        }
    }

    /// 자식 프로세스 PID를 반환한다.
    pub fn process_id(&self) -> Option<u32> {
        if let Ok(child) = self.child.lock() {
            child.process_id()
        } else {
            None
        }
    }

    /// 자식 프로세스가 종료했는지 확인한다 (논블로킹).
    pub fn try_wait(&self) -> Option<portable_pty::ExitStatus> {
        if let Ok(mut child) = self.child.lock() {
            child.try_wait().ok().flatten()
        } else {
            None
        }
    }

    /// 자식 프로세스를 강제 종료한다.
    pub fn kill(&self) {
        if let Ok(mut child) = self.child.lock() {
            child.kill().ok();
        }
    }

    #[cfg(windows)]
    /// Windows: 자식 프로세스 핸들을 반환한다 (PEB 읽기용).
    /// portable-pty의 Child를 downcast하여 WaitableChild에서 핸들을 획득한다.
    /// 불가능하면 PID 기반으로 OpenProcess로 대체한다.
    pub fn child_process_handle(&self) -> Option<isize> {
        // portable-pty는 Windows에서 process handle을 직접 노출하지 않으므로
        // PID를 통해 OpenProcess로 핸들을 얻는다.
        let pid = self.process_id()?;
        if pid == 0 {
            return None;
        }
        unsafe {
            use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
            let handle = OpenProcess(
                PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                false,
                pid,
            );
            match handle {
                Ok(h) => Some(h.0 as isize),
                Err(_) => None,
            }
        }
    }
}

/// PTY 생성 파라미터
pub struct PtySpawnParams {
    pub program: String,
    pub args: Vec<String>,
    pub working_directory: Option<std::path::PathBuf>,
    pub env: Vec<(String, String)>,
    pub rows: u16,
    pub cols: u16,
}

/// PTY를 생성하고 자식 프로세스를 실행한다.
///
/// 반환값: (PtyHandle, PTY reader)
/// reader는 `spawn_pty_reader()`에 전달하여 읽기 스레드를 시작한다.
pub fn spawn_pty(
    params: PtySpawnParams,
) -> anyhow::Result<(PtyHandle, Box<dyn Read + Send>)> {
    let pty_system = native_pty_system();

    let size = PtySize {
        rows: params.rows.max(1),
        cols: params.cols.max(1),
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = pty_system
        .openpty(size)
        .map_err(|e| anyhow::anyhow!("PTY 열기 실패: {}", e))?;

    // 자식 프로세스 커맨드 구성
    let mut cmd = CommandBuilder::new(&params.program);
    for arg in &params.args {
        cmd.arg(arg);
    }
    if let Some(ref cwd) = params.working_directory {
        cmd.cwd(cwd);
    }
    for (key, value) in &params.env {
        cmd.env(key.as_str(), value.as_str());
    }

    // 자식 프로세스 실행
    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| anyhow::anyhow!("프로세스 실행 실패: {}", e))?;

    // reader/writer 분리
    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| anyhow::anyhow!("PTY reader 복제 실패: {}", e))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|e| anyhow::anyhow!("PTY writer 획득 실패: {}", e))?;

    let handle = PtyHandle {
        master: Mutex::new(pair.master),
        writer: Arc::new(Mutex::new(writer)),
        child: Mutex::new(child),
    };

    Ok((handle, reader))
}

/// PTY 읽기 스레드를 시작한다.
///
/// alacritty EventLoop의 `pty_read`와 동등한 역할을 하지만
/// `polling`/`piper` 없이 단순 blocking read를 사용한다.
///
/// 읽은 바이트를 `vte::ansi::Processor`로 파싱하여 `Term`에 반영하고,
/// `ZedListener`를 통해 `Event::Wakeup`을 전달한다.
///
/// `shell_events_tx` 가 `Some` 이면 OSC 133 (FinalTerm shell integration) 시퀀스를
/// 병행 검출해 별도 채널로 emit 한다. alac VTE 0.15.0 은 OSC 133 미처리이므로
/// alac 파서에는 무해하게 통과시키되, 본 스캐너가 동일 바이트를 검사한다.
pub fn spawn_pty_reader(
    mut reader: Box<dyn Read + Send>,
    terminal: Arc<FairMutex<Term<ZedListener>>>,
    event_proxy: ZedListener,
    shell_events_tx: Option<UnboundedSender<ShellIntegrationEvent>>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("pty-reader".into())
        .spawn(move || {
            let mut buf = vec![0u8; READ_BUFFER_SIZE];
            let mut parser = ansi::Processor::<ansi::StdSyncHandler>::new();
            let mut osc133 = Osc133Scanner::new();

            loop {
                // PTY에서 데이터 읽기 (blocking)
                let bytes_read = match reader.read(&mut buf) {
                    Ok(0) => {
                        // EOF — 자식 프로세스 종료
                        event_proxy.send_event(AlacTermEvent::Exit);
                        event_proxy.send_event(AlacTermEvent::Wakeup);
                        break;
                    }
                    Ok(n) => n,
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                    Err(e) => {
                        log::error!("PTY 읽기 오류: {}", e);
                        event_proxy.send_event(AlacTermEvent::Exit);
                        event_proxy.send_event(AlacTermEvent::Wakeup);
                        break;
                    }
                };

                // OSC 133 검출 — alac 파서로 넘기기 전에 동일 바이트를 스캐너에 공급
                if let Some(ref tx) = shell_events_tx {
                    let events = osc133.feed(&buf[..bytes_read]);
                    for event in events {
                        if tx.unbounded_send(event).is_err() {
                            // 수신측이 끊겼다 — 이후 매칭은 무시
                            break;
                        }
                    }
                }

                // 읽은 데이터를 파싱하여 Term에 반영
                let mut offset = 0;
                while offset < bytes_read {
                    // Term 잠금 시도 (unfair — pty_read 스레드 우선)
                    let _lease = terminal.lease();
                    let mut term = terminal.lock_unfair();

                    let end = (offset + MAX_LOCKED_READ).min(bytes_read);
                    parser.advance(&mut *term, &buf[offset..end]);
                    offset = end;

                    drop(term);
                    drop(_lease);
                }

                // 파싱된 바이트가 동기화되지 않은 경우 Wakeup 이벤트 전송
                if parser.sync_bytes_count() < bytes_read && bytes_read > 0 {
                    event_proxy.send_event(AlacTermEvent::Wakeup);
                }
            }
        })
        .expect("pty-reader 스레드 생성 실패")
}
