# /simplify 후속 — Claude plugin registry 공유 크레이트 추출 (옵션 B)

## 목표
`uninstall_claude_plugin_registration()`(cli) 과 `uninstall_plugin()`(settings_ui) 의 중복된 JSON 편집 로직 + 상수(`PLUGIN_NAME`/`MARKETPLACE_NAME`/`ENABLED_KEY`)를 신규 경량 크레이트로 추출한다. 어느 한쪽이 바뀌면 다른 쪽이 조용히 깨지는 드리프트 위험을 구조적으로 제거.

## 배경
- /simplify 1차 정리에서 `KEEP IN SYNC` 주석만으로 완화했던 should-fix 1 항목.
- 사용자 B 옵션 승인 (2026-04-21).

## 설계

### 신규 크레이트 `crates/claude_plugin_registry`
- **공개 상수**: `PLUGIN_NAME` / `MARKETPLACE_NAME` / `ENABLED_KEY`
- **공개 함수**:
  - `settings_path() -> Option<PathBuf>` — `~/.claude/settings.json` 경로 (`dirs::home_dir` 기반)
  - `read_settings() -> Option<Value>` — 파일 부재/파싱 실패 시 None
  - `write_settings(&Value) -> bool` — parent dir 생성 + pretty write
  - `remove_plugin_registration() -> Result<bool, String>` — `Ok(true)` 변경+저장 성공 / `Ok(false)` 변경 대상 없음 (파일 부재·손상 포함) / `Err` 저장 실패
  - `is_plugin_installed() -> bool` — `enabledPlugins` 에 prefix 매칭
- **의존성**: `serde_json.workspace = true`, `dirs.workspace = true`. gpui/util 등은 의존하지 않음 (cli 경로 가볍게 유지).

### cli 측
- `crates/cli/src/main.rs::uninstall_claude_plugin_registration()` 제거
- `crates/cli/src/main.rs::prune_object_if_empty` 제거
- 호출부를 `claude_plugin_registry::remove_plugin_registration()` 로 교체
- `util::paths::home_dir()` 사용 종료 (공유 크레이트가 `dirs` 사용)

### settings_ui 측
- `notification_setup.rs::claude_code_settings_path` / `read_claude_code_settings` / `write_claude_code_settings` 제거 → 공유 함수 호출
- 상수 3개 제거 → 공유 크레이트 상수 사용
- `plugin_installed_uncached()` → `claude_plugin_registry::is_plugin_installed()` 호출
- `install_plugin()` 은 유지 (marketplace 디렉터리 탐색은 settings_ui 전용 로직)하되 내부 JSON I/O는 공유 함수 사용
- `uninstall_plugin()` 은 thin wrapper로 축소 + 캐시 무효화만 유지
- `cleanup_legacy_marker_hook` 의 settings 읽기/쓰기도 공유 함수 사용

### workspace
- 루트 `Cargo.toml` members 에 `"crates/claude_plugin_registry"` 추가 (알파벳 순 `buffer_diff` 다음)
- 루트 `Cargo.toml` `[workspace.dependencies]` 에 `claude_plugin_registry = { path = "crates/claude_plugin_registry" }` 추가

## 작업 단계

- [x] (1) `crates/claude_plugin_registry/Cargo.toml` + `src/lib.rs` 생성
- [x] (2) 루트 `Cargo.toml` members + workspace.dependencies 추가
- [x] (3) `crates/cli/Cargo.toml` 에 `claude_plugin_registry.workspace = true` 추가
- [x] (4) `crates/cli/src/main.rs` 수정 (함수 2개 제거, 호출부 교체)
- [x] (5) `crates/settings_ui/Cargo.toml` 에 `claude_plugin_registry.workspace = true` 추가
- [x] (6) `crates/settings_ui/src/pages/notification_setup.rs` 공유 함수 호출로 리팩토링
- [x] (7) 검증: `cargo check -p cli -p settings_ui -p Dokkaebi` 클린

## 검증 방법
- `cargo check -p cli` — cli 경로 빌드 + 신규 deps 정합
- `cargo check -p settings_ui` — settings_ui 경로 + 공유 함수 호출
- `cargo check -p Dokkaebi` — 통합 빌드, 신규 경고/에러 0건 확인

## 수정하지 않음
- `marketplace_root_dir()` / `install_plugin()` 의 plugin 디렉터리 탐색 로직은 settings_ui 전용이라 공유 안 함.
- `is_plugin_installed` 의 TTL 캐시(`PLUGIN_INSTALLED_CACHE` Mutex)는 settings_ui 전용 (GPU 렌더 경로 최적화용). 공유 크레이트는 uncached 버전만 노출.
- `cleanup_legacy_marker_hook` 의 마이그레이션 로직은 settings_ui 전용 유지 (다음 메이저에서 제거 예정).

## 승인 필요 사항
- 신규 크레이트 생성 + workspace 구조 변경 + 의존성 추가 → CLAUDE.md 1단계 승인 대상
- **사용자 승인 완료** (2026-04-21, 옵션 B 선택)
