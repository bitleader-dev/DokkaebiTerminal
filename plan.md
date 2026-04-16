# Zed v0.232.2 중규모 Features 5건 백포트 (2026-04-16, 3차)

## 목표
중규모 Features 5건 순차 이식. 위험도·규모 순으로 진행, 각 단계 빌드 검증 필수.

## 사전 조사 결과

### 이식 순서 (안전순)
1. **#49881 JSX tag.component.jsx** — grammar 2파일, 각 6캡처 추가 (가장 단순)
2. **#53194 card layout padding** — `thread_view.rs` 1곳 5줄 변경
3. **#50582 OpenAI reasoning_effort** — 3파일(settings_content, open_ai_compatible, add_llm_provider_modal), 필드 추가 + 2회 clone 호출
4. **#50221 project_panel sort_order** — 새 `ProjectPanelSortOrder` enum + 정렬 함수 시그너처 변경(호출처 4곳) + settings/UI. 중규모
5. **#53033 show_merge_conflict_indicator** — 설정 + conflict_view.rs 대폭 리팩토링(알림 → 상태바 indicator). **규모 큼, 개별 검토**

### 주요 확인 사항 (#53033)
- `conflict_view.rs`가 현재 notification 방식으로 동작 중. 업스트림은 상태바 `StatusItemView`로 변경 + `register_conflict_notification` 함수 삭제
- `ThreadView::suppress_merge_conflict_notification/unsuppress_merge_conflict_notification` 제거
- Dokkaebi 현재 상태를 먼저 확인해야 리스크 판단 가능. 규모가 너무 크면 이번 plan에서 제외하고 별도 진행

## 범위 (수정 대상 파일)
### 1. PR #49881
- `crates/grammars/src/javascript/highlights.scm` JSX 요소 3곳 × 3캡처 = 9개 `@tag.component.jsx` 추가
- `crates/grammars/src/tsx/highlights.scm` 동일 패턴

### 2. PR #53194
- `crates/agent_ui/src/conversation_view/thread_view.rs:7390` 근처 card layout map 클로저 5줄 변경

### 3. PR #50582
- `crates/settings_content/src/language_model.rs` `OpenAiCompatibleAvailableModel`에 `reasoning_effort: Option<OpenAiReasoningEffort>` 필드 추가
- `crates/language_models/src/provider/open_ai_compatible.rs:402,417` 2곳 `None` → `self.model.reasoning_effort.clone()`
- `crates/agent_ui/src/agent_configuration/add_llm_provider_modal.rs:202` `ModelInput` 빌더에 `reasoning_effort: None` 1줄 추가

### 4. PR #50221
- `crates/settings/src/vscode_import.rs` — `ProjectPanelSortOrder` enum + `sort_order` 기본값
- `assets/settings/default.json` — `sort_order` 설정 주석 + 기본값 추가
- `crates/project_panel/src/project_panel_settings.rs` — `sort_order` 필드 추가
- `crates/project_panel/src/project_panel.rs` — `par_sort_worktree_entries_with_mode` → `par_sort_worktree_entries(entries, mode, order)` 시그너처 변경, 호출처 2곳 업데이트, `sort_order` 변화 감지 observer 추가
- benchmark 파일(`benches/sorting.rs`) 업데이트 — 생략 가능

### 5. PR #53033 (심화 조사 후 진행 여부 판단)
- **사전 확인**: 상류 patch의 `conflict_view.rs` 전체 재작성 → Dokkaebi 현재 구조와 대조 후 리스크 평가
- 조사 결과 편차 크면 본 plan에서 **제외**하고 사용자 재확인 후 별도 진행

## 수정 제외 (가드레일)
- 업스트림 테스트 추가(+각 10~50줄) 생략
- benchmark 파일 업데이트 필요 최소
- Dokkaebi 정책상 vim/macos/linux 키맵 미동기

## 작업 단계
- [x] 1. #49881 JSX 하이라이팅 이식 + grammars 파일 변경 완료
- [x] 2. #53194 card layout padding 이식 완료
- [x] 3. #50582 reasoning_effort 이식 완료
- [~] 4. #50221 sort_order — **skip** (util::paths::compare_rel_paths_by 선행 의존성 필요)
- [~] 5. #53033 conflict_view — **skip** (300줄+ 대규모 리팩토링, 별도 승인 대상)
- [x] 6. 전체 `cargo check -p Dokkaebi` 최종 검증 (58.88s, exit 0)
- [x] 7. `notes.md` 갱신
- [ ] 8. git commit + push

## 승인 필요 사항
- 사용자 "a" 선택으로 승인 완료. #53033은 사전 조사 후 리스크 크면 별도 승인 요청.
