# Windows 윈도우 클래스 이름 Zed → Dokkaebi 리네이밍 plan v1

> **작성일**: 2026-04-30
> **상태**: ✅ 종료 (2026-04-30) — 기본 제안 모두 채택, 코드 2 파일 + CLAUDE.md + notes.md 갱신 완료
> **트리거**: 사용자 요청 — "현재 프로젝트에서는 윈도우 클래스 이름이 `Dokkaebi::Window` / `Dokkaebi::PlatformWindow` 이렇게 표시되도록 수정 가능한가?"

## 목표
Windows 에서 Dokkaebi 가 등록·생성하는 윈도우 클래스 이름을 상류 Zed 잔재 `Zed::Window` / `Zed::PlatformWindow` 에서 `Dokkaebi::Window` / `Dokkaebi::PlatformWindow` 로 교체. 외부 도구(AHK, Spy++, Inspect.exe, accessibility tool 등)가 윈도우 클래스로 Dokkaebi 창을 식별할 수 있게 한다.

## 범위
- **수정 파일 2개** — 모두 `crates/gpui_windows`:
  1. `crates/gpui_windows/src/window.rs:1247` — `const WINDOW_CLASS_NAME: PCWSTR = w!("Zed::Window");` → `w!("Dokkaebi::Window")`
  2. `crates/gpui_windows/src/platform.rs:1322` — `const PLATFORM_WINDOW_CLASS_NAME: PCWSTR = w!("Zed::PlatformWindow");` → `w!("Dokkaebi::PlatformWindow")`
- **CLAUDE.md 갱신** — 「이미 리네이밍된 식별자 (상류 동기화 시 충돌 주의)」 섹션에 본 항목 추가 (상류 PR이 윈도우 클래스 이름을 참조할 때 충돌 회피용)

## 영향 분석
### 충돌 / 호환성
- **윈도우 클래스 등록**: Windows 윈도우 클래스는 **프로세스별** 등록(`RegisterClassW` 의 `hInstance` 기준). 같은 머신에서 zed.exe 가 `Zed::Window` 를 등록해도 dokkaebi.exe 는 별도 프로세스라 충돌 없음. 본 변경 후에도 동일.
- **상수 노출 범위**: 두 상수 모두 `const`(미pub) — 같은 파일 내부에서 `RegisterClassW` 와 `CreateWindowExW` 양쪽에 사용. 외부 crate 에서 참조 0건(grep 으로 전수 확인).
- **실제 사용처**: `WINDOW_CLASS_NAME` 은 `register_window_class` (line 1249) 와 `CreateWindowExW` 호출(line 494) 2곳, `PLATFORM_WINDOW_CLASS_NAME` 은 `register_platform_window_class` (line 1324) 와 `CreateWindowExW` 호출(line 141) 2곳에서만 사용.

### 외부 도구 영향
- **자동화 스크립트(AHK, AutoIt 등)**: 클래스 이름으로 `FindWindow`/`WinExist` 하던 외부 스크립트가 있다면 `Zed::Window` → `Dokkaebi::Window` 로 갱신 필요. Dokkaebi 는 별도 앱이라 이는 의도된 동작.
- **접근성 도구(Inspect.exe, Spy++)**: 클래스 이름이 `Dokkaebi::*` 로 표시되어 Dokkaebi 식별이 직관적.
- **OS 자체**: 클래스 이름은 식별자일 뿐이라 OS 동작에는 영향 없음.

### Zed UI 백그라운드 표시 버그와의 관계
- 본 변경이 「원본 zed.exe 실행 시 Dokkaebi UI 미표시」 버그의 직접 fix 는 **아님**(클래스는 프로세스별이라 충돌하지 않음). 다만 식별자 분리는 디버깅·모니터링 도구로 두 앱을 명확히 구분할 수 있게 해 진단 보조에 유리.
- 본 plan 은 사용자 요청 「수정 가능한가」에만 응답. 위 버그의 진단·수정은 별도 plan 으로 진행 예정(사용자가 콘솔 메시지·작업 표시줄 동작·로그 확인 정보 제공 후).

## 작업 단계
1. **[x] 코드 수정 2곳**
   - `crates/gpui_windows/src/window.rs:1247` 문자열 교체
   - `crates/gpui_windows/src/platform.rs:1322` 문자열 교체
2. **[x] 검증**
   - `cargo check -p gpui_windows` 통과 — 신규 warning 0
   - `cargo check -p Dokkaebi` 통과 — 8 기존 warning 동일(`TryFutureExt` unused 등)
   - `cargo check -p Dokkaebi --tests` 통과 — 9 기존 warning 동일
   - 잔재 grep `Zed::Window`/`Zed::PlatformWindow` → 0건
3. **[x] 문서 갱신**
   - `CLAUDE.md` 「이미 리네이밍된 식별자」 섹션에 항목 추가
   - `notes.md` 「## 최근 변경」 섹션 맨 위에 항목 추가
   - `assets/release_notes.md` **갱신 제외** — 사용자 체감 동작 변화 없음
   - 버전 bump 없음 (v0.5.0 유지)

## 검증 방법
- `cargo check -p gpui_windows` 통과 (신규 warning 0)
- `cargo check -p Dokkaebi` 통과 (신규 warning 0)
- `cargo check -p Dokkaebi --tests` 통과 (memory `feedback_tests_check_on_api_removal` 규칙)
- 잔재 grep: `"Zed::Window"`, `"Zed::PlatformWindow"`, `Zed::Window`, `Zed::PlatformWindow` 매치 0건 확인

## 승인 필요 항목
- ✅ **plan 자체** — 본 plan 검토·승인
- 본 변경은 작업 분류상 「공개 API 변경 아님」「DB 스키마 변경 아님」「의존성 추가 아님」「대량 수정 아님」 → CLAUDE.md 1단계 승인 필수 조건 비해당. 다만 memory `feedback_plan_approval.md` 규칙상 모든 코드 작업 전 plan 승인 필수 → 본 plan 자체 승인이 곧 작업 승인.

## 결정 필요 사항 (사용자 답변 요청)
1. **CLAUDE.md 갱신 여부**: 본 변경을 「이미 리네이밍된 식별자」 섹션에 추가할까요? (기본 제안: **추가** — 향후 상류 동기화 시 윈도우 클래스 관련 PR 이 들어오면 충돌 인식·자동 치환 가이드)
2. **release_notes.md 갱신 여부**: 일반 사용자 가시 변경이 없어 기본 제외. 다만 「외부 자동화 스크립트 호환성 변경」 으로 보고 ` ### 외부 호환성` 카테고리에 1줄 기재할 수도 있음. (기본 제안: **제외**)
3. **버전 bump 여부**: 사용자 가시 동작 0 → 기본 제안 **bump 없음**.
