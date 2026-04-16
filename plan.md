# REPL/Notebook 크레이트 완전 제거 (2026-04-16)

## 목표
`crates/repl/` 크레이트 전체(Jupyter Notebook 편집기 + 인라인 REPL 기능)를 Dokkaebi에서 제거한다. Dokkaebi는 Windows 전용 + Zed cloud 미사용으로 Jupyter 커널 의존 기능이 사실상 dead 경로이며, 의존성 스택(`runtimelib`, `jupyter-protocol`, `jupyter-websocket-client`, `nbformat`)을 빌드에서 제외해 빌드 시간/바이너리 크기를 줄인다.

## 사전 조사 결과 (사실관계)
- `repl` 크레이트는 `crates/zed/` 에서만 참조됨 (다른 crate 연쇄 영향 없음 확인)
- Jupyter 관련 4개 의존성은 오직 `repl` 내부에서만 사용됨 → 워크스페이스 Cargo.toml 에서도 제거 가능
- `html_to_markdown` 은 repl 외에 `agent_ui`, `agent`, `assistant_slash_commands` 에서도 사용 → **유지**
- `settings_content/src/settings_content.rs:150` `pub repl: Option<ReplSettingsContent>`, `:1182 pub struct ReplSettingsContent` → 제거 대상
- macOS/Linux 키맵은 CLAUDE.md "대상 플랫폼" 규칙에 따라 **수정 제외**

## 범위 (수정 대상 파일)
1. `crates/repl/` 전체 디렉토리 — 삭제
2. `Cargo.toml` — `repl = { path = "crates/repl" }` 제거, Jupyter 4개 의존성(`runtimelib`, `jupyter-protocol`, `jupyter-websocket-client`, `nbformat`) 제거
3. `crates/zed/Cargo.toml:170,258` — `repl` 의존성 라인 2곳 제거
4. `crates/zed/src/main.rs` — `repl::init`(721줄), `repl::notebook::init`(758줄) 2줄 제거
5. `crates/zed/src/zed.rs` — `repl::init`(5106), `repl::notebook::init`(5107) 2줄 제거, `"repl",` namespace 목록(4877) 1줄 제거
6. `crates/zed/src/zed/quick_action_bar.rs` — `mod repl_menu;`(2), `render_repl_menu` 호출(618) 제거
7. `crates/zed/src/zed/quick_action_bar/repl_menu.rs` — 파일 삭제
8. `crates/feature_flags/src/flags.rs:3-7` — `NotebookFeatureFlag` struct + impl 제거
9. `crates/settings_content/src/settings_content.rs` — `repl` 필드(150), `ReplSettingsContent` struct(1182 및 주변) 제거
10. `assets/settings/default.json` — `"repl": {...}` 섹션 제거 (2394 근처)
11. `assets/keymaps/default-windows.json`:
    - 163-164: `repl::Run`, `repl::RunInPlace`
    - 1315-1340: `NotebookEditor` / `NotebookEditor > Editor` 컨텍스트 2개 섹션 전체
12. `assets/locales/ko.json` — `action.notebook::*`, `action.repl::*` 키 제거 (총 ~21개)
13. `assets/locales/en.json` — 동일 키 제거 (총 ~21개)

## 수정 제외 (가드레일)
- `assets/keymaps/default-macos.json`, `default-linux.json` — macOS/Linux 제외 원칙
- `crates/html_to_markdown/` — 다른 crate에서 사용 중이므로 의존성 유지
- README.md — 수정 금지 규칙
- `alacritty_terminal`, `async-tungstenite`, `jupyter-websocket-client` 중 repl 외 사용처가 있는지 실제 작업 중 재확인 (repl Cargo.toml 은 쓰지만 다른 crate에서도 쓰면 유지)

## 승인 필요 여부
- **승인 필요 (높음)**
- 구조 변경 ✓ (크레이트 1개 전체 삭제)
- 공개 API 변경 ✓ (`repl::*` 모든 pub 심볼 제거)
- 의존성 제거 ✓ (워크스페이스 Cargo.toml 변경)
- 대량 삭제 ✓ (파일 20+ 개, i18n 키 40+ 개)
- 되돌리기 난이도: 중간 (git revert로 가능)

## 작업 단계 (논리 묶음 4개)

### Phase 1: init 호출 제거 (컴파일은 깨지지만 방향 설정)
- [x] 1-1. `crates/zed/src/main.rs` `repl::init`, `repl::notebook::init` 2줄 제거
- [x] 1-2. `crates/zed/src/zed.rs` init 2줄 + namespace 목록 1줄 제거
- [x] 1-3. `crates/zed/src/zed/quick_action_bar.rs` mod/호출 제거
- [x] 1-4. `crates/zed/src/zed/quick_action_bar/repl_menu.rs` 삭제
- [x] 1-5. `crates/zed/Cargo.toml` repl 의존성 + test-support feature 3곳 제거

### Phase 2: 크레이트 및 워크스페이스 의존성 제거
- [x] 2-1. `crates/repl/` 디렉토리 전체 삭제
- [x] 2-2. 루트 `Cargo.toml` 에서 `repl` members + path + Jupyter 4개 의존성 제거
- [x] 2-3. `crates/feature_flags/src/flags.rs` `NotebookFeatureFlag` 제거
- [x] 2-4. `cargo check -p Dokkaebi` 성공 (1m 57s, exit 0, 신규 경고 0건)

### Phase 3: 설정 스키마 및 기본값 정리
- [x] 3-1. `crates/settings_content/src/settings_content.rs` `repl` 필드 + `ReplSettingsContent` struct 제거
- [x] 3-2. `crates/settings/src/vscode_import.rs` `repl: None` 줄 제거 (컴파일 에러 연쇄 수정)
- [x] 3-3. `assets/settings/default.json` `"jupyter"` + `"repl"` 두 섹션 제거
- [x] 3-4. `cargo check -p settings_content -p Dokkaebi` 성공 (1m 06s, exit 0)

### Phase 4: 키맵 및 i18n 정리
- [x] 4-1. `assets/keymaps/default-windows.json` `Editor && jupyter` 컨텍스트 + `NotebookEditor` 2개 섹션 제거
- [x] 4-2. `assets/locales/ko.json` — 21개 키 삭제
- [x] 4-3. `assets/locales/en.json` — 21개 키 삭제
- [x] 4-4. 최종 `cargo check -p Dokkaebi` 성공 (1.79s 증분, exit 0)
- [x] 4-5. JSON 파일 4개 표준 파서 유효성 검증 통과

### Phase 4a: 런타임 패닉 수정 (에디터 Jupyter 잔여 코드)
- [x] 4a-1. `crates/editor/src/editor_settings.rs` — `Jupyter` struct + `jupyter` 필드 + `jupyter_enabled` 메서드 + from_settings 초기화 제거
- [x] 4a-2. `crates/editor/src/editor.rs:2723-2725` — `"jupyter"` 키 컨텍스트 추가 코드 제거
- [x] 4a-3. `crates/settings_content/src/editor.rs` — `JupyterContent` struct + `jupyter` 필드 + 잔여 매크로 + HashMap import 제거
- [x] 4a-4. `crates/settings/src/vscode_import.rs:274` — `jupyter: None` 제거
- [x] 4a-5. `cargo check -p editor -p settings_content -p Dokkaebi` 성공 (56.60s, exit 0)

### Phase 5: 문서 갱신 및 완료 보고
- [x] 5-1. `notes.md` 최근 변경 섹션 맨 위에 항목 추가
- [x] 5-2. 완료 보고

## 검증 방법
- 각 Phase 종료 시 `cargo check -p Dokkaebi` 실행 → exit 0 + repl 관련 신규 경고 0 확인
- 키맵/설정 JSON 은 주석·후행쉼표 제거한 텍스트를 표준 JSON 파서로 검증
- 런타임 확인은 사용자 환경에서 앱 빌드·실행 후 수행 (1) `.ipynb` 파일이 일반 텍스트로 열리는지, (2) 툴바에 REPL 메뉴가 사라졌는지, (3) 이전 `ctrl-shift-enter`(repl::Run)가 다른 동작(editor::NewlineBelow 등)으로 되돌아갔는지

## 롤백 방안
- 문제가 커질 경우 `git revert <commit>` 로 일괄 복원
- 분할 커밋 4개(Phase별)로 작업하면 특정 Phase만 롤백 가능

## 예상 영향
- `.ipynb` 파일: 일반 텍스트 에디터로 열림
- 툴바 REPL 메뉴: 사라짐
- `ctrl-shift-enter`, `ctrl-alt-enter` 바인딩: 기존 에디터/다른 컨텍스트 동작으로 폴백 (테스트 필요)
- 기존 사용자 settings.json 의 `"repl": {...}` 섹션: unknown field 로 스킵됨 (경고 수준, 에러 아님)
- 빌드 시간/바이너리 크기: Jupyter 스택 제거로 감소 (수치는 실측 필요)

## 진행 표시
- [ ] 예정 / [/] 진행 중 / [x] 검증 완료
