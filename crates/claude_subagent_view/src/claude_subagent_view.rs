//! Claude Code 서브에이전트 뷰 탭 크레이트.
//!
//! - `state` 모듈: 활성/완료된 서브에이전트 상태를 gpui Global 로 보관.
//!   `crates/zed` 쪽의 `open_listener` (IPC 수신) + `claude_subagent_tail`
//!   (transcript JSONL tail 리더) 가 write, 본 크레이트의 view 가 read.
//! - `view` 모듈: 서브에이전트 단위로 열리는 워크스페이스 Item. 메타/로그/결과 렌더.

pub mod process_snapshot;
pub mod state;
pub mod view;

pub use state::{
    ClaudeSubagentStore, SubagentId, SubagentLogEntry, SubagentPanelPosition, SubagentState,
    SubagentStatus, append_log, claude_code_settings, contains, init, mark_stopped, snapshot,
    status_only, try_mark_tail_spawned, upsert_start,
};
pub use view::{ClaudeSubagentView, open_subagent_view};
