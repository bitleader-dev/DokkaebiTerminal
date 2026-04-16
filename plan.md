# i18n 하드코딩 문자열 치환 (2026-04-16)

## 목표
다이얼로그(window.prompt), 툴팁(Tooltip::text), 버튼(Button::new), 라벨(Label::new)에
하드코딩된 영문 문자열을 i18n 키 호출로 치환한다.

## 범위 (수정 대상 파일)
### 다이얼로그
1. `crates/workspace/src/workspace.rs` — restart 다이얼로그
2. `crates/zed/src/zed.rs` — quit 다이얼로그, unsupported GPU 다이얼로그
3. `crates/rules_library/src/rules_library.rs` — delete 다이얼로그 (t_args)
4. `crates/search/src/project_search.rs` — unsaved edits 다이얼로그
5. `crates/workspace/src/pane.rs` — save changes 다이얼로그

### Agent UI 툴팁/버튼/라벨
6. `crates/agent_ui/src/agent_diff.rs`
7. `crates/agent_ui/src/conversation_view/thread_view.rs`
8. `crates/agent_ui/src/text_thread_editor.rs`
9. `crates/agent_ui/src/text_thread_history.rs`
10. `crates/agent_ui/src/agent_configuration.rs`

### 기타
11. `crates/title_bar/src/application_menu.rs`
12. `crates/settings_ui/src/components/input_field.rs`
13. `crates/recent_projects/src/recent_projects.rs`

## 수정 제외
- `project_panel` 삭제 다이얼로그 (이미 처리됨)
- `git_panel.rs:1401` 근처 삭제 다이얼로그 (이미 처리됨)
- macOS/Linux 전용 코드
- README.md

## 작업 단계
- [x] 1. 각 파일의 실제 위치 grep으로 확인
- [x] 2. 파일별 Edit으로 치환
- [x] 3. cargo check로 빌드 검증
- [x] 4. notes.md 갱신

## 승인 필요 사항
- 없음 (기존 i18n 키 사용, 문자열만 치환)
