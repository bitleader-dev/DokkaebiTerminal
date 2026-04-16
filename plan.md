# Zed v0.232.2 PR #53033 show_merge_conflict_indicator 백포트 (2026-04-16)

## 목표
병합 충돌 알림을 우측 하단 팝업에서 **상태바 indicator**로 대체. 새 설정 `show_merge_conflict_indicator`로 토글 가능. 11개 파일 변경.

## 사전 조사 결과 (Dokkaebi 현재 상태)
- `crates/git_ui/src/conflict_view.rs:522` `register_conflict_notification` **존재** (이전 세션 i18n 한글화로 `i18n::t_args("conflict_view.unresolved_*", ...)` 사용 중)
- `crates/git_ui/src/git_ui.rs:70` `register_conflict_notification` 호출 **존재**
- `crates/workspace/src/workspace.rs:8175` `merge_conflict_notification_id()` **존재**
- `crates/agent_ui/src/conversation_view/thread_view.rs:816,883,891` suppress/unsuppress 호출 **존재** (L816은 #51756 백포트와 얽혀 있음)
- `IconName::GitMergeConflict` L153 **존재** (사용 가능)
- `IconName::DokkaebiAssistant` L279 **존재** (상류 `ZedAssistant` 대응, Dokkaebi 리네이밍 완료)
- 기존 `show_turn_stats` 필드 이미 `agent_settings.rs:54`, `settings_content/agent.rs:176`에 존재 → 유사 위치에 `show_merge_conflict_indicator` 추가

## 추가 고려 사항 — i18n
상류 patch의 `MergeConflictIndicator::render` 본문은 4개 영문 리터럴:
- `"Resolve Merge Conflict{s} with Agent"` (단/복수)
- `"Found {count} conflict{s} across the codebase"` (단/복수)
- `"Click to Resolve with Agent"` (tooltip meta)

Dokkaebi 정책상 UI 문자열은 i18n 필수. 이식 시 `i18n::t`/`i18n::t_args`로 치환하며 키 추가:
- `conflict_view.indicator.resolve_single` / `resolve_multi`
- `conflict_view.indicator.tooltip_single` / `tooltip_multi` (`{count}` placeholder)
- `conflict_view.indicator.tooltip_meta`

이전 세션에서 추가한 `conflict_view.unresolved_single|multi|resolve_with_agent` 3개 키는 `register_conflict_notification` 삭제와 함께 **고아 키** 됨 → ko/en.json에서 제거.

## 범위 (수정 대상 11개 파일)
1. `crates/settings_content/src/agent.rs` — `show_merge_conflict_indicator: Option<bool>` 필드 추가 (show_turn_stats 바로 뒤)
2. `crates/agent_settings/src/agent_settings.rs` — `AgentSettings` struct 필드 + `from_settings` 매핑
3. `crates/agent/src/tool_permissions.rs` — 테스트용 AgentSettings 빌더에 필드 추가 (1줄)
4. `crates/agent_ui/src/agent_ui.rs` — 테스트용 AgentSettings 빌더에 필드 추가 (1줄)
5. `crates/agent_ui/src/conversation_view/thread_view.rs` — `suppress_merge_conflict_notification`/`unsuppress_merge_conflict_notification` 메서드 + 호출 제거 (L816, L830, L845 근처)
6. `crates/workspace/src/workspace.rs:8175` — `merge_conflict_notification_id()` 함수 삭제 (5줄)
7. `crates/git_ui/src/conflict_view.rs` — `register_conflict_notification` 함수 + 관련 use (`RefCell`, `Rc`, `MessageNotification`, `notification_panel` 등) 제거, `MergeConflictIndicator` struct + Render + StatusItemView impl 신규 추가 (i18n 치환 포함)
8. `crates/git_ui/src/git_ui.rs` — `pub use conflict_view::MergeConflictIndicator;` 추가, `register_conflict_notification(workspace, cx)` 호출 제거
9. `crates/zed/src/zed.rs:initialize_workspace` — `MergeConflictIndicator::new(workspace, cx)` 생성 + `status_bar.add_left_item(...)` 추가
10. `crates/settings_ui/src/page_data.rs` — AI 설정 페이지에 "Show Merge Conflict Indicator" SettingItem 추가 (기존 배열 크기 +1, 위치 확인 필요)
11. `assets/settings/default.json` — `show_merge_conflict_indicator: true` 추가 + 주석
12. `assets/locales/ko.json`, `en.json` — 신규 5개 키 추가, 고아 3개 키 제거

## 수정 제외 (가드레일)
- `IconName::ZedAssistant` → 이미 Dokkaebi는 `DokkaebiAssistant` 쓰지만 해당 함수 자체 삭제되므로 참조 정리
- 상류의 다른 코드 스타일 변경 최소 반영

## 작업 단계
- [ ] 1. settings_content/agent.rs 이식
- [ ] 2. agent_settings 이식 (필드 + from_settings)
- [ ] 3. tool_permissions.rs + agent_ui.rs 테스트 빌더 수정 + `cargo check -p agent_settings -p agent -p agent_ui`
- [ ] 4. workspace/src/workspace.rs `merge_conflict_notification_id` 제거
- [ ] 5. thread_view.rs suppress/unsuppress 관련 제거
- [ ] 6. git_ui/git_ui.rs 호출 제거 + pub use 추가
- [ ] 7. conflict_view.rs 대규모 재작성 (register_conflict_notification 삭제 + MergeConflictIndicator 추가)
- [ ] 8. ko/en.json i18n 키 교체 (신규 5개 + 고아 3개 제거)
- [ ] 9. zed/src/zed.rs 상태바 항목 추가
- [ ] 10. settings_ui/page_data.rs 설정 UI 항목 추가
- [ ] 11. default.json 기본값 추가
- [ ] 12. `cargo check -p Dokkaebi` 최종 검증
- [ ] 13. notes.md 갱신 + git commit + push

## 승인 필요 사항
- 사용자 "a" 선택으로 승인 완료
