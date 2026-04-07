# 앱 실행 속도 개선

## 목표
- 앱 시작 시 첫 창이 표시되기까지의 시간을 단축한다.
- 핵심 전략: **UI 렌더링 전 동기 차단(block_on) 최소화** — 첫 창을 빨리 띄우고 나머지는 백그라운드 로드.

## 범위
- `crates/zed/src/main.rs` 초기화 흐름 내 동기 차단 작업 비동기화
- `crates/zed/src/zed.rs` 테마 eager 로드 방식 변경
- 컴파일 옵션/할당자 설정 검토

## 작업 단계

### [x] 1. 시작 시간 측정 기반 구축 (선행 작업)
- 각 초기화 단계에 `log::info!("[startup] ...")` 타이밍 로그 추가
- 기본 초기화, 설정/HTTP, 언어 레지스트리, 세션, 테마, 폰트, 컴포넌트, 총 시간 측정

### [x] 2. 테마 Eager Load 비동기화
- `zed.rs:2247` `eager_load_active_theme_and_icon_theme()`의 `block_on()` → `cx.spawn().detach()` 비동기 전환
- 로드 완료 전까지 빌트인 기본 테마로 렌더링, 완료 후 `reload_theme()`/`reload_icon_theme()` 호출

### [x] 3. 폰트 로드 분할
- `main.rs` `load_embedded_fonts()` 분할
- Phase 1 (동기): monospace 폰트(Lilex) 4개만 `block_on()`으로 즉시 로드
- Phase 2 (비동기): UI 폰트(IBM Plex Sans) 4개는 `cx.spawn().detach()`로 지연 로드

### [x] 4. 컴포넌트 초기화 지연 확대
- 13개 비필수 컴포넌트를 기존 `cx.spawn().detach()` 지연 블록으로 이동
- 이동 대상: journal, encoding_selector, language_selector, line_ending_selector,
  toolchain_selector, theme_selector, prompt_palette, settings_profile_selector,
  language_tools, onboarding, settings_ui, keymap_editor, json_schema_store
- 핵심 유지: editor, workspace, search, vim, terminal_view, notepad_panel, title_bar, git_ui 등

### [ ] 5. 언어 레지스트리 경량화 (향후 검토)
- miniprofiler 타이밍 로그로 실제 소요 시간 확인 후 판단
- 비용이 클 경우: 자주 쓰는 언어만 즉시 등록, 나머지 lazy

### [ ] 6. MiMalloc 할당자 활성화 (향후 검토)
- `crates/zed/Cargo.toml`에 mimalloc feature 기본 활성화
- 메모리 할당 성능 향상, 단 메모리 사용량 측정 필요

## 검증 결과
- `cargo check -p zed`: 빌드 성공 (기존 경고만 존재, 신규 에러/경고 없음)
