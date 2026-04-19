use collections::HashMap;
pub use ipc_channel::ipc;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct IpcHandshake {
    pub requests: ipc::IpcSender<CliRequest>,
    pub responses: ipc::IpcReceiver<CliResponse>,
}

/// Claude Code 플러그인이 송신하는 작업 알림 종류
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotifyKind {
    /// 작업 완료 (Stop hook)
    Stop,
    /// 사용자 입력 대기 (Notification hook, matcher: idle_prompt)
    Idle,
    /// 도구 사용 권한 요청 (PermissionRequest hook)
    Permission,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CliRequest {
    Open {
        paths: Vec<String>,
        urls: Vec<String>,
        diff_paths: Vec<[String; 2]>,
        diff_all: bool,
        wsl: Option<String>,
        wait: bool,
        open_new_workspace: Option<bool>,
        reuse: bool,
        env: Option<HashMap<String, String>>,
        user_data_dir: Option<String>,
    },
    /// Claude Code 플러그인 → Dokkaebi 작업 알림 전달
    /// 워크스페이스 토스트 또는 전역 알림으로 표시한다.
    Notify {
        kind: NotifyKind,
        title: String,
        message: String,
        /// 알림 발생 위치 (어느 워크스페이스에 표시할지 라우팅 힌트)
        cwd: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CliResponse {
    Ping,
    Stdout { message: String },
    Stderr { message: String },
    Exit { status: i32 },
}

/// When Zed started not as an *.app but as a binary (e.g. local development),
/// there's a possibility to tell it to behave "regularly".
///
/// Note that in the main zed binary, this variable is unset after it's read for the first time,
/// therefore it should always be accessed through the `FORCE_CLI_MODE` static.
pub const FORCE_CLI_MODE_ENV_VAR_NAME: &str = "ZED_FORCE_CLI_MODE";
