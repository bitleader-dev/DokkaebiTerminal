# 언어 드롭다운 고도화 — 표시 이름 + 시스템 언어 옵션 (2026-04-17)

## 목표
1) 언어 드롭다운 표시 이름을 "English"/"한국어"로 변경
2) 맨 위에 "시스템 언어" 항목 추가 + **기본값**으로 설정
3) `Locale::System` 선택 시 OS 언어(Windows 사용자 UI 언어)를 감지해 실제 UI 문자열을 그 언어로 표시. "ko"로 시작하면 Ko, 그 외는 En으로 폴백.

## 핵심 설계
- `Locale` enum에 `System` variant 추가 + `#[default]`를 `System`으로 이동
- `I18n` 글로벌에 `effective_locale` 필드 추가 — `locale`이 `System`이면 OS 감지 결과로 해석, 아니면 자기 자신. 실제 번역 lookup은 `effective_locale` 사용. `current_locale()`은 설정값(`System`/`En`/`Ko`) 그대로 반환(드롭다운 선택 유지용)
- `Locale` variant에 `#[strum(serialize = "locale.system|en|ko")]` 속성 부여 → `VariantNames::VARIANTS`가 i18n 키로 바로 사용되어 드롭다운이 "시스템 언어"/"English"/"한국어" 표시
- OS 감지는 workspace에 이미 존재하는 `sys-locale = "0.3.1"` 사용

## 범위 (수정 대상 파일)
1. `crates/settings_content/src/locale.rs`
   - `System` variant(맨 위) 추가 + `#[default]` 이동
   - `#[strum(serialize = "locale.system|en|ko")]` attribute 3개
2. `crates/i18n/Cargo.toml`
   - `sys-locale.workspace = true` 의존성 추가
3. `crates/i18n/src/i18n.rs`
   - `I18n` struct에 `effective_locale: Locale` 추가
   - `set_locale`에서 `System`일 때 OS 감지 결과 저장, 그 외는 locale 그대로
   - `translate`에서 `effective_locale` 사용
   - 신규 헬퍼 `detect_os_locale()` — `sys_locale::get_locale()` 파싱
4. `crates/theme_selector/src/locale_selector.rs`
   - `languages` 목록 맨 앞에 `System` 항목 추가(`display_name`은 i18n 키 `language_selector.system`)
5. `assets/locales/ko.json`, `en.json`
   - `locale.system`, `locale.en`, `locale.ko` 3개 키 (ko: "시스템 언어"/"English"/"한국어", en: "System"/"English"/"Korean")
   - `language_selector.system` 키 (ko: "시스템 언어", en: "System")

## 구조/공개 API 영향
- `Locale` enum variant 추가 — 공개 API 변경에 해당하나, 사용자가 명시적으로 "시스템 언어" 추가를 요청해 사전 승인됨
- `#[default]` 변경 — 기존 설정에 `locale` 필드가 없던 사용자의 기본 동작이 "en 고정" → "OS 언어 자동"으로 바뀜. Dokkaebi는 한국어 포크이므로 한국어 OS 사용자에게 바람직한 방향
- `I18n`의 `locale` 필드 semantics 변경 — 내부 private 구조로 외부 crate 영향 없음

## 작업 단계
- [x] 1. `locale.rs` — `System` variant + default + strum attribute 추가
- [x] 2. `i18n/Cargo.toml` — `sys-locale` 의존성 추가
- [x] 3. `i18n.rs` — `effective_locale` + `detect_os_locale` + set_locale/translate 수정
- [x] 4. `locale_selector.rs` — `System` 항목 추가
- [x] 5. `ko.json`/`en.json` — 4개 키 추가
- [x] 6. `cargo check -p settings_content` / `-p i18n` / `-p theme_selector` / `-p settings_ui` / `-p Dokkaebi` 검증
- [x] 7. `notes.md` 갱신
- [/] 8. 빌드 후 수동 검증 (사용자 위임)

## 검증 방법
- 각 크레이트 `cargo check`로 컴파일 에러 0건 확인
- 런타임: (a) 드롭다운에 "시스템 언어"/"English"/"한국어" 3항목 표시, (b) 기본값이 시스템 언어로 표시, (c) Windows 한국어 OS에서 System 선택 시 UI가 한국어 출력

## 승인 필요 사항
- `Locale` enum variant 추가 + `#[default]` 이동 — 사용자 명시 요청으로 사전 승인
