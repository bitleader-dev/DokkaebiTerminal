//! Claude Code transcript JSONL 파일을 tail 방식으로 읽어 서브에이전트의
//! 진행 상황(도구 호출/결과 요약)을 `ClaudeSubagentStore` 에 기록하는 모듈.
//!
//! 흐름:
//! 1. `handle_notify_request` 가 SubagentStart IPC 를 처리한 뒤 여기에 있는
//!    `spawn_transcript_tail()` 을 호출한다.
//! 2. 백그라운드 task 가 transcript 파일을 열어 마지막 offset 부터 polling (~200ms) 으로
//!    append 된 라인을 읽는다. 서브에이전트가 `mark_stopped` 된 이후 짧은 grace
//!    period 를 두고 종료한다.
//! 3. 각 라인은 JSON 으로 파싱해 아래 타입을 분류한다:
//!    - user message 의 content: tool_result (parent_tool_use_id / tool_use_id 매칭)
//!    - assistant message 의 content: tool_use / text
//!    발견한 이벤트는 `append_log` 로 store 에 누적.
//!
//! Claude Code transcript 스펙은 공식 문서에 완전히 공개되지 않았으므로 필드
//! 부재 시 silently skip 한다. 파싱 실패 라인도 무시.
//!
//! 이 모듈은 `crates/zed` 내부에 두어(옵션 A 확정) 추가 크레이트 없이 동작한다.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use claude_subagent_view::{SubagentLogEntry, SubagentStatus, append_log, status_only};
use gpui::AsyncApp;
use serde_json::Value;
use smol::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, SeekFrom},
};
use std::time::SystemTime;
use util::truncate_and_trailoff;

/// transcript tail polling 주기. Claude Code 가 flush 하기까지의 지연(경험상 ~300ms)
/// 을 고려해 200ms 로 설정. 너무 짧으면 I/O 낭비, 너무 길면 UX 지연.
const POLL_INTERVAL: Duration = Duration::from_millis(200);
/// 서브에이전트 완료 후 transcript tail 을 유지할 시간. 마지막 결과 라인이
/// flush 되기 전에 task 가 종료되지 않도록 여유를 둔다.
const POST_STOP_GRACE: Duration = Duration::from_secs(2);
/// 한 번의 polling 에서 읽을 최대 바이트. 무한 read loop 을 방지.
const MAX_READ_PER_TICK: u64 = 1024 * 1024; // 1 MiB
/// 한 로그 엔트리 detail 의 최대 문자 길이. 초과 시 말줄임표로 자른다.
const LOG_ENTRY_MAX_CHARS: usize = 5000;

/// 백그라운드 task 를 spawn 해 지정 transcript 파일을 tail 한다.
/// `subagent_id` 가 store 에서 사라지거나 상태가 Stopped 로 전환되면
/// grace 기간 후 자동 종료.
pub fn spawn_transcript_tail(
    subagent_id: String,
    transcript_path: PathBuf,
    cx: &mut AsyncApp,
) {
    cx.spawn(async move |cx| {
        if let Err(err) = tail_loop(&subagent_id, &transcript_path, cx).await {
            log::debug!(
                "claude subagent transcript tail 종료: id={} err={}",
                subagent_id,
                err
            );
        }
    })
    .detach();
}

async fn tail_loop(
    subagent_id: &str,
    transcript_path: &Path,
    cx: &mut AsyncApp,
) -> anyhow::Result<()> {
    // 파일 핸들과 offset·최신 size 를 루프 전체에서 재사용.
    // 파일 생성이 늦어질 수 있으므로 최초 오픈은 루프 내부에서 지연 재시도.
    let mut file: Option<File> = None;
    let mut offset: u64 = 0;
    let mut last_size: u64 = 0;
    let mut stopped_since: Option<SystemTime> = None;

    loop {
        // store 에서 상태 확인 — 엔트리가 사라지면 즉시 종료.
        // status_only 는 전체 SubagentState 를 clone 하지 않아 tick 당 비용이 낮다.
        let Some(status) = cx.update(|cx| status_only(cx, subagent_id)) else {
            return Ok(());
        };

        // Stopped 상태라면 grace 시작. grace 만료되면 종료.
        if matches!(
            status,
            SubagentStatus::Completed | SubagentStatus::Cancelled | SubagentStatus::Failed
        ) {
            let now = SystemTime::now();
            let since = stopped_since.get_or_insert(now);
            if now.duration_since(*since).unwrap_or_default() >= POST_STOP_GRACE {
                // 마지막 한 번 더 읽고 종료.
                let _ = read_new_lines(
                    &mut file,
                    transcript_path,
                    &mut offset,
                    &mut last_size,
                    subagent_id,
                    cx,
                )
                .await;
                return Ok(());
            }
        }

        // 신규 라인 읽기.
        if let Err(e) = read_new_lines(
            &mut file,
            transcript_path,
            &mut offset,
            &mut last_size,
            subagent_id,
            cx,
        )
        .await
        {
            log::debug!(
                "transcript tail read 실패(재시도): id={} path={:?} err={}",
                subagent_id,
                transcript_path,
                e
            );
            // 에러 시 파일 핸들 invalidate — 다음 tick 에 다시 열도록 유도.
            file = None;
        }

        cx.background_executor().timer(POLL_INTERVAL).await;
    }
}

/// 파일 핸들을 lazy 하게 유지하며, 파일 길이가 `offset` 을 넘어설 때만 읽는다.
/// 파일이 없거나 축소된 경우 핸들을 리셋해 다음 tick 에 재-open 한다.
async fn read_new_lines(
    file_slot: &mut Option<File>,
    transcript_path: &Path,
    offset: &mut u64,
    last_size: &mut u64,
    subagent_id: &str,
    cx: &mut AsyncApp,
) -> anyhow::Result<()> {
    if file_slot.is_none() {
        match File::open(transcript_path).await {
            Ok(f) => *file_slot = Some(f),
            // 파일이 아직 없으면 다음 tick 에 재시도.
            Err(_) => return Ok(()),
        }
    }
    let file = file_slot
        .as_mut()
        .expect("file slot 은 위 분기에서 보장됨");

    // 매 tick metadata 만 확인 — 파일 길이가 변하지 않았으면 seek/read 전부 생략.
    let metadata = file.metadata().await?;
    let size = metadata.len();
    if size < *offset {
        // truncate/rotate 감지 — 핸들 버리고 offset 초기화 후 다음 tick 에 재-open.
        *file_slot = None;
        *offset = 0;
        *last_size = 0;
        return Ok(());
    }
    if size == *last_size {
        // 길이 변화 없음: read 자체를 스킵(가장 흔한 tick).
        return Ok(());
    }
    if size == *offset {
        *last_size = size;
        return Ok(());
    }

    let to_read = std::cmp::min(size - *offset, MAX_READ_PER_TICK);
    file.seek(SeekFrom::Start(*offset)).await?;
    let mut buf = vec![0u8; to_read as usize];
    let read = file.read(&mut buf).await?;
    buf.truncate(read);
    *offset += read as u64;
    *last_size = size;

    // buf 를 개행으로 분리. 마지막 라인이 미완성이면 offset 을 되돌려 다음 tick 에 재시도.
    // `rposition` 으로 끝에서부터 마지막 개행을 한 번에 찾아 이중 스캔을 피한다.
    let Some(last_newline_rel) = buf.iter().rposition(|b| *b == b'\n') else {
        // 줄바꿈이 한 번도 안 나왔으면 이번 read 만큼 rollback.
        *offset -= read as u64;
        return Ok(());
    };
    let process_until = last_newline_rel + 1;
    // 미완성 tail 부분은 rollback.
    let rollback = read - process_until;
    if rollback > 0 {
        *offset -= rollback as u64;
    }

    let chunk = &buf[..process_until];
    for line in chunk.split(|b| *b == b'\n') {
        if line.is_empty() {
            continue;
        }
        // parent_tool_use_id 빠른 필터 — 공유 transcript 에는 부모 에이전트와
        // 다른 서브에이전트 라인이 뒤섞여 있으므로 JSON 파싱 전에 자기 id 가
        // 포함된 라인만 남긴다. bytes::windows 기반 substring 검사는 미려하진
        // 않지만 파싱·allocation 비용을 피해 N 병렬 서브에이전트 뷰의 hot path
        // 를 눈에 띄게 줄인다. (정확 매칭은 parse 이후 한 번 더 확인)
        if !line_belongs_to_subagent(line, subagent_id) {
            continue;
        }
        if let Ok(value) = serde_json::from_slice::<Value>(line) {
            if !matches_parent_tool_use_id(&value, subagent_id) {
                continue;
            }
            if let Some(entry) = classify_entry(&value) {
                let id = subagent_id.to_owned();
                cx.update(|cx| {
                    append_log(cx, id, entry);
                });
            }
        }
    }

    Ok(())
}

/// transcript JSONL 한 줄을 분류해 로그 엔트리로 변환.
/// 현재 분류 대상:
/// - assistant message 의 tool_use 블록: label=tool name, detail=input 첫 5000자
/// - user message 의 tool_result 블록: label="tool_result", detail=결과 첫 5000자
/// - assistant message 의 text 블록: label="assistant", detail=본문 첫 5000자
///
/// 서브에이전트 내부에서 기록되는 라인만 매칭하기 위해 호출 전 단계에서
/// `matches_parent_tool_use_id` 로 parent_tool_use_id == subagent_id 필터가 선행된다.
fn classify_entry(value: &Value) -> Option<SubagentLogEntry> {
    let ty = value.get("type")?.as_str()?;
    let msg = value.get("message")?;
    let content = msg.get("content")?;
    let arr = content.as_array()?;
    // 한 라인당 여러 block 이 있을 수 있으나 첫 의미 있는 block 1개만 로그화.
    for block in arr {
        let block_ty = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match (ty, block_ty) {
            ("assistant", "tool_use") => {
                let name = block
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("tool_use")
                    .to_owned();
                // tool_use 의 input 은 객체이므로 key: value 쌍을 줄바꿈으로 이어
                // 플레인 텍스트로 포맷한다. `v.to_string()` 은 JSON 재직렬화라
                // 값 안의 실제 줄바꿈이 `\n` 문자열로 이스케이프돼 사용자에게 그대로 노출됨.
                let input_str = block
                    .get("input")
                    .map(format_value_plain)
                    .unwrap_or_default();
                let detail = truncate_and_trailoff(&input_str, LOG_ENTRY_MAX_CHARS);
                return Some(SubagentLogEntry {
                    timestamp: SystemTime::now(),
                    label: name,
                    detail,
                });
            }
            ("user", "tool_result") => {
                let content_val = block.get("content");
                let detail_raw = match content_val {
                    Some(Value::String(s)) => s.clone(),
                    Some(Value::Array(parts)) => parts
                        .iter()
                        .filter_map(|p| p.get("text").and_then(|v| v.as_str()))
                        .collect::<Vec<_>>()
                        .join(" "),
                    // 복잡한 content(객체 등)는 JSON 재직렬화 대신 플레인 포맷으로.
                    Some(other) => format_value_plain(other),
                    None => String::new(),
                };
                let detail = truncate_and_trailoff(&detail_raw, LOG_ENTRY_MAX_CHARS);
                return Some(SubagentLogEntry {
                    timestamp: SystemTime::now(),
                    label: "tool_result".into(),
                    detail,
                });
            }
            ("assistant", "text") => {
                let text = block
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                if text.trim().is_empty() {
                    continue;
                }
                return Some(SubagentLogEntry {
                    timestamp: SystemTime::now(),
                    label: "assistant".into(),
                    detail: truncate_and_trailoff(&text, LOG_ENTRY_MAX_CHARS),
                });
            }
            _ => continue,
        }
    }
    None
}

/// 파싱 전 1차 필터 — 라인 바이트에 subagent_id 가 포함돼 있는지만 검사.
/// transcript 에는 main agent 와 다른 서브에이전트 라인이 섞여 있어, 자기 id 가
/// 없는 라인을 파싱 단계에서 버려 N 서브에이전트 × 전체 라인 수의 불필요 파싱을 제거한다.
fn line_belongs_to_subagent(line: &[u8], subagent_id: &str) -> bool {
    let needle = subagent_id.as_bytes();
    if needle.is_empty() || line.len() < needle.len() {
        return false;
    }
    line.windows(needle.len()).any(|w| w == needle)
}

/// 파싱 후 2차 필터 — JSON 상의 parent_tool_use_id 가 정확히 subagent_id 와 일치하는지.
/// 부모가 없는 라인(최상위 agent 라인)은 skip.
fn matches_parent_tool_use_id(value: &Value, subagent_id: &str) -> bool {
    value
        .get("parent_tool_use_id")
        .and_then(|v| v.as_str())
        .is_some_and(|s| s == subagent_id)
}

/// `Value` 를 JSON 재직렬화 없이 플레인 텍스트로 포맷한다.
/// 문자열 값은 원본 그대로(줄바꿈 유지) 내보내고, 객체는 `key: value` 쌍을 줄바꿈으로,
/// 배열은 쉼표로 이어 붙인다. `v.to_string()` 을 그대로 쓰면 `\n` 이 `"\\n"` 으로
/// 이스케이프돼 사용자에게 백슬래시가 보이는 문제를 피한다.
fn format_value_plain(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(arr) => arr
            .iter()
            .map(format_value_plain)
            .collect::<Vec<_>>()
            .join(", "),
        Value::Object(obj) => obj
            .iter()
            .map(|(k, v)| format!("{}: {}", k, format_value_plain(v)))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}
