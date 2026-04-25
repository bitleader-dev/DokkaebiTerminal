# Zed v0.233.10 Dokkaebi 백포트 — 계획

> **작성일**: 2026-04-25
> **상류 기준**: `zed-industries/zed` v0.233.10 (2026-04-24 publish, stable hotfix)
> **Dokkaebi 기준선**: v0.4.0 (최근 백포트 v0.233.6 까지 적용 완료)

---

## 1. 목표·범위
- Zed v0.233.10 의 단일 PR(#54829) Dokkaebi 백포트.
- 변경 성격: OpenAI 모델 enum 확장 (`gpt-5.5` / `gpt-5.5-pro` 추가).
- Dokkaebi 재구성(REPL/Collab/Dev Container/cloud)·Windows-only 정책 영향 없음.

## 2. 적용 PR

### #54829 — OpenAI gpt-5.5 / gpt-5.5-pro 지원 추가
- merge SHA: `63e070ea695c56e7500081680a32b3cafb1a48df`
- 상류 변경 파일 2개, +23/-2
- Dokkaebi 적용 대상 파일 2개 (모두 존재 확인)
  - `crates/open_ai/src/open_ai.rs`
  - `crates/language_models/src/provider/open_ai.rs`

#### 상류 patch 적용 위치 (10곳)
1. `crates/open_ai/src/open_ai.rs:97~` `Model` enum 에 `FivePointFive`, `FivePointFivePro` 추가 (`#[serde(rename = ...)]`)
2. 동 파일 `from_id` 매치에 두 키 추가
3. 동 파일 `id` 매치에 두 키 추가
4. 동 파일 `display_name` 매치에 두 키 추가
5. 동 파일 `max_token_count` 매치에 두 키 추가 (1_050_000)
6. 동 파일 `max_output_tokens` 매치에 두 키 추가 (Some(128_000))
7. 동 파일 `reasoning_effort` 매치 — `FivePointThreeCodex | FivePointFourPro` → `+ FivePointFivePro` 합병 (Medium)
8. 동 파일 `supports_parallel_tool_calls` 매치 — `FivePointFivePro` 만 false 분기에 추가 (5.5는 wildcard true)
9. 동 파일 `supports_prompt_cache_key` 매치에 5.5/5.5-pro 두 키 추가 (true 분기)
10. `crates/language_models/src/provider/open_ai.rs:312~` `LanguageModel` impl 의 supports_* 매치에 두 키 추가

#### Dokkaebi 독자 보정 (1곳, 상류 patch에 없지만 필수)
- `crates/language_models/src/provider/open_ai.rs:1246-1252` token counting 분기. 상류 v0.233.10 head 동일 파일은 599줄로 이 분기를 다른 형태로 재구성했으나, Dokkaebi는 독자 유지. enum variant 추가 시 non-exhaustive match 에러 회피를 위해:
  - 주석에 `5.5, 5.5-pro` 추가
  - `FivePointFour | FivePointFourPro` 분기에 `FivePointFive | FivePointFivePro` 합쳐 동일하게 `"gpt-5"` 토크나이저 fallback

## 3. 자동 제외
- 없음 (단일 PR 적용)

## 4. 작업 단계
- [x] 1. 범위 확인 — 승인 받음 (2026-04-25)
- [x] 2. patch 적용 (위 11개 변경 지점)
- [x] 3. `cargo check -p open_ai` 통과 확인 (외부 PowerShell — 사용자 검증)
- [x] 4. `cargo check -p language_models` 통과 확인 (외부 PowerShell — 사용자 검증)
- [x] 5. `cargo check -p Dokkaebi --tests` 신규 경고/에러 0건 확인 (외부 PowerShell — 사용자 검증)
- [x] 6. 문서 갱신
  - `notes.md` 에 2026-04-25 Phase 26 항목 추가
  - `assets/release_notes.md` v0.4.0 헤더 날짜 갱신(04-24 → 04-25) + `### 새로운 기능` 에 "OpenAI GPT-5.5 / GPT-5.5 Pro 모델 지원" 1 항목 추가
- [x] 7. 완료 보고

## 5. 검증 방법
- `cargo check -p open_ai`
- `cargo check -p language_models`
- `cargo check -p Dokkaebi --tests`
- 모두 통과 + 신규 경고 0건이어야 완료.

## 6. 승인 필요 항목
- 없음 (단일 PR 백포트, 사용자 사전 승인 완료)
