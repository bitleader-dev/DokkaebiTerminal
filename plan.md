# 코드 리뷰 지적사항 수정

## 목표
- 코드 리뷰에서 발견된 6개 이슈 수정

## 범위

### 수정 파일

| 파일 | 변경 내용 |
|------|-----------|
| `crates/notepad_panel/src/notepad_panel.rs` | SettingsStore 옵저버 변경 감지 추가 + soft_wrap 호출 통일 + 주석 개선 |
| `crates/agent_ui/src/conversation_view.rs` | ZED_AGENT_ID 중복 비교 제거 |
| `crates/settings_ui/src/page_data.rs` | "Notepad Restore" 설정 i18n 키 적용 |
| `assets/locales/ko.json` | `settings.notepad_panel.restore.*` 키 추가 |
| `assets/locales/en.json` | `settings.notepad_panel.restore.*` 키 추가 |

### 수정하지 않는 것
- 구조 변경, 새 의존성, 공개 API 변경 없음

## 작업 단계

### [x] 1. `notepad_panel.rs` — SettingsStore 옵저버 개선
- `NotepadPanel` 구조체에 `last_horizontal_scroll: bool` 필드 추가
- 옵저버 콜백에서 이전 값과 비교 → 변경 시에만 `set_soft_wrap_mode()` 호출
- 초기화 블록의 `set_soft_wrap()` → `set_soft_wrap_mode(SoftWrap::EditorWidth, cx)`로 통일
- WHAT 주석 → WHY 주석으로 변경

### [x] 2. `conversation_view.rs` — ZED_AGENT_ID 비교 통합
- `is_native_agent` 변수로 한 번만 비교 후 재사용

### [x] 3. `page_data.rs` + i18n 파일 — "Notepad Restore" i18n 적용
- title/description을 i18n 키로 변경
- `ko.json`, `en.json`에 `settings.notepad_panel.restore.title/description` 키 추가

### [x] 4. cargo check 검증

## 검증 방법
- `cargo check` 통과

## 승인 필요 사항
- 없음
