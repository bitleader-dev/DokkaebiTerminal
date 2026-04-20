# 토스트 팝업 표시 시간 설정 추가 (5~300초, 기본 5)

## 목표
설정 > 알림 페이지에 **"토스트 팝업"** 섹션을 신설하고 그 안에 **"토스트 팝업 표시 시간"** 숫자 입력 항목 추가. 값 범위 5~300초, 기본 5초. 설정값은 Claude Code 작업 알림 토스트(Stop/Idle)의 auto-dismiss 타이머에 반영된다. Permission 토스트는 기존대로 수동 dismiss 유지(자동 dismiss 없음).

## 이전 작업 상태 (참고, 이 plan 과 독립)
- 2026-04-21 "발신 터미널 타겟팅" 작업은 코드 수정 + 문서 갱신 완료, 수동 검증/커밋 대기. 이번 작업과 겹치는 파일은 `crates/zed/src/zed/open_listener.rs` 1곳뿐이고 수정 지점이 다르므로 충돌 없음. notes.md/release_notes.md 에 이미 해당 항목 존재.
- 커밋 분리 방침(승인 #3-A)은 유효. 이번 작업도 자체 커밋 대상이며 사용자 명시 요청 시에만 실행.

## 수정 범위 (파일 4개)

### 1. `crates/settings_content/src/settings_content.rs`
`NotificationSettingsContent` 에 `toast_display_seconds: Option<u32>` 필드 신규. 한글 doc 주석.
```rust
/// 작업 알림 토스트 팝업(Stop/Idle)의 auto-dismiss 시간(초).
/// 5~300 범위로 사용되며, 범위 밖 값은 런타임에서 clamp 처리된다.
/// Permission 토스트는 승인 응답이 필요하므로 이 설정의 영향을 받지 않는다.
///
/// Default: 5
pub toast_display_seconds: Option<u32>,
```
- 이 파일의 이 struct 는 `#[with_fallible_options]` + `MergeFrom` 적용이므로 필드 추가만으로 JSON schema/merge 경로가 자동 반영됨(기존 `task_alert_toast` 패턴과 동일).

### 2. `crates/settings_ui/src/page_data.rs`
`notification_page()` 안에 신규 내부 함수 + 섹션 결합 추가.

```rust
// 토스트 표시 시간 기본값. 설정 미지정 시 5초로 동작.
static DEFAULT_TOAST_DISPLAY_SECONDS: u32 = 5;

fn toast_popup_section() -> [SettingsPageItem; 2] {
    [
        SettingsPageItem::SectionHeader("settings_page.section.toast_popup"),
        SettingsPageItem::SettingItem(SettingItem {
            title: "settings_page.item.toast_display_seconds",
            description: "settings_page.desc.toast_display_seconds",
            field: Box::new(SettingField {
                json_path: Some("notification.toast_display_seconds"),
                pick: |settings_content| {
                    Some(
                        settings_content
                            .notification
                            .as_ref()
                            .and_then(|n| n.toast_display_seconds.as_ref())
                            .unwrap_or(&DEFAULT_TOAST_DISPLAY_SECONDS),
                    )
                },
                write: |settings_content, value| {
                    settings_content
                        .notification
                        .get_or_insert_with(Default::default)
                        .toast_display_seconds = value;
                },
            }),
            metadata: None,
            files: USER,
        }),
    ]
}
```

- `concat_sections![claude_code_section()]` → `concat_sections![claude_code_section(), toast_popup_section()]` 로 확장.
- `u32` 는 `init_renderers` 의 `add_basic_renderer::<u32>(render_editable_number_field)` 로 자동 렌더되므로 추가 UI 코드 불필요.

### 3. `crates/zed/src/zed/open_listener.rs`
- `handle_notify_request` 의 설정 읽기 블록을 3-tuple 로 확장:
  ```rust
  let (task_alert_enabled, toast_enabled, toast_display_secs) = cx.update(|cx| {
      let settings = SettingsStore::global(cx)
          .raw_user_settings()
          .and_then(|user| user.content.notification.as_ref());
      (
          settings.and_then(|n| n.task_alert).unwrap_or(true),
          settings.and_then(|n| n.task_alert_toast).unwrap_or(true),
          settings
              .and_then(|n| n.toast_display_seconds)
              .unwrap_or(5)
              .clamp(5, 300),
      )
  });
  ```
- Stop/Idle auto-dismiss 타이머 `Duration::from_secs(5)` 를 `Duration::from_secs(toast_display_secs as u64)` 로 교체.
- 기존 주석 "Stop/Idle 토스트는 5초 자동 dismiss. Permission 은 승인 응답이 필요하므로..." 를 "Stop/Idle 토스트는 `toast_display_seconds` (기본 5초, 5~300 clamp) 경과 후 자동 dismiss" 로 갱신.

### 4. i18n — `assets/locales/ko.json` + `assets/locales/en.json`
3개 키 추가 (ko/en 양쪽):
- `settings_page.section.toast_popup` — "토스트 팝업" / "Toast Popup"
- `settings_page.item.toast_display_seconds` — "토스트 팝업 표시 시간 (초)" / "Toast Popup Display Duration (sec)"
- `settings_page.desc.toast_display_seconds` — "작업 완료·입력 대기 토스트가 자동으로 닫히기까지의 시간(초). 5~300초 범위, 기본 5초. 권한 요청 토스트는 이 설정의 영향을 받지 않음." / 영문 동등 표현

## 수정하지 않음
- `assets/settings/default.json` — 기존 `notification` 섹션에 `task_alert` / `task_alert_toast` 키가 없이도 런타임 기본값으로 동작하는 패턴을 따라 `toast_display_seconds` 도 default.json 에 명시하지 않는다(추가하면 기존 노이즈와 다르게 튀고, Option::None 처리가 기본 경로).
- `render_editable_number_field` 렌더러 자체 — UI 수준 min/max 강제는 현재 `SettingsFieldMetadata` 에 필드가 없어 구조 변경이 필요하며, 본 작업 범위를 넘음. 사용자가 범위 밖 값을 입력하면 open_listener.rs 의 `clamp(5, 300)` 로 보정되고 설명 문구가 범위를 안내.
- Permission 토스트 auto-dismiss 동작 — 변경 없음(설정 무관하게 수동 dismiss 유지).
- 설정 UI 의 "Claude Code" 섹션(플러그인/작업 알림/토스트 팝업 알림 3항목) — 그대로 유지. 신규 "토스트 팝업" 섹션은 그 아래에 별도 추가.

## 영향 범위
- **공개 API**: `NotificationSettingsContent` 에 Optional 필드 추가. 기존 소비자 영향 없음(모두 Option 기본 None).
- **스키마**: JSON schema 는 `with_fallible_options` 매크로로 자동 반영.
- **런타임**: Stop/Idle 토스트 dismiss 지연시간만 바뀜. 미설정/default 에서는 기존 5초와 동일.
- **i18n**: ko/en 에 키 3개 추가.

## 승인 필요 사항 (CLAUDE.md 1단계)
- 공개 설정 스키마(`NotificationSettingsContent`) 필드 추가 → **승인 필요 조건 "공개 API 변경"에 해당**.
- 설정 UI 섹션 신설 → 화면 구성 변경(사용자 체감).

### 승인 요청 항목
1. **UI 배치** — 새 "토스트 팝업" 섹션을 기존 "Claude Code" 섹션 **아래**에 추가 (권장). 섹션 순서를 바꾸거나 "Claude Code" 섹션 안에 병합하길 원하시면 알려주세요.
2. **범위 밖 입력 처리** — UI 수준 강제 없이 백엔드 `clamp(5, 300)` 으로만 보정(권장). 또는 `SettingsFieldMetadata` 에 min/max 필드를 추가해 UI 에서 입력 단계부터 막는 안(→ 별도 구조 변경 작업 필요).
3. **표시 단위** — 설정 라벨·설명 문구를 "초(sec)" 단위로 표기(권장). 분 단위 원하시면 수정.

## 검증
1. `cargo check -p Dokkaebi -p settings_ui -p settings_content` 클린.
2. 수동:
   - (T1) 설정 > 알림 진입 → "토스트 팝업" 섹션 표시 + 숫자 입력 필드가 5로 표시되는지.
   - (T2) 값을 30 으로 변경 → Claude Code 작업 완료 시 토스트가 약 30초 뒤 사라지는지.
   - (T3) 값을 2(범위 밖) 입력 → settings.json 에는 2 저장되지만 런타임은 5로 dismiss (clamp).
   - (T4) 값을 500(범위 밖) 입력 → 런타임은 300으로 dismiss.
   - (T5) Permission 토스트는 시간 경과해도 자동 dismiss 되지 않는지.
   - (T6) 입력 대기(Idle) 토스트에도 동일 시간 적용되는지.

## 승인 결과 (사용자 지시 "승인" → 모든 권장안 채택)
- **#1 UI 배치**: "토스트 팝업" 별도 섹션을 "Claude Code" 섹션 아래에 추가.
- **#2 범위 밖 입력**: UI 강제 없이 백엔드 `clamp(5, 300)` 만.
- **#3 표시 단위**: 초(sec) 단위 표기.

## 작업 단계
- [x] 1. 승인 완료
- [x] 2. `NotificationSettingsContent::toast_display_seconds` 필드 추가
- [x] 3. `page_data.rs` `toast_popup_section()` 추가 + `notification_page()` 결합
- [x] 4. `open_listener.rs` 설정 읽기 3-tuple 확장 + `Duration::from_secs(5)` → 동적
- [x] 5. i18n ko/en 키 3개 추가
- [x] 6. `cargo check -p Dokkaebi -p settings_ui -p settings_content` 클린 (44.63s, 신규 경고/에러 0건)
- [x] 7. `notes.md` 항목 추가
- [x] 8. `release_notes.md` "알림 메뉴 재구성" 항목 보강 (기본 5초 + 5~300초 조정 가능 안내 삽입)
- [/] 9. 수동 검증 — **사용자 검증 대기**
- [ ] 10. 커밋 — 사용자 명시 요청 시에만
