# 설정 화면 dead/미지원 옵션 정리 (2026-04-15)

## 목표
Dokkaebi(Windows 전용, Zed cloud 비활성) 환경에서 **동작하지 않거나 사용되지 않는** 설정 항목을 설정 UI에서 제거.

## 작업 범위

### 파트 1: dead/플랫폼 전용 항목 제거 (①번)

| # | 대상 | 이유 |
|---|---|---|
| 1 | `title_bar.show_onboarding_banner` (+ title_bar 섹션 전체) | 설정 스키마/UI/로케일에만 존재, UI 렌더링 코드 어디도 이 값을 읽지 않음. 완전 dead |
| 2 | `terminal.option_as_meta` | macOS Option 키 전용. Windows에서는 `!cfg!(target_os = "macos")` 조건으로 값과 무관하게 항상 meta 처리(`terminal/src/mappings/keys.rs:221`) |
| 3 | 로케일 dead 문자열 2건 | 이전 "창" 섹션 제거 작업의 구버전 description 직접 문자열 잔재 |

### 파트 2: Edit Predictions "Cloud AI" 기본 제거 (②번 B-권장)

| # | 대상 | 변경 내용 |
|---|---|---|
| 4 | `crates/edit_prediction_ui/src/edit_prediction_button.rs:1428` | `providers.push(EditPredictionProvider::Dokkaebi);` 1줄 제거 → Active Provider 드롭다운에서 "Cloud AI" 항목 숨김 |
| 5 | `assets/settings/default.json:1568` | `"provider": "dokkaebi"` → `"provider": "none"` 변경 (새 사용자 기본값이 Dokkaebi cloud를 가리키지 않도록) |
| 6 | `crates/edit_prediction/src/onboarding_modal.rs:70` | 온보딩 모달에서 `set_edit_prediction_provider(EditPredictionProvider::Dokkaebi, cx)` → `EditPredictionProvider::None` 으로 변경 |

## 가드레일
- **`EditPredictionProvider::Dokkaebi` enum variant 자체는 유지** — 수많은 곳(edit_prediction.rs, edit_prediction_button.rs 등)에서 분기 조건으로 사용. variant 제거 시 리팩토링 범위 폭증
- **`default.json` 외 사용자 설정 파일에 이미 "dokkaebi"가 박혀 있는 경우**는 그대로 유지 — 기존 동작 보존
- `settings_content` 의 내부 데이터 타입(`workspace.use_system_window_tabs`, `workspace.window_decorations`, `terminal.option_as_meta`, `title_bar.show_onboarding_banner`)은 GPUI/하위 레이어 호환을 위해 절대 건드리지 않음. 설정 UI 노출만 제거
- `ResetOnboarding` action(edit_prediction.rs:2962)은 이미 `::None`으로 설정하므로 변경 불필요
- i18n 키는 사용 여부 확인 후 dead만 제거 (다른 곳에서도 쓰면 유지)

## 작업 단계
- [x] 1. `page_data.rs`의 `title_bar_section()` 함수 제거 + `window_and_layout_page()` 호출 제거
- [x] 2. `page_data.rs`의 `behavior_settings_section()` 에서 `option_as_meta` SettingItem 제거 + 배열 크기 `[4]` → `[3]` 조정
- [x] 3. `ko.json` / `en.json` 에서 관련 i18n 키 삭제 + dead 로케일 2건 삭제
- [x] 4. `edit_prediction_button.rs:1428` 한 줄 제거
- [x] 5. `default.json:1568` `"dokkaebi"` → `"none"` 변경
- [x] 6. `onboarding_modal.rs:70` `Dokkaebi` → `None` 변경
- [x] 7. `cargo check -p settings_ui -p edit_prediction_ui -p edit_prediction` 검증 — 성공 (9.15s, exit 0, 신규 경고 0건)
- [x] 8. `notes.md` 갱신
- [x] 9. 완료 보고

## 승인 상태
- 사용자 승인: "B-권장, ①번 dead 3건 같이 진행" (2026-04-15)

## 진행 표시
- [ ] 예정 / [/] 진행 중 / [x] 검증 완료
