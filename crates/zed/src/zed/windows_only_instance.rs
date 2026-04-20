use std::{sync::Arc, thread::JoinHandle};

use anyhow::Context;
use cli::{CliRequest, CliResponse, IpcHandshake, ipc::IpcOneShotServer};
use parking_lot::Mutex;
use release_channel::app_identifier;
use util::ResultExt;
use windows::{
    Win32::{
        Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GENERIC_WRITE, GetLastError, HANDLE},
        Storage::FileSystem::{
            CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_MODE, OPEN_EXISTING,
            PIPE_ACCESS_INBOUND, ReadFile, WriteFile,
        },
        System::{
            Pipes::{
                ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_MESSAGE,
                PIPE_TYPE_MESSAGE, PIPE_WAIT,
            },
            Threading::CreateMutexW,
        },
    },
    core::HSTRING,
};

use crate::{Args, OpenListener, RawOpenRequest};

#[inline]
fn is_first_instance() -> bool {
    unsafe {
        CreateMutexW(
            None,
            false,
            &HSTRING::from(format!("{}-Instance-Mutex", app_identifier())),
        )
        .expect("Unable to create instance mutex.")
    };
    unsafe { GetLastError() != ERROR_ALREADY_EXISTS }
}

pub fn handle_single_instance(opener: OpenListener, args: &Args) -> bool {
    let is_first_instance = is_first_instance();
    if is_first_instance {
        // We are the first instance, listen for messages sent from other instances
        std::thread::Builder::new()
            .name("EnsureSingleton".to_owned())
            .spawn(move || {
                with_pipe(&|url| {
                    opener.open(RawOpenRequest {
                        urls: vec![url],
                        ..Default::default()
                    })
                })
            })
            .unwrap();
    } else {
        // 이미 실행 중인 본체가 있음. 그 본체에 visible window가 없으면 UI
        // 초기화에 실패한 좀비일 가능성이 높으므로 경고 로그를 남긴다.
        // cli의 handshake 워치독이 좀비를 kill하지 못한 경우의 안전망.
        if !has_visible_dokkaebi_window() {
            log::warn!(
                "이미 실행 중인 Dokkaebi 본체가 감지되었으나 visible window가 \
                 없습니다. UI 초기화에 실패한 좀비 상태일 가능성이 높습니다. \
                 작업 관리자에서 dokkaebi.exe를 강제 종료한 뒤 재실행하세요."
            );
        }
        if !args.foreground {
            // We are not the first instance, send args to the first instance
            send_args_to_instance(args).log_err();
        }
    }

    is_first_instance
}

/// Dokkaebi 실행 파일과 연결된 visible top-level window가 하나라도 존재하는지
/// 확인한다. 좀비 본체(mutex는 잡았으나 UI가 없음) 탐지용. 현재 프로세스 자신의
/// window도 포함될 수 있으므로 "정상 본체 + 자기 자신"이 있는 경우도 true를
/// 반환한다. 호출자는 `is_first_instance == false`(즉 다른 본체 존재) 조건에서만
/// 사용할 것.
fn has_visible_dokkaebi_window() -> bool {
    use std::sync::atomic::{AtomicBool, Ordering};
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
    };
    use windows::core::BOOL;

    static FOUND: AtomicBool = AtomicBool::new(false);

    unsafe extern "system" fn enum_proc(hwnd: HWND, _lparam: LPARAM) -> BOOL {
        if unsafe { IsWindowVisible(hwnd) }.as_bool() {
            let mut pid: u32 = 0;
            unsafe {
                GetWindowThreadProcessId(hwnd, Some(&mut pid));
            }
            if pid != 0 && is_dokkaebi_pid(pid) {
                FOUND.store(true, Ordering::SeqCst);
                return BOOL(0); // stop enumeration
            }
        }
        BOOL(1)
    }

    FOUND.store(false, Ordering::SeqCst);
    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM(0));
    }
    FOUND.load(Ordering::SeqCst)
}

/// 주어진 PID의 실행 파일 이름이 `dokkaebi.exe` 인지 확인한다.
/// `QueryFullProcessImageNameW` 결과를 `Path::file_name` 으로 추출해 UNC 접두
/// (`\\?\`)나 구분자 혼용에도 안정적으로 동작한다.
fn is_dokkaebi_pid(pid: u32) -> bool {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::Path;
    use windows::Win32::Foundation::{CloseHandle, MAX_PATH};
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };

    const DOKKAEBI_EXE: &str = "dokkaebi.exe";

    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return false;
        };
        let mut buf = [0u16; MAX_PATH as usize];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut size,
        )
        .is_ok();
        let _ = CloseHandle(handle);
        if !ok {
            return false;
        }
        let os_path = OsString::from_wide(&buf[..size as usize]);
        Path::new(&os_path)
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case(DOKKAEBI_EXE))
    }
}

fn with_pipe(f: &dyn Fn(String)) {
    let pipe = unsafe {
        CreateNamedPipeW(
            &HSTRING::from(format!("\\\\.\\pipe\\{}-Named-Pipe", app_identifier())),
            PIPE_ACCESS_INBOUND,
            PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
            1,
            128,
            128,
            0,
            None,
        )
    };
    if pipe.is_invalid() {
        log::error!("Failed to create named pipe: {:?}", unsafe {
            GetLastError()
        });
        return;
    }

    loop {
        if let Some(message) = retrieve_message_from_pipe(pipe)
            .context("Failed to read from named pipe")
            .log_err()
        {
            f(message);
        }
    }
}

fn retrieve_message_from_pipe(pipe: HANDLE) -> anyhow::Result<String> {
    unsafe { ConnectNamedPipe(pipe, None)? };
    let message = retrieve_message_from_pipe_inner(pipe);
    unsafe { DisconnectNamedPipe(pipe).log_err() };
    message
}

fn retrieve_message_from_pipe_inner(pipe: HANDLE) -> anyhow::Result<String> {
    let mut buffer = [0u8; 128];
    unsafe {
        ReadFile(pipe, Some(&mut buffer), None, None)?;
    }
    let message = std::ffi::CStr::from_bytes_until_nul(&buffer)?;
    Ok(message.to_string_lossy().into_owned())
}

// This part of code is mostly from crates/cli/src/main.rs
fn send_args_to_instance(args: &Args) -> anyhow::Result<()> {
    if let Some(dock_menu_action_idx) = args.dock_action {
        let url = format!("zed-dock-action://{}", dock_menu_action_idx);
        return write_message_to_instance_pipe(url.as_bytes());
    }

    let (server, server_name) =
        IpcOneShotServer::<IpcHandshake>::new().context("Handshake before Zed spawn")?;
    let url = format!("zed-cli://{server_name}");

    let request = {
        let mut paths = vec![];
        let mut urls = vec![];
        let mut diff_paths = vec![];
        for path in args.paths_or_urls.iter() {
            match std::fs::canonicalize(&path) {
                Ok(path) => paths.push(path.to_string_lossy().into_owned()),
                Err(error) => {
                    if path.starts_with("zed://")
                        || path.starts_with("http://")
                        || path.starts_with("https://")
                        || path.starts_with("file://")
                        || path.starts_with("ssh://")
                    {
                        urls.push(path.clone());
                    } else {
                        log::error!("error parsing path argument: {}", error);
                    }
                }
            }
        }

        for path in args.diff.chunks(2) {
            let old = std::fs::canonicalize(&path[0]).log_err();
            let new = std::fs::canonicalize(&path[1]).log_err();
            if let Some((old, new)) = old.zip(new) {
                diff_paths.push([
                    old.to_string_lossy().into_owned(),
                    new.to_string_lossy().into_owned(),
                ]);
            }
        }

        CliRequest::Open {
            paths,
            urls,
            diff_paths,
            diff_all: false,
            wait: false,
            wsl: args.wsl.clone(),
            open_new_workspace: None,
            reuse: false,
            env: None,
            user_data_dir: args.user_data_dir.clone(),
        }
    };

    let exit_status = Arc::new(Mutex::new(None));
    let sender: JoinHandle<anyhow::Result<()>> = std::thread::Builder::new()
        .name("CliReceiver".to_owned())
        .spawn({
            let exit_status = exit_status.clone();
            move || {
                let (_, handshake) = server.accept().context("Handshake after Zed spawn")?;
                let (tx, rx) = (handshake.requests, handshake.responses);

                tx.send(request)?;

                while let Ok(response) = rx.recv() {
                    match response {
                        CliResponse::Ping => {}
                        CliResponse::Stdout { message } => log::info!("{message}"),
                        CliResponse::Stderr { message } => log::error!("{message}"),
                        CliResponse::Exit { status } => {
                            exit_status.lock().replace(status);
                            return Ok(());
                        }
                    }
                }
                Ok(())
            }
        })
        .unwrap();

    write_message_to_instance_pipe(url.as_bytes())?;
    sender.join().unwrap()?;
    if let Some(exit_status) = exit_status.lock().take() {
        std::process::exit(exit_status);
    }
    Ok(())
}

fn write_message_to_instance_pipe(message: &[u8]) -> anyhow::Result<()> {
    unsafe {
        let pipe = CreateFileW(
            &HSTRING::from(format!("\\\\.\\pipe\\{}-Named-Pipe", app_identifier())),
            GENERIC_WRITE.0,
            FILE_SHARE_MODE::default(),
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES::default(),
            None,
        )?;
        WriteFile(pipe, Some(message), None, None)?;
        CloseHandle(pipe)?;
    }
    Ok(())
}
