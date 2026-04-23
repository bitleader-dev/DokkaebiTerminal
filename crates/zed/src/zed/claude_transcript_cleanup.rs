//! Claude Code 세션 transcript 자동 정리 모듈.
//!
//! Dokkaebi 시작 시 1회 호출. 설정 `claude_code.transcript_cleanup_enabled` 가
//! true 면 `~/.claude/projects/` 이하의 `.jsonl` 파일 중 mtime 이
//! `transcript_retention_days` 를 지난 것을 백그라운드에서 조용히 삭제한다.
//!
//! 설계 원칙:
//! - 비동기/비차단: gpui background executor 로 spawn, UI 스레드 영향 없음
//! - 실패 허용: 파일 삭제 실패·메타데이터 조회 실패 등은 로그만 남기고 계속
//! - mtime 기반: 현재 사용 중인 세션 파일(최근 mtime)은 자연히 제외됨
//! - 사용자 UX 간섭 없음: 다이얼로그·토스트 없이 조용히 처리

use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use claude_subagent_view::claude_code_settings;
use gpui::App;
use util::ResultExt;
use walkdir::WalkDir;

const MIN_RETENTION_DAYS: u32 = 7;
const MAX_RETENTION_DAYS: u32 = 365;
const DEFAULT_RETENTION_DAYS: u32 = 60;
/// 두 번의 정리 사이 최소 간격. 재부팅·단축 세션 재시작으로 시작 시점에
/// 중복 walk 가 반복되지 않도록 sentinel 파일 mtime 으로 가드한다.
const CLEANUP_COOLDOWN: Duration = Duration::from_secs(24 * 60 * 60);
/// 마지막 정리 시각을 mtime 으로 보관하는 sentinel 파일 이름.
/// `~/.claude/` 루트에 생성(`.claude/projects` 가 없거나 비어 있어도 동작).
const CLEANUP_STAMP_FILENAME: &str = ".dokkaebi_cleanup_stamp";

/// 설정을 읽어 정리가 활성화돼 있으면 백그라운드 task 를 spawn 한다.
/// 호출 시점: Dokkaebi 초기화(workspace::init 직후 권장).
pub fn run_cleanup_if_enabled(cx: &mut App) {
    let (enabled, retention_days) = claude_code_settings(cx)
        .map(|c| {
            (
                c.transcript_cleanup_enabled.unwrap_or(false),
                c.transcript_retention_days
                    .unwrap_or(DEFAULT_RETENTION_DAYS)
                    .clamp(MIN_RETENTION_DAYS, MAX_RETENTION_DAYS),
            )
        })
        .unwrap_or((false, DEFAULT_RETENTION_DAYS));

    if !enabled {
        log::debug!("[transcript-cleanup] disabled by settings");
        return;
    }

    let Some(root) = claude_plugin_registry::projects_root() else {
        log::debug!("[transcript-cleanup] ~/.claude/projects 경로 해석 실패 — skip");
        return;
    };
    let stamp_path = cleanup_stamp_path();

    // 24h 내 재실행 가드 — sentinel mtime 이 cooldown 내면 skip. 재부팅·단축 세션
    // 반복으로 시작 시마다 전체 재귀 walk 가 돌지 않도록 저렴하게 차단한다.
    if let Some(path) = &stamp_path
        && let Ok(metadata) = std::fs::metadata(path)
        && let Ok(mtime) = metadata.modified()
        && SystemTime::now()
            .duration_since(mtime)
            .is_ok_and(|age| age < CLEANUP_COOLDOWN)
    {
        log::debug!("[transcript-cleanup] within cooldown — skip");
        return;
    }

    cx.background_executor()
        .spawn(async move {
            let deleted = cleanup_transcripts(&root, retention_days as u64);
            log::info!(
                "[transcript-cleanup] root={:?} retention_days={} deleted={}",
                root,
                retention_days,
                deleted
            );
            if let Some(path) = stamp_path {
                touch_stamp(&path);
            }
        })
        .detach();
}

/// sentinel 파일 경로. `~/.claude/.dokkaebi_cleanup_stamp`.
fn cleanup_stamp_path() -> Option<PathBuf> {
    claude_plugin_registry::settings_path()
        .and_then(|p| p.parent().map(|parent| parent.join(CLEANUP_STAMP_FILENAME)))
}

/// sentinel 파일을 touch — 존재하면 mtime 갱신, 없으면 빈 파일 생성.
fn touch_stamp(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // create 로 mtime 을 현재로 갱신. 실패하면 다음 기회에 재시도되는 것이므로 무시.
    let _ = std::fs::File::create(path);
}

/// `root` 이하 모든 `.jsonl` 파일을 재귀 순회해 mtime 이 `retention_days` 이상 지난
/// 파일을 삭제한다. 반환값은 삭제된 파일 개수.
/// `root` 부재 시 WalkDir 이 조용히 빈 iterator 를 내보내 자연스럽게 0 이 반환된다.
fn cleanup_transcripts(root: &Path, retention_days: u64) -> usize {
    let threshold = match SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(retention_days * 86_400))
    {
        Some(t) => t,
        None => return 0,
    };
    let mut deleted = 0usize;
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        // walkdir::Metadata 는 std::io 계열을 재노출하므로 modified() 는 io::Error.
        // 앞 entry.metadata() 는 walkdir::Error 라 타입이 달라 체인을 끊어 각각 처리.
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let Ok(mtime) = metadata.modified() else {
            continue;
        };
        if mtime > threshold {
            continue;
        }
        if std::fs::remove_file(path).log_err().is_some() {
            deleted += 1;
        }
    }
    deleted
}
