use alacritty_terminal::tty::Pty;
use gpui::{Context, Task};
use parking_lot::{MappedRwLockReadGuard, Mutex, RwLock, RwLockReadGuard};
#[cfg(target_os = "windows")]
use std::num::NonZeroU32;
#[cfg(unix)]
use std::os::fd::AsRawFd;
use std::{path::PathBuf, sync::Arc};

#[cfg(target_os = "windows")]
use windows::Win32::{Foundation::HANDLE, System::Threading::GetProcessId};

use sysinfo::{Pid, Process, ProcessRefreshKind, RefreshKind, System, UpdateKind};

/// Windows에서 ConPTY 원래 핸들을 사용하여 프로세스의 PEB에서 직접 cwd를 읽는다.
/// sysinfo가 OpenProcess에 실패하더라도 원래 핸들은 충분한 권한을 가지고 있다.
#[cfg(target_os = "windows")]
mod win_peb_cwd {
    use std::ffi::c_void;
    use std::mem;
    use std::path::PathBuf;

    type NTSTATUS = i32;

    /// NtQueryInformationProcess 함수 시그니처
    type NtQueryInformationProcessFn = unsafe extern "system" fn(
        process_handle: *mut c_void,
        process_information_class: u32,
        process_information: *mut c_void,
        process_information_length: u32,
        return_length: *mut u32,
    ) -> NTSTATUS;

    const PROCESS_BASIC_INFORMATION_CLASS: u32 = 0;

    #[repr(C)]
    struct ProcessBasicInformation {
        exit_status: i32,
        // 패딩 (64비트에서 포인터 정렬을 위해 컴파일러가 자동 삽입)
        peb_base_address: *mut c_void,
        affinity_mask: usize,
        base_priority: i32,
        // 패딩
        unique_process_id: usize,
        inherited_from: usize,
    }

    /// PEB 구조체 (ProcessParameters 필드까지만 정의)
    #[repr(C)]
    struct Peb {
        _reserved1: [u8; 2],
        _being_debugged: u8,
        _reserved2: u8,
        _reserved3: [*mut c_void; 2], // Mutant, ImageBaseAddress
        _ldr: *mut c_void,
        process_parameters: *mut c_void,
    }

    /// UNICODE_STRING (64비트)
    #[repr(C)]
    struct UnicodeString {
        length: u16,
        _maximum_length: u16,
        buffer: *mut u16,
    }

    /// CURDIR
    #[repr(C)]
    struct CurDir {
        dos_path: UnicodeString,
        _handle: *mut c_void,
    }

    /// RTL_USER_PROCESS_PARAMETERS (CurrentDirectory까지만 정의)
    #[repr(C)]
    struct RtlUserProcessParameters {
        _maximum_length: u32,
        _length: u32,
        _flags: u32,
        _debug_flags: u32,
        _console_handle: *mut c_void,
        _console_flags: u32,
        _standard_input: *mut c_void,
        _standard_output: *mut c_void,
        _standard_error: *mut c_void,
        current_directory: CurDir,
    }

    unsafe extern "system" {
        fn LoadLibraryW(name: *const u16) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
        fn ReadProcessMemory(
            process: *mut c_void,
            base_address: *const c_void,
            buffer: *mut c_void,
            size: usize,
            number_of_bytes_read: *mut usize,
        ) -> i32;
    }

    /// 원래 프로세스 핸들을 사용하여 PEB에서 현재 작업 디렉토리를 읽는다.
    pub fn read_process_cwd(handle: isize) -> Option<PathBuf> {
        unsafe {
            // ntdll.dll에서 NtQueryInformationProcess를 동적 로드
            let ntdll_name: Vec<u16> = "ntdll.dll\0".encode_utf16().collect();
            let ntdll = LoadLibraryW(ntdll_name.as_ptr());
            if ntdll.is_null() {
                log::debug!("PEB cwd: ntdll.dll 로드 실패");
                return None;
            }

            let proc_name = b"NtQueryInformationProcess\0";
            let nt_query_ptr = GetProcAddress(ntdll, proc_name.as_ptr());
            if nt_query_ptr.is_null() {
                log::debug!("PEB cwd: NtQueryInformationProcess 찾기 실패");
                return None;
            }
            let nt_query: NtQueryInformationProcessFn = mem::transmute(nt_query_ptr);

            let h = handle as *mut c_void;

            // PEB 주소 조회
            let mut pbi: ProcessBasicInformation = mem::zeroed();
            let status = nt_query(
                h,
                PROCESS_BASIC_INFORMATION_CLASS,
                &mut pbi as *mut _ as *mut c_void,
                mem::size_of::<ProcessBasicInformation>() as u32,
                std::ptr::null_mut(),
            );
            if status != 0 {
                log::debug!("PEB cwd: NtQueryInformationProcess 실패 (status=0x{:x})", status);
                return None;
            }
            if pbi.peb_base_address.is_null() {
                log::debug!("PEB cwd: PEB 주소가 null");
                return None;
            }

            // PEB 읽기
            let mut peb: Peb = mem::zeroed();
            let mut bytes_read: usize = 0;
            if ReadProcessMemory(
                h,
                pbi.peb_base_address,
                &mut peb as *mut _ as *mut c_void,
                mem::size_of::<Peb>(),
                &mut bytes_read,
            ) == 0
            {
                log::debug!("PEB cwd: PEB 읽기 실패");
                return None;
            }
            if peb.process_parameters.is_null() {
                log::debug!("PEB cwd: ProcessParameters 주소가 null");
                return None;
            }

            // RTL_USER_PROCESS_PARAMETERS 읽기
            let mut params: RtlUserProcessParameters = mem::zeroed();
            if ReadProcessMemory(
                h,
                peb.process_parameters,
                &mut params as *mut _ as *mut c_void,
                mem::size_of::<RtlUserProcessParameters>(),
                &mut bytes_read,
            ) == 0
            {
                log::debug!("PEB cwd: ProcessParameters 읽기 실패");
                return None;
            }

            // CurrentDirectory.DosPath 읽기
            let length = params.current_directory.dos_path.length as usize;
            let buffer_ptr = params.current_directory.dos_path.buffer;
            if length == 0 || buffer_ptr.is_null() {
                log::debug!("PEB cwd: DosPath가 비어있거나 null (length={})", length);
                return None;
            }

            let mut buffer: Vec<u16> = vec![0u16; length / 2 + 1];
            if ReadProcessMemory(
                h,
                buffer_ptr as *const c_void,
                buffer.as_mut_ptr() as *mut c_void,
                length,
                &mut bytes_read,
            ) == 0
            {
                log::debug!("PEB cwd: DosPath 문자열 읽기 실패");
                return None;
            }

            let actual_chars = bytes_read / 2;
            let cwd = String::from_utf16_lossy(&buffer[..actual_chars]);
            let cwd = cwd.trim_end_matches('\0').trim_end_matches('\\');
            if cwd.is_empty() {
                return None;
            }

            let path = PathBuf::from(format!("{}\\", cwd));
            log::debug!("PEB cwd: 직접 읽기 성공 = {:?}", path);
            Some(path)
        }
    }
}

use crate::{Event, Terminal};

#[derive(Clone, Copy)]
pub struct ProcessIdGetter {
    /// Unix: file descriptor (i32), Windows: HANDLE (pointer-sized)
    handle: isize,
    fallback_pid: u32,
}

impl ProcessIdGetter {
    pub fn fallback_pid(&self) -> Pid {
        Pid::from_u32(self.fallback_pid)
    }

    /// 원래 프로세스 핸들을 반환한다 (Windows: ConPTY 핸들, Unix: PTY fd).
    pub fn handle(&self) -> isize {
        self.handle
    }
}

#[cfg(unix)]
impl ProcessIdGetter {
    fn new(pty: &Pty) -> ProcessIdGetter {
        ProcessIdGetter {
            handle: pty.file().as_raw_fd() as isize,
            fallback_pid: pty.child().id(),
        }
    }

    fn pid(&self) -> Option<Pid> {
        // Negative pid means error.
        // Zero pid means no foreground process group is set on the PTY yet.
        // Avoid killing the current process by returning a zero pid.
        let pid = unsafe { libc::tcgetpgrp(self.handle as i32) };
        if pid > 0 {
            return Some(Pid::from_u32(pid as u32));
        }

        if self.fallback_pid > 0 {
            return Some(Pid::from_u32(self.fallback_pid));
        }

        None
    }
}

#[cfg(windows)]
impl ProcessIdGetter {
    fn new(pty: &Pty) -> ProcessIdGetter {
        let child = pty.child_watcher();
        let handle = child.raw_handle();
        let fallback_pid = child.pid().unwrap_or_else(|| unsafe {
            NonZeroU32::new_unchecked(GetProcessId(HANDLE(handle as *mut std::ffi::c_void)))
        });

        ProcessIdGetter {
            handle: handle as isize,
            fallback_pid: u32::from(fallback_pid),
        }
    }

    fn pid(&self) -> Option<Pid> {
        let pid = unsafe { GetProcessId(HANDLE(self.handle as *mut std::ffi::c_void)) };
        // the GetProcessId may fail and returns zero, which will lead to a stack overflow issue
        if pid == 0 {
            // in the builder process, there is a small chance, almost negligible,
            // that this value could be zero, which means child_watcher returns None,
            // GetProcessId returns 0.
            if self.fallback_pid == 0 {
                return None;
            }
            return Some(Pid::from_u32(self.fallback_pid));
        }
        Some(Pid::from_u32(pid))
    }
}

#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub name: String,
    pub cwd: PathBuf,
    pub argv: Vec<String>,
}

/// Fetches Zed-relevant Pseudo-Terminal (PTY) process information
pub struct PtyProcessInfo {
    system: RwLock<System>,
    refresh_kind: ProcessRefreshKind,
    pid_getter: ProcessIdGetter,
    pub current: RwLock<Option<ProcessInfo>>,
    task: Mutex<Option<Task<()>>>,
}

impl PtyProcessInfo {
    pub fn new(pty: &Pty) -> PtyProcessInfo {
        let process_refresh_kind = ProcessRefreshKind::nothing()
            .with_cmd(UpdateKind::Always)
            .with_cwd(UpdateKind::Always)
            .with_exe(UpdateKind::Always);
        let refresh_kind = RefreshKind::nothing().with_processes(process_refresh_kind);
        let system = System::new_with_specifics(refresh_kind);

        PtyProcessInfo {
            system: RwLock::new(system),
            refresh_kind: process_refresh_kind,
            pid_getter: ProcessIdGetter::new(pty),
            current: RwLock::new(None),
            task: Mutex::new(None),
        }
    }

    pub fn pid_getter(&self) -> &ProcessIdGetter {
        &self.pid_getter
    }

    fn refresh(&self) -> Option<MappedRwLockReadGuard<'_, Process>> {
        let pid = self.pid_getter.pid()?;
        if self.system.write().refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::Some(&[pid]),
            true,
            self.refresh_kind,
        ) == 1
        {
            RwLockReadGuard::try_map(self.system.read(), |system| system.process(pid)).ok()
        } else {
            None
        }
    }

    fn get_child(&self) -> Option<MappedRwLockReadGuard<'_, Process>> {
        let pid = self.pid_getter.fallback_pid();
        RwLockReadGuard::try_map(self.system.read(), |system| system.process(pid)).ok()
    }

    #[cfg(unix)]
    pub(crate) fn kill_current_process(&self) -> bool {
        let Some(pid) = self.pid_getter.pid() else {
            return false;
        };
        unsafe { libc::killpg(pid.as_u32() as i32, libc::SIGKILL) == 0 }
    }

    #[cfg(not(unix))]
    pub(crate) fn kill_current_process(&self) -> bool {
        self.refresh().is_some_and(|process| process.kill())
    }

    pub(crate) fn kill_child_process(&self) -> bool {
        self.get_child().is_some_and(|process| process.kill())
    }

    /// 프로세스 정보를 조회하여 반환한다. `self.current`는 갱신하지 않는다.
    fn load(&self) -> Option<ProcessInfo> {
        let pid = self.pid_getter.pid();
        let process = self.refresh();
        if process.is_none() {
            log::debug!("터미널 cwd: refresh 실패 (pid={:?})", pid);
            // Windows: sysinfo refresh 실패 시에도 원래 핸들로 PEB에서 cwd를 읽는다
            #[cfg(target_os = "windows")]
            {
                if let Some(cwd) = win_peb_cwd::read_process_cwd(self.pid_getter.handle()) {
                    return Some(ProcessInfo {
                        name: String::new(),
                        cwd,
                        argv: Vec::new(),
                    });
                }
            }
            return None;
        }
        let process = process.unwrap();
        let raw_cwd = process.cwd();
        let mut cwd = raw_cwd.map_or(PathBuf::new(), |p| p.to_owned());

        // Windows: sysinfo가 cwd를 가져오지 못한 경우 원래 핸들로 직접 읽기 시도
        #[cfg(target_os = "windows")]
        if cwd.as_os_str().is_empty() {
            if let Some(peb_cwd) = win_peb_cwd::read_process_cwd(self.pid_getter.handle()) {
                log::debug!("터미널 cwd: sysinfo 실패, PEB 폴백 성공 = {:?}", peb_cwd);
                cwd = peb_cwd;
            }
        }

        log::debug!(
            "터미널 cwd: pid={:?}, name={:?}, cwd={:?}",
            pid,
            process.name(),
            cwd
        );

        let info = ProcessInfo {
            name: process.name().to_str()?.to_owned(),
            cwd,
            argv: process
                .cmd()
                .iter()
                .filter_map(|s| s.to_str().map(ToOwned::to_owned))
                .collect(),
        };
        Some(info)
    }

    /// Updates the cached process info, emitting a [`Event::TitleChanged`] event if the Zed-relevant info has changed
    pub fn emit_title_changed_if_changed(self: &Arc<Self>, cx: &mut Context<'_, Terminal>) {
        if self.task.lock().is_some() {
            return;
        }
        let this = self.clone();
        let has_changed = cx.background_executor().spawn(async move {
            // 이전 값을 먼저 캡처한 후 새 값을 조회하여 비교한다.
            let previous = this.current.read().clone();
            let current = this.load();
            let has_changed = match (previous.as_ref(), current.as_ref()) {
                (None, None) => false,
                (Some(prev), Some(now)) => prev.cwd != now.cwd || prev.name != now.name,
                _ => true,
            };
            // 새 값이 있을 때만 current를 갱신한다.
            // sysinfo가 빈 cwd를 반환하면 이전 유효한 cwd를 보존한다.
            if let Some(ref new_info) = current {
                let should_update = if new_info.cwd.as_os_str().is_empty() {
                    // 새 cwd가 빈 경로인 경우: 이전 값이 없을 때만 갱신
                    previous.as_ref().is_none_or(|prev| prev.cwd.as_os_str().is_empty())
                } else {
                    true
                };
                if should_update {
                    *this.current.write() = Some(new_info.clone());
                }
            }
            has_changed
        });
        let this = Arc::downgrade(self);
        *self.task.lock() = Some(cx.spawn(async move |term, cx| {
            if has_changed.await {
                term.update(cx, |_, cx| cx.emit(Event::TitleChanged)).ok();
            }
            if let Some(this) = this.upgrade() {
                this.task.lock().take();
            }
        }));
    }

    pub fn pid(&self) -> Option<Pid> {
        self.pid_getter.pid()
    }
}
