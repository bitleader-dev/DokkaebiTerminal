# Info(정보) 다이얼로그 커스터마이징

## 목표
- Info 다이얼로그를 Dokkaebi 전용 정보 화면으로 변경
- 버전 0.1.0, 원본 Zed 프로젝트 크레딧 및 링크 표시

## 범위

### 수정 파일

| 파일 | 변경 내용 |
|------|-----------|
| `crates/zed/Cargo.toml` | version → "0.1.0", `[package.metadata.dokkaebi]` 에 `upstream_version` 추가 |
| `crates/zed/build.rs` | `upstream_version`을 Cargo.toml에서 읽어 `DOKKAEBI_UPSTREAM_VERSION` 환경변수로 노출 |
| `crates/zed/src/zed.rs` | `about()` 함수를 워크스페이스 모달(`AboutDialog`) 방식으로 변경 |
| `crates/gpui_windows/src/window.rs` | 변경 없음 (네이티브 TaskDialog 대신 커스텀 모달 사용) |
| `assets/locales/ko.json` | `about.*` i18n 키 추가 |
| `assets/locales/en.json` | `about.*` i18n 키 추가 |

### 수정하지 않는 것
- `PlatformWindow::prompt()` 트레잇 시그니처 변경 없음
- 기존 프롬프트 시스템 변경 없음
- 새 의존성 추가 없음

## 설계

### 현재 구조
- `about()` → `window.prompt(PromptLevel::Info, ...)` → Windows 네이티브 TaskDialog
- 타이틀 "Info" 하드코딩, 링크 미지원, i18n 미지원

### 변경 후 구조
- `about()` → `workspace.toggle_modal::<AboutDialog>(...)` → 커스텀 gpui 모달
- i18n 지원, 클릭 가능한 링크, 커스텀 타이틀

### AboutDialog 구조
```
┌─────────────────────────────────┐
│ 정보                      (title) │
├─────────────────────────────────┤
│ Dokkaebi Dev 0.1.0 (debug)      │
│                                 │
│ Dokkaebi는 Zed 오픈소스 프로젝트  │
│ 기반으로 제작 되었습니다. (v0.231.0) │
│                                 │
│       [Copy]  [OK]              │
└─────────────────────────────────┘
```
- "Zed 오픈소스 프로젝트" 클릭 시 https://github.com/zed-industries/zed 열기
- 버전은 `env!("CARGO_PKG_VERSION")`에서 읽음
- 원본 버전은 `env!("DOKKAEBI_UPSTREAM_VERSION")`에서 읽음

## 작업 단계

### [x] 1. `crates/zed/Cargo.toml` — 버전 및 메타데이터 수정
- `version = "0.1.0"`
- `[package.metadata.dokkaebi]` 섹션에 `upstream_version = "0.231.0"` 추가

### [x] 2. `crates/zed/build.rs` — 원본 버전 환경변수 노출
- Cargo.toml 파일을 파싱하여 `upstream_version` 값을 읽고 `cargo:rustc-env=DOKKAEBI_UPSTREAM_VERSION=값` 출력

### [x] 3. `assets/locales/ko.json`, `en.json` — i18n 키 추가
- `about.title`: "정보" / "Info"
- `about.credit_prefix`: "Dokkaebi는 " / "Dokkaebi is based on the "
- `about.credit_link`: "Zed 오픈소스 프로젝트" / "Zed open source project"
- `about.credit_suffix`: " 기반으로 제작 되었습니다." / "."

### [x] 4. `crates/zed/src/zed.rs` — AboutDialog 구현
- `AboutDialog` 구조체 생성 (Render + Focusable + EventEmitter<DismissEvent> + ModalView)
- `about()` 함수를 `workspace.toggle_modal::<AboutDialog>()` 호출로 변경
- Copy 버튼: 버전 정보를 클립보드에 복사
- OK 버튼: 다이얼로그 닫기
- 링크 클릭: `cx.open_url()` 호출

### [x] 5. 검증
- `cargo check` 통과

## 검증 방법
- `cargo check` 빌드 확인

## 승인 필요 사항
- `PlatformWindow` 트레잇 변경 없음
- 기존 `about()` 함수의 동작 방식 변경 (네이티브 TaskDialog → 커스텀 모달): **승인 필요**
