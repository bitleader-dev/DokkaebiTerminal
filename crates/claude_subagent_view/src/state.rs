//! 서브에이전트 상태 저장소.
//!
//! gpui Global 로 구현된 `ClaudeSubagentStore` 가 서브에이전트별 상태를 보관한다.
//! write 경로: `crates/zed` 의 `open_listener::handle_notify_request`
//!            (SubagentStart/Stop IPC 수신), `claude_subagent_tail` (transcript 로그).
//! read 경로: 본 크레이트의 `view::ClaudeSubagentView` (Item 렌더).
//!
//! 라이프사이클:
//! - SubagentStart 수신 → `upsert_start()` → 상태 `Running` 으로 등록
//! - transcript tail 에서 assistant/tool_use/tool_result 감지 → `append_log()`
//! - SubagentStop 수신 → `mark_stopped()` → 상태 `Completed`, 결과 저장
//! - 사용자가 탭을 닫을 때까지 엔트리는 유지 (상태 확인 가능하도록)

use std::time::SystemTime;

use collections::HashMap;
use gpui::{App, AppContext, Entity, EventEmitter, Global};
use serde::{Deserialize, Serialize};
use settings::{ClaudeCodeSettingsContent, SettingsStore};

/// 사용자 설정에서 Claude Code 섹션만 읽어 오는 공용 헬퍼.
/// SettingsStore 가 아직 초기화되지 않았거나 사용자 설정이 비어 있으면 None 을 반환한다.
/// IPC 처리, transcript 정리, 뷰 렌더 세 경로가 동일한 접근 체인을 쓰던 것을 단일 함수로 통합.
pub fn claude_code_settings(cx: &App) -> Option<&ClaudeCodeSettingsContent> {
    cx.try_global::<SettingsStore>()?
        .raw_user_settings()
        .and_then(|user| user.content.claude_code.as_ref())
}

/// tool_input 기반 안정적 해시로 생성된 서브에이전트 식별자.
/// dispatch.sh 가 md5(subagent_type + description + prompt) 로 생성하여 전달.
pub type SubagentId = String;

/// 서브에이전트 뷰 탭 배치 위치. 설정 `claude_code.subagent_panel_position` 값.
/// "새 파일"/"새 터미널" 과 달리 서브에이전트 뷰는 항상 split 으로 열려 터미널과
/// 병렬 표시된다. 같은 방향에 기존 서브에이전트 split 이 있으면 재사용.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SubagentPanelPosition {
    /// 활성 pane 을 오른쪽으로 split 해 우측 pane 에 탭 추가.
    #[default]
    Right,
    /// 활성 pane 을 아래쪽으로 split 해 하단 pane 에 탭 추가.
    Bottom,
}

/// 서브에이전트 실행 상태.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubagentStatus {
    /// PreToolUse 수신 직후. 실행 중.
    Running,
    /// PostToolUse 수신. 정상 종료.
    Completed,
    /// 세션 종료(Stop) 전에 마감 이벤트를 받지 못한 서브에이전트.
    Cancelled,
    /// tool_response 가 에러 표지를 담고 있을 때. (v1: 구분 없이 Completed 처리)
    Failed,
}

/// transcript 로그 타임라인 엔트리. 한 줄 = 한 이벤트(도구 호출/결과 요약).
#[derive(Debug, Clone)]
pub struct SubagentLogEntry {
    pub timestamp: SystemTime,
    /// 짧은 라벨 (예: "Read", "Edit", "tool_result")
    pub label: String,
    /// 상세 미리보기 (긴 payload 는 `claude_subagent_tail::truncate` 로 잘려 들어옴)
    pub detail: String,
}

/// 서브에이전트 엔트리가 유지하는 로그 상한. 초과 시 오래된 엔트리부터 drop.
/// 한 엔트리 detail 이 최대 5_000자 수준이라 500 × 5_000 = 2.5 MB 내외 상한으로 간주.
const MAX_LOG_ENTRIES: usize = 500;

/// 서브에이전트 1건의 상태 전체.
#[derive(Debug, Clone)]
pub struct SubagentState {
    pub id: SubagentId,
    pub session_id: Option<String>,
    pub subagent_type: String,
    pub description: String,
    pub prompt: String,
    /// transcript JSONL 파일 경로 (tail 리더용).
    pub transcript_path: Option<String>,
    /// 이벤트 수신 당시 cwd (탭 라우팅용).
    pub cwd: Option<String>,
    /// 부모 터미널 식별용 PID (토스트 쪽과 동일한 ancestors 원본).
    pub parent_pid: Option<u32>,
    pub status: SubagentStatus,
    pub started_at: SystemTime,
    pub finished_at: Option<SystemTime>,
    pub result: Option<String>,
    pub log: Vec<SubagentLogEntry>,
    /// transcript tail task 가 spawn 되었는지 표지. 동일 id 의 Start IPC 가 거의
    /// 동시에 두 번 들어와도 tail 이 한 번만 spawn 되도록 `try_mark_tail_spawned` 가
    /// store update 안에서 atomic 하게 검사·설정한다.
    pub tail_spawned: bool,
}

impl SubagentState {
    pub fn elapsed(&self) -> std::time::Duration {
        let end = self.finished_at.unwrap_or_else(SystemTime::now);
        end.duration_since(self.started_at).unwrap_or_default()
    }
}

/// 서브에이전트 상태 저장소. gpui Global 로 보관.
/// 내부는 `Entity<StoreInner>` 한 겹을 두어 gpui 구독(Subscription) 흐름을
/// 그대로 사용할 수 있게 한다.
pub struct ClaudeSubagentStore {
    inner: Entity<StoreInner>,
}

impl Global for ClaudeSubagentStore {}

pub struct StoreInner {
    /// view 는 Entity<StoreInner> 를 subscribe 하기 위해서만 참조하고
    /// 데이터는 본 모듈의 free 함수를 통해서만 읽고 쓴다. 외부에서 필드에 직접
    /// 접근할 일이 없으므로 private 유지.
    entries: HashMap<SubagentId, SubagentState>,
    /// 최신 등록 순서 유지용 (UI 정렬).
    order: Vec<SubagentId>,
}

/// Store 엔티티가 emit 하는 이벤트. View 가 구독해 탭을 열거나 리렌더.
#[derive(Debug, Clone)]
pub enum SubagentStoreEvent {
    /// 신규 서브에이전트 등록. view 가 이 이벤트를 받아 탭을 연다.
    Started(SubagentId),
    /// 기존 엔트리 갱신 (로그 추가/상태 변경). view 가 리렌더.
    Updated(SubagentId),
    /// 서브에이전트 종료 (정상/취소/실패).
    Stopped(SubagentId),
}

impl EventEmitter<SubagentStoreEvent> for StoreInner {}

/// gpui Global 초기화. app 부팅 시 1회 호출.
pub fn init(cx: &mut App) {
    let inner = cx.new(|_| StoreInner {
        entries: HashMap::default(),
        order: Vec::new(),
    });
    cx.set_global(ClaudeSubagentStore { inner });
}

impl ClaudeSubagentStore {
    /// Store 엔티티 핸들을 반환. view 는 이 핸들을 구독해 이벤트를 받는다.
    pub fn entity(&self) -> Entity<StoreInner> {
        self.inner.clone()
    }

    /// Global 접근 헬퍼.
    pub fn get(cx: &App) -> Option<&ClaudeSubagentStore> {
        cx.try_global::<ClaudeSubagentStore>()
    }
}

/// Start 이벤트 수신 시 호출. 기존 id 가 있으면 메타만 갱신(재진입 안전).
pub fn upsert_start(
    cx: &mut App,
    id: SubagentId,
    session_id: Option<String>,
    subagent_type: String,
    description: String,
    prompt: String,
    transcript_path: Option<String>,
    cwd: Option<String>,
    parent_pid: Option<u32>,
) {
    // ClaudeSubagentStore::get 은 &App 차용이므로 entity 를 복제 후 즉시 놓아준다.
    let Some(entity) = ClaudeSubagentStore::get(cx).map(|s| s.entity()) else {
        log::warn!("ClaudeSubagentStore 미초기화 상태에서 upsert_start 호출");
        return;
    };
    entity.update(cx, |inner, cx| {
        if let Some(existing) = inner.entries.get_mut(&id) {
            // 재진입: 메타만 덮어씀. 로그/시간은 유지.
            existing.session_id = session_id.or(existing.session_id.take());
            existing.subagent_type = subagent_type;
            existing.description = description;
            existing.prompt = prompt;
            existing.transcript_path = transcript_path.or(existing.transcript_path.take());
            existing.cwd = cwd.or(existing.cwd.take());
            existing.parent_pid = parent_pid.or(existing.parent_pid.take());
            existing.status = SubagentStatus::Running;
            cx.emit(SubagentStoreEvent::Updated(id));
        } else {
            let state = SubagentState {
                id: id.clone(),
                session_id,
                subagent_type,
                description,
                prompt,
                transcript_path,
                cwd,
                parent_pid,
                status: SubagentStatus::Running,
                started_at: SystemTime::now(),
                finished_at: None,
                result: None,
                log: Vec::new(),
                tail_spawned: false,
            };
            inner.entries.insert(id.clone(), state);
            inner.order.push(id.clone());
            cx.emit(SubagentStoreEvent::Started(id));
        }
    });
}

/// Stop 이벤트 수신 시 호출. 결과를 기록하고 상태를 Completed 로 표시.
/// 엔트리가 없으면(시작 이벤트 유실) 조용히 무시.
pub fn mark_stopped(cx: &mut App, id: SubagentId, result: Option<String>) {
    let Some(entity) = ClaudeSubagentStore::get(cx).map(|s| s.entity()) else {
        return;
    };
    entity.update(cx, |inner, cx| {
        if let Some(state) = inner.entries.get_mut(&id) {
            state.status = SubagentStatus::Completed;
            state.finished_at = Some(SystemTime::now());
            if let Some(r) = result {
                state.result = Some(r);
            }
            cx.emit(SubagentStoreEvent::Stopped(id));
        }
    });
}

/// 로그 엔트리 append. transcript tail 리더가 호출.
/// 엔트리가 `MAX_LOG_ENTRIES` 를 넘으면 가장 오래된 엔트리부터 drop 해 메모리 상한을 유지.
pub fn append_log(cx: &mut App, id: SubagentId, entry: SubagentLogEntry) {
    let Some(entity) = ClaudeSubagentStore::get(cx).map(|s| s.entity()) else {
        return;
    };
    entity.update(cx, |inner, cx| {
        if let Some(state) = inner.entries.get_mut(&id) {
            state.log.push(entry);
            let len = state.log.len();
            if len > MAX_LOG_ENTRIES {
                // drain 으로 앞쪽을 한 번에 제거 — 장시간 실행 시 Vec 재할당 최소화.
                state.log.drain(..(len - MAX_LOG_ENTRIES));
            }
            cx.emit(SubagentStoreEvent::Updated(id));
        }
    });
}

/// 특정 id 의 상태 스냅샷. view 렌더용.
pub fn snapshot(cx: &App, id: &str) -> Option<SubagentState> {
    let entity = ClaudeSubagentStore::get(cx)?.entity();
    let inner = entity.read(cx);
    inner.entries.get(id).cloned()
}

/// 특정 id 의 서브에이전트 존재 여부만 확인. tail loop 의 spawn 중복 가드용.
pub fn contains(cx: &App, id: &str) -> bool {
    let Some(store) = ClaudeSubagentStore::get(cx) else {
        return false;
    };
    store.inner.read(cx).entries.contains_key(id)
}

/// 특정 id 의 상태만 조회. tail loop 이 매 tick `snapshot()` 으로
/// 전체 `SubagentState`(Vec<LogEntry> 포함) 를 clone 하던 비용을 제거.
pub fn status_only(cx: &App, id: &str) -> Option<SubagentStatus> {
    let store = ClaudeSubagentStore::get(cx)?;
    store.inner.read(cx).entries.get(id).map(|s| s.status)
}

/// transcript tail task 의 spawn 표지를 atomic 하게 검사·설정한다.
/// 거의 동시에 두 번 들어온 Start IPC 가 모두 spawn 분기를 통과하던 race 를 차단.
/// 반환값:
/// - `true`  — 호출자가 spawn 권한을 획득. 즉시 tail 을 시작해야 한다.
/// - `false` — 엔트리가 없거나 이미 다른 호출자가 spawn 권한을 가져갔다. spawn 금지.
pub fn try_mark_tail_spawned(cx: &mut App, id: &str) -> bool {
    let Some(entity) = ClaudeSubagentStore::get(cx).map(|s| s.entity()) else {
        return false;
    };
    entity.update(cx, |inner, _cx| {
        let Some(state) = inner.entries.get_mut(id) else {
            return false;
        };
        if state.tail_spawned {
            return false;
        }
        state.tail_spawned = true;
        true
    })
}

