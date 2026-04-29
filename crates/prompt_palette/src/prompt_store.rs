// 프롬프트 저장소
// 프롬프트 항목의 데이터 모델과 JSON 파일 기반 영속화를 담당한다.

use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use gpui::App;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// 프롬프트 placeholder 의 단일 인자 정의
/// 프롬프트 텍스트 안의 `{{name}}` 자리표시자를 호출 시 어떤 라벨/기본값으로 입력받을지 표현
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PromptArgument {
    /// 변수명 — 프롬프트 텍스트의 `{{name}}` 안 name 과 일치
    pub name: String,
    /// 입력 모달에 표시될 라벨/설명
    pub description: String,
    /// 기본값 (선택)
    pub default: Option<String>,
}

/// 프롬프트 항목 — 터미널에 입력될 프롬프트 텍스트와 메타정보
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PromptEntry {
    /// 고유 ID
    pub id: String,
    /// 터미널에 입력될 프롬프트 텍스트 (`{{name}}` placeholder 포함 가능)
    pub prompt: String,
    /// 목록에 표시될 설명글
    pub description: String,
    /// 태그 목록 — 다중 분류 (예: `["git", "branch"]`)
    /// 검색 시 prompt + description 과 함께 매칭 후보로 사용
    #[serde(default)]
    pub tags: Vec<String>,
    /// (deprecated) 단일 카테고리 — 구 스키마 후방 호환 전용. load 시 `tags` 로 마이그레이션.
    /// 신 스키마에서는 항상 빈 문자열이고 직렬화 시 생략됨.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub category: String,
    /// 프롬프트 placeholder 인자 목록. 비어있으면 placeholder 처리 안 함(후방 호환).
    #[serde(default)]
    pub arguments: Vec<PromptArgument>,
    /// 누적 사용 횟수 (confirm 시점마다 +1). 자동 정렬에 사용.
    #[serde(default)]
    pub use_count: u64,
    /// 마지막 사용 시각 (UTC). 자동 정렬의 1순위 키.
    #[serde(default)]
    pub last_used_at: Option<DateTime<Utc>>,
}

impl PromptEntry {
    /// 새 프롬프트 항목 생성 (arguments 빈 배열, use_count 0, last_used_at None 으로 초기화)
    pub fn new(prompt: String, description: String, tags: Vec<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            prompt,
            description,
            tags,
            category: String::new(),
            arguments: Vec::new(),
            use_count: 0,
            last_used_at: None,
        }
    }

    /// arguments 를 채워서 반환하는 빌더
    pub fn with_arguments(mut self, arguments: Vec<PromptArgument>) -> Self {
        self.arguments = arguments;
        self
    }
}

/// 전체 프롬프트 컬렉션
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PromptCollection {
    pub prompts: Vec<PromptEntry>,
}

/// 프롬프트 JSON 파일 경로 반환
fn prompts_file_path() -> PathBuf {
    paths::config_dir().join("prompts.json")
}

/// JSON 파일에서 프롬프트 컬렉션 로드
/// 구 스키마(`category` 단일 값)를 발견하면 자동으로 `tags` 다중 배열로 마이그레이션 + 디스크 영구화.
/// 마이그레이션 직전에 `prompts.json.bak` 백업을 한 번 생성한다(이미 있으면 스킵).
pub fn load_prompts() -> PromptCollection {
    let path = prompts_file_path();
    if !path.exists() {
        return PromptCollection::default();
    }

    let mut collection: PromptCollection = std::fs::read_to_string(&path)
        .context("프롬프트 파일 읽기 실패")
        .and_then(|content| {
            serde_json::from_str::<PromptCollection>(&content)
                .context("프롬프트 JSON 파싱 실패")
        })
        .unwrap_or_default();

    // 구 스키마 → 신 스키마 마이그레이션
    let needs_save = migrate_collection(&mut collection);

    if needs_save {
        // 마이그레이션 직전 백업 생성 (이미 .bak 가 있으면 스킵 — 한 번만)
        let bak_path = path.with_file_name(format!(
            "{}.bak",
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        ));
        if !bak_path.exists() {
            let _ = std::fs::copy(&path, &bak_path);
        }
        // 신 스키마로 영구화
        let _ = save_prompts(&collection);
    }

    collection
}

/// 프롬프트 컬렉션을 JSON 파일에 저장
pub fn save_prompts(collection: &PromptCollection) -> Result<()> {
    let path = prompts_file_path();

    // 상위 디렉토리가 없으면 생성
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("프롬프트 디렉토리 생성 실패")?;
    }

    let json = serde_json::to_string_pretty(collection).context("프롬프트 직렬화 실패")?;
    std::fs::write(&path, json).context("프롬프트 파일 쓰기 실패")?;
    Ok(())
}

/// 프롬프트 추가
pub fn add_prompt(collection: &mut PromptCollection, entry: PromptEntry) {
    collection.prompts.push(entry);
}

/// 프롬프트 수정 (ID 기준)
pub fn update_prompt(
    collection: &mut PromptCollection,
    id: &str,
    prompt: String,
    description: String,
    tags: Vec<String>,
) -> bool {
    if let Some(entry) = collection.prompts.iter_mut().find(|e| e.id == id) {
        entry.prompt = prompt;
        entry.description = description;
        entry.tags = tags;
        // 구 스키마 잔재가 있으면 정리
        entry.category.clear();
        true
    } else {
        false
    }
}

/// 쉼표 구분 텍스트를 tags 벡터로 파싱
/// 빈 문자열·중복 공백·앞뒤 공백 제거 + 빈 항목 자동 제외
pub fn parse_tags_input(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// 사용 기록을 갱신 — `use_count += 1`, `last_used_at = Utc::now()` 후 비동기 save
/// confirm 직전 호출. save 실패는 사용자 차단 없이 무시(다음 호출에 재시도).
pub fn record_usage(entry_id: &str, cx: &mut App) {
    let mut collection = load_prompts();
    let updated = if let Some(entry) = collection.prompts.iter_mut().find(|e| e.id == entry_id) {
        entry.use_count = entry.use_count.saturating_add(1);
        entry.last_used_at = Some(Utc::now());
        true
    } else {
        false
    };
    if updated {
        let collection_clone = collection.clone();
        cx.background_executor()
            .spawn(async move {
                let _ = save_prompts(&collection_clone);
            })
            .detach();
    }
}

/// 빈 query 상태에서의 자동 정렬 비교 함수
/// 우선순위: last_used_at desc(None 은 가장 뒤) → use_count desc → 등록 순서(원래 인덱스 asc)
/// 반환: a 가 b 보다 앞에 와야 하면 Less
pub fn compare_by_recency(
    a: &PromptEntry,
    a_index: usize,
    b: &PromptEntry,
    b_index: usize,
) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    // 1순위: last_used_at desc — Some 이 None 보다 앞, 그 안에서 더 최근이 앞
    match (a.last_used_at, b.last_used_at) {
        (Some(a_time), Some(b_time)) => match b_time.cmp(&a_time) {
            Ordering::Equal => {}
            ord => return ord,
        },
        (Some(_), None) => return Ordering::Less,
        (None, Some(_)) => return Ordering::Greater,
        (None, None) => {}
    }
    // 2순위: use_count desc
    match b.use_count.cmp(&a.use_count) {
        Ordering::Equal => {}
        ord => return ord,
    }
    // 3순위: 등록 순서 asc (원래 인덱스가 작을수록 먼저)
    a_index.cmp(&b_index)
}

/// 구 스키마(`category` 단일 값) → 신 스키마(`tags` 다중 배열) 메모리 내 마이그레이션
/// 변경이 발생했으면 `true` 반환 (호출자가 디스크에 영구화 결정)
fn migrate_collection(collection: &mut PromptCollection) -> bool {
    let mut needs_save = false;
    for entry in &mut collection.prompts {
        if !entry.category.is_empty() && entry.tags.is_empty() {
            // category → tags 이전
            entry.tags.push(std::mem::take(&mut entry.category));
            needs_save = true;
        } else if !entry.category.is_empty() {
            // tags 가 이미 있는데 category 도 남아있다면 category 만 정리
            entry.category.clear();
            needs_save = true;
        }
    }
    needs_save
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_with(prompt: &str, tags: Vec<String>, category: &str) -> PromptEntry {
        PromptEntry {
            id: Uuid::new_v4().to_string(),
            prompt: prompt.to_string(),
            description: String::new(),
            tags,
            category: category.to_string(),
            arguments: Vec::new(),
            use_count: 0,
            last_used_at: None,
        }
    }

    /// 사용 빈도 정렬 검증용 — use_count + last_used_at 직접 지정
    fn entry_with_usage(prompt: &str, use_count: u64, last_used_at: Option<DateTime<Utc>>) -> PromptEntry {
        PromptEntry {
            id: Uuid::new_v4().to_string(),
            prompt: prompt.to_string(),
            description: String::new(),
            tags: Vec::new(),
            category: String::new(),
            arguments: Vec::new(),
            use_count,
            last_used_at,
        }
    }

    #[test]
    fn parse_tags_basic() {
        assert_eq!(parse_tags_input("git, docker, k8s"), vec!["git", "docker", "k8s"]);
    }

    #[test]
    fn parse_tags_trim_and_skip_empty() {
        assert_eq!(
            parse_tags_input("  a , , b ,c,, "),
            vec!["a", "b", "c"]
        );
    }

    #[test]
    fn parse_tags_empty_input() {
        assert_eq!(parse_tags_input(""), Vec::<String>::new());
    }

    #[test]
    fn parse_tags_single() {
        assert_eq!(parse_tags_input("git"), vec!["git"]);
    }

    #[test]
    fn migrate_category_to_tags_when_tags_empty() {
        let mut c = PromptCollection {
            prompts: vec![entry_with("git status", vec![], "git")],
        };
        let changed = migrate_collection(&mut c);
        assert!(changed);
        assert_eq!(c.prompts[0].tags, vec!["git"]);
        assert_eq!(c.prompts[0].category, "");
    }

    #[test]
    fn migrate_clears_category_when_tags_already_set() {
        // tags 가 이미 있는데 category 도 잔재로 남아있는 경우 → category 정리만
        let mut c = PromptCollection {
            prompts: vec![entry_with(
                "git push",
                vec!["git".to_string()],
                "old-cat",
            )],
        };
        let changed = migrate_collection(&mut c);
        assert!(changed);
        assert_eq!(c.prompts[0].tags, vec!["git"]);
        assert_eq!(c.prompts[0].category, "");
    }

    #[test]
    fn migrate_no_op_when_already_migrated() {
        // category 이미 비어있고 tags 가 있으면 변경 없음
        let mut c = PromptCollection {
            prompts: vec![entry_with("ls -al", vec!["unix".to_string()], "")],
        };
        let changed = migrate_collection(&mut c);
        assert!(!changed);
        assert_eq!(c.prompts[0].tags, vec!["unix"]);
    }

    #[test]
    fn migrate_no_op_when_both_empty() {
        let mut c = PromptCollection {
            prompts: vec![entry_with("echo hi", vec![], "")],
        };
        let changed = migrate_collection(&mut c);
        assert!(!changed);
    }

    // ─── 사용 빈도 정렬 (compare_by_recency) ───────────────────────

    #[test]
    fn sort_recency_some_before_none() {
        let now = Utc::now();
        let a = entry_with_usage("with-time", 0, Some(now));
        let b = entry_with_usage("no-time", 0, None);
        assert_eq!(
            compare_by_recency(&a, 0, &b, 1),
            std::cmp::Ordering::Less,
            "last_used_at Some 항목은 None 항목보다 앞에 와야 함"
        );
    }

    #[test]
    fn sort_recency_more_recent_first() {
        let earlier = Utc::now();
        let later = earlier + chrono::Duration::seconds(60);
        let a = entry_with_usage("earlier", 0, Some(earlier));
        let b = entry_with_usage("later", 0, Some(later));
        // b 가 더 최근 → b 가 앞
        assert_eq!(
            compare_by_recency(&a, 0, &b, 1),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn sort_recency_use_count_breaks_tie_when_no_time() {
        let a = entry_with_usage("less", 1, None);
        let b = entry_with_usage("more", 5, None);
        // last_used_at 둘 다 None 동률 → use_count desc → b 가 앞
        assert_eq!(
            compare_by_recency(&a, 0, &b, 1),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn sort_recency_index_breaks_tie_when_all_equal() {
        let a = entry_with_usage("first", 0, None);
        let b = entry_with_usage("second", 0, None);
        // 모든 정렬 키 동률 → 등록 순서(인덱스) asc → a 가 앞
        assert_eq!(compare_by_recency(&a, 0, &b, 1), std::cmp::Ordering::Less);
    }

    #[test]
    fn sort_recency_full_priority_chain() {
        // 같은 last_used_at 이면 use_count 비교, use_count 도 같으면 인덱스
        let now = Utc::now();
        let a = entry_with_usage("a", 1, Some(now));
        let b = entry_with_usage("b", 3, Some(now));
        // 시간 동률 → use_count desc → b 가 앞
        assert_eq!(
            compare_by_recency(&a, 0, &b, 1),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn migrate_multiple_entries() {
        let mut c = PromptCollection {
            prompts: vec![
                entry_with("a", vec![], "x"),
                entry_with("b", vec!["y".to_string()], ""),
                entry_with("c", vec![], ""),
            ],
        };
        let changed = migrate_collection(&mut c);
        assert!(changed);
        assert_eq!(c.prompts[0].tags, vec!["x"]);
        assert_eq!(c.prompts[1].tags, vec!["y"]);
        assert_eq!(c.prompts[2].tags, Vec::<String>::new());
    }
}

/// 프롬프트 arguments 갱신 (ID 기준)
pub fn update_arguments(
    collection: &mut PromptCollection,
    id: &str,
    arguments: Vec<PromptArgument>,
) -> bool {
    if let Some(entry) = collection.prompts.iter_mut().find(|e| e.id == id) {
        entry.arguments = arguments;
        true
    } else {
        false
    }
}

/// 프롬프트 삭제 (ID 기준)
pub fn remove_prompt(collection: &mut PromptCollection, id: &str) -> bool {
    let before = collection.prompts.len();
    collection.prompts.retain(|e| e.id != id);
    collection.prompts.len() < before
}

/// 고유 카테고리 목록 반환 (정렬됨)
pub fn categories(collection: &PromptCollection) -> Vec<String> {
    let mut cats: Vec<String> = collection
        .prompts
        .iter()
        .map(|e| e.category.clone())
        .filter(|c| !c.is_empty())
        .collect();
    cats.sort();
    cats.dedup();
    cats
}
