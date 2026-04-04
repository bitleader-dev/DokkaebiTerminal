// 프롬프트 저장소
// 프롬프트 항목의 데이터 모델과 JSON 파일 기반 영속화를 담당한다.

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// 프롬프트 항목 — 터미널에 입력될 프롬프트 텍스트와 메타정보
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PromptEntry {
    /// 고유 ID
    pub id: String,
    /// 터미널에 입력될 프롬프트 텍스트
    pub prompt: String,
    /// 목록에 표시될 설명글
    pub description: String,
    /// 카테고리 (예: "git", "docker", "일반")
    pub category: String,
}

impl PromptEntry {
    /// 새 프롬프트 항목 생성
    pub fn new(prompt: String, description: String, category: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            prompt,
            description,
            category,
        }
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
pub fn load_prompts() -> PromptCollection {
    let path = prompts_file_path();
    if !path.exists() {
        return PromptCollection::default();
    }

    std::fs::read_to_string(&path)
        .context("프롬프트 파일 읽기 실패")
        .and_then(|content| {
            serde_json::from_str::<PromptCollection>(&content)
                .context("프롬프트 JSON 파싱 실패")
        })
        .unwrap_or_default()
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
    category: String,
) -> bool {
    if let Some(entry) = collection.prompts.iter_mut().find(|e| e.id == id) {
        entry.prompt = prompt;
        entry.description = description;
        entry.category = category;
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
