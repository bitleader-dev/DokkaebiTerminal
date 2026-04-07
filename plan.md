# 터미널 컨텍스트 메뉴 "작업 실행" — 현재 경로 전달

## 목표
- 터미널 컨텍스트 메뉴의 "작업 실행(Spawn Task)" 메뉴 클릭 시, 해당 터미널의 현재 작업 디렉토리(cwd)를 task에 전달하여 해당 경로에서 명령어가 실행되도록 구현

## 범위
- `TaskOverrides`에 `cwd` 필드 추가
- `toggle_modal_with_overrides` 함수 신설
- `terminal_view.rs`에서 터미널의 `working_directory()`를 캡처하여 전달
- `confirm`/`confirm_input`/`spawn_oneshot`에서 cwd override 적용

## 작업 단계

### [x] 1. TaskOverrides에 cwd 필드 추가
- `modal.rs`의 `TaskOverrides` 구조체에 `cwd: Option<PathBuf>` 추가
- `confirm()`, `confirm_input()`, `spawn_oneshot()`에서 cwd override를 `task.resolved.cwd`에 적용

### [x] 2. toggle_modal_with_overrides 함수 추가
- `tasks_ui.rs`에 `toggle_modal_with_overrides()` 함수 신설
- 기존 `toggle_modal()`은 이 함수를 호출하는 래퍼로 변경

### [x] 3. terminal_view에서 터미널 cwd를 전달
- `terminal_view` Cargo.toml에 `tasks_ui` 의존성 추가
- 컨텍스트 메뉴에서 `.entry()` 사용하여 터미널의 `working_directory()`를 캡처
- `TaskOverrides.cwd`로 `toggle_modal_with_overrides`에 전달

### [x] 4. 빌드 검증 및 문서 갱신
- `cargo check -p terminal_view` 성공
- `notes.md` 갱신
