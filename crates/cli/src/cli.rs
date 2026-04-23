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
    /// 서브에이전트 시작 (PreToolUse hook, matcher: Task)
    SubagentStart,
    /// 서브에이전트 완료 (PostToolUse hook, matcher: Task)
    SubagentStop,
}

/// Subagent(Task 도구) 이벤트 전용 payload. SubagentStart/Stop 에만 채워지며,
/// 일반 토스트(Stop/Idle/Permission)에는 None 으로 전달된다.
/// 이전에는 동일한 7개 필드를 CliRequest/Args/NotifyRequestArgs 3곳에 병렬 선언해
/// 파라미터 sprawl 이 발생했다. 단일 구조체로 묶어 변경 비용 집중화.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SubagentPayload {
    /// Claude Code session id (hook payload `session_id`). 동일 세션의 시작/종료 매칭 보조용.
    #[serde(default)]
    pub session_id: Option<String>,
    /// Claude Code transcript JSONL 경로. 본체가 tail 리더로 진행 상황을 실시간 읽을 때 사용.
    #[serde(default)]
    pub transcript_path: Option<String>,
    /// tool_input 기반 안정적 해시 id. PreToolUse/PostToolUse 에서 동일 id 로 매칭.
    #[serde(default)]
    pub subagent_id: Option<String>,
    /// Start: tool_input.subagent_type (서브에이전트 유형 이름).
    #[serde(default)]
    pub subagent_type: Option<String>,
    /// Start: tool_input.description (간단 요약, ~200자 권장).
    #[serde(default)]
    pub description: Option<String>,
    /// Start: tool_input.prompt (실제 지시문, ~500자 권장).
    #[serde(default)]
    pub prompt: Option<String>,
    /// Stop: tool_response 를 텍스트로 평탄화한 최종 결과 (~1000자 권장).
    #[serde(default)]
    pub result: Option<String>,
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
    /// Claude Code 플러그인 → Dokkaebi 작업 알림 전달.
    /// 제목/본문 생성 규칙은 본체 `compose_claude_notification_text` 참조.
    Notify {
        kind: NotifyKind,
        /// 알림 발생 위치 (어느 워크스페이스에 표시할지 라우팅 힌트)
        cwd: Option<String>,
        /// 알림 송신 프로세스 PID (dispatch.sh의 `$PPID` = Claude 프로세스).
        /// Dokkaebi 본체는 이 PID의 parent chain을 따라가며 각 터미널의
        /// shell PID와 일치하는 터미널을 정확히 식별한다.
        ///
        /// `#[serde(default)]`: 구 cli(해당 필드 없음)에서 새 본체로 IPC 시
        /// 역직렬화 실패를 막기 위해 누락 시 None으로 처리한다.
        #[serde(default)]
        pid: Option<u32>,
        /// cli 프로세스의 Win32 parent chain(자기 자신 포함, 최상위까지).
        /// cli는 IPC 호출 전 bash/dispatch.sh가 아직 살아있는 시점에 Toolhelp
        /// snapshot으로 완전한 chain을 확보한다. 본체는 이 vector를 그대로
        /// 사용해 각 터미널의 shell PID와 매칭한다(dispatch.sh가 백그라운드
        /// 실행 후 종료해 본체 쪽 sysinfo에서 parent 추적이 끊기는 문제 해결).
        #[serde(default)]
        ancestors: Vec<u32>,
        /// Stop 이벤트: 사용자가 직전에 입력한 프롬프트 요약 (최대 200자 truncate).
        #[serde(default)]
        notify_prompt: Option<String>,
        /// Stop 이벤트: 어시스턴트 마지막 응답 요약 (최대 200자 truncate).
        #[serde(default)]
        notify_response: Option<String>,
        /// Permission 이벤트: 요청 도구 이름 (예: "Bash", "Edit").
        #[serde(default)]
        notify_tool_name: Option<String>,
        /// Permission 이벤트: 도구 입력 preview (command / file_path 등, 최대 120자 truncate).
        #[serde(default)]
        notify_tool_preview: Option<String>,
        /// Idle 이벤트: Claude Code가 hook payload 로 보낸 원본 메시지.
        #[serde(default)]
        notify_idle_summary: Option<String>,
        /// Subagent 이벤트 전용 payload. SubagentStart/Stop 에만 Some.
        #[serde(default)]
        subagent: Option<SubagentPayload>,
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
