# Zed v0.232.2 PR #53008 LanguageAwareStyling 백포트 (2026-04-16, 최종)

## 목표
`"semantic_tokens": "full"` 설정 시 에디터에서 **진단(오류/경고) 밑줄이 사라지는 버그** 수정. `language_aware: bool` 파라미터를 `LanguageAwareStyling { tree_sitter: bool, diagnostics: bool }` 구조체로 확장해 두 개념을 개별 제어.

## 사전 조사 결과

### Dokkaebi 현재 상태
- `editor/src/element.rs:3656` 이미 `use_tree_sitter` 분리해서 `highlighted_chunks`에 전달. 하지만 `highlighted_chunks` 시그니처는 `language_aware: bool` 그대로 → `buffer.rs::chunks`에서 `diagnostics = language_aware`로 결합되어 진단도 함께 꺼짐
- `display_map.rs:1670` `use_tree_sitter_for_syntax()` 메서드 존재 — 호출처 이미 준비됨
- `buffer.rs:3736-3744` `language_aware`와 `diagnostics` 결합이 그대로 남아있음 (PR #53008이 해결하는 핵심 문제)

### 상류 patch 분석 완료 (18파일, +468/-97)
- **core**: `language/src/buffer.rs` — `LanguageAwareStyling` 구조체 정의 + `chunks` 시그니처 변경
- **체인 (6파일)**: `multi_buffer`, `display_map`, `block_map`, `custom_highlights`, `fold_map`, `inlay_map`, `tab_map`, `wrap_map`
- **호출처 (5파일)**: `editor.rs`, `element.rs`, `outline_panel.rs`, `lsp_store.rs`, `vim/state.rs`
- **테스트 (3파일)**: `buffer_tests.rs`, `multi_buffer_tests.rs`, `project_tests.rs` — 본체 시그니처 변경에 따라 호출 migrate 필요
- **별도**: `semantic_tokens.rs` +130줄 = 신규 테스트 1건만 → **이식 생략**

### 이식 패턴 (기계적 치환)
- `language_aware: bool` → `language_aware: LanguageAwareStyling`
- `false`(비-language-aware) 호출 → `LanguageAwareStyling { tree_sitter: false, diagnostics: false }`
- `true`(언어 인식 + 진단) 호출 → `LanguageAwareStyling { tree_sitter: true, diagnostics: true }`
- `element.rs`는 `use_tree_sitter`를 `LanguageAwareStyling { tree_sitter: use_tree_sitter, diagnostics: true }`로 변환 (**이 부분이 버그 수정의 핵심**)

## 범위 (수정 대상 17개 파일 + 테스트 3개)

### Core
1. `crates/language/src/buffer.rs` — `LanguageAwareStyling` 구조체 pub struct 정의 + `chunks` 시그니처 변경 + 내부 2개 호출(`words_in_range`, `outline_item_text`) migrate
2. `crates/language/src/buffer_tests.rs` — `test_random_chunk_bitmaps` 1곳 호출 migrate

### Multi buffer
3. `crates/multi_buffer/src/multi_buffer.rs` — `MultiBufferChunks.language_aware` 필드 타입 + `MultiBufferSnapshot::chunks` 시그니처 + 내부 2개 호출 (text, text_for_range)
4. `crates/multi_buffer/src/multi_buffer_tests.rs` — 2곳 호출 migrate

### Display map chain
5. `crates/editor/src/display_map.rs` — `chunks`/`highlighted_chunks` 시그니처 + 내부 4개 호출
6. `crates/editor/src/display_map/block_map.rs` — `chunks` 시그니처 + 내부 호출 + 테스트 1
7. `crates/editor/src/display_map/custom_highlights.rs` — `new` 시그니처 + 테스트 1
8. `crates/editor/src/display_map/fold_map.rs` — `chunks` 시그니처 + 내부 3개 호출 + 테스트 2
9. `crates/editor/src/display_map/inlay_map.rs` — `chunks` 시그니처 + 테스트 3
10. `crates/editor/src/display_map/tab_map.rs` — `chunks` 시그니처 + 내부 호출 3 + 테스트 1
11. `crates/editor/src/display_map/wrap_map.rs` — `chunks` 시그니처 + 내부 호출 3 + 테스트 1

### Callers
12. `crates/editor/src/editor.rs` — import + 2개 호출
13. `crates/editor/src/element.rs` — import + 2개 호출 (`use_tree_sitter` → `LanguageAwareStyling { tree_sitter: use_tree_sitter, diagnostics: true }`)
14. `crates/outline_panel/src/outline_panel.rs` — import + 1개 호출
15. `crates/project/src/lsp_store.rs` — import + 1개 호출
16. `crates/project/tests/integration/project_tests.rs` — import + 1개 호출
17. `crates/vim/src/state.rs` — import + 2개 호출

### 생략
- `crates/editor/src/semantic_tokens.rs` +130줄 신규 테스트는 이식 생략

## 수정 제외 (가드레일)
- `semantic_tokens.rs` 테스트 추가 생략
- Dokkaebi 독자 `use_tree_sitter_for_syntax()` 메서드는 보존

## 작업 단계
- [ ] 1. `buffer.rs` + tests: `LanguageAwareStyling` 정의 + 시그니처 변경 → `cargo check -p language`
- [ ] 2. `multi_buffer.rs` + tests: 시그니처 전파 → `cargo check -p multi_buffer`
- [ ] 3. `display_map` 체인 7개 파일 일괄 수정 → `cargo check -p editor`
- [ ] 4. Callers 6개 파일 호출 migrate → `cargo check -p editor -p outline_panel -p project -p vim`
- [ ] 5. 전체 `cargo check -p Dokkaebi` 최종 검증
- [ ] 6. notes.md 갱신 + commit + push

## 런타임 영향
- `semantic_tokens: "full"` 설정한 사용자에게만 실질 효과(진단 밑줄 복구)
- 기본값 사용자는 영향 없음(동일 동작: `LanguageAwareStyling { tree_sitter: true, diagnostics: true }`)

## 승인 필요 사항
- 사용자 "a 진행" 승인 완료
