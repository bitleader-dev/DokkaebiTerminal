# Zed v0.232.2 PR #52886 ESLint v3.0.24 백포트 (2026-04-16, 7차)

## 목표
ESLint LSP 서버 2.4.4 → 3.0.24 업그레이드 및 ESLint 8-10 버전별 config 처리 개선.

## 사전 조사 결과 (확정)
- `crates/languages/src/eslint.rs:37` 현재 `CURRENT_VERSION = "2.4.4"`
- `crates/languages/Cargo.toml:56` `semver.workspace = true` **이미 존재** → 의존성 추가 불필요
- workspace Cargo.toml L663 `semver = { version = "1.0", features = ["serde"] }` 선언됨
- `node_runtime::read_package_installed_version` 함수 `node_runtime.rs:774`에 존재 → 사용 가능
- `crates/languages/src/lib.rs` EsLintLspAdapter 생성자 호출 1곳 수정 필요

## 범위 (수정 대상 2개 파일)
1. `crates/languages/src/eslint.rs` (+394/-26)
   - `CURRENT_VERSION` `"2.4.4"` → `"3.0.24"`, tag name 동일 갱신
   - `FLAT_CONFIG_FILE_NAMES` 단일 상수 → `_V8_21`, `_V8_57`, `_V10` 3개로 분기
   - `struct EsLintLspAdapter`에 `fs: Arc<dyn Fs>` 필드 + `use` 추가(`node_runtime::read_package_installed_version`, `project::Fs`, `semver::Version`)
   - `new()`에 `fs` 인자 + 저장
   - Workspace configuration 생성 로직 재작성(설치된 ESLint 버전 감지 → 버전별 flat config 플래그 분기)

2. `crates/languages/src/lib.rs` (+1/-1)
   - `EsLintLspAdapter::new(node)` 호출에 `fs` 인자 추가

## 수정 제외 (가드레일)
- 업스트림 테스트 추가 생략 (있다면)
- 버전 감지 실패 시 fallback 동작 보수적 유지

## 작업 단계
- [ ] 1. 상류 eslint.rs 전체 patch 확보
- [ ] 2. lib.rs 호출처 확인
- [ ] 3. eslint.rs 전체 재작성
- [ ] 4. lib.rs 호출 시그니처 업데이트
- [ ] 5. `cargo check -p languages` 빌드 검증
- [ ] 6. 전체 `cargo check -p Dokkaebi` 최종 검증
- [ ] 7. notes.md 갱신 + commit + push

## 런타임 주의 사항
- 기존 ESLint 2.4.4 사용자가 앱 업데이트 후 첫 실행 시 3.0.24 LSP 바이너리를 새로 다운로드 (~수 MB, 네트워크 필요)
- 기존 설치(`vscode-eslint-2.4.4/`)는 자동 삭제되지 않음 (디스크에 잔존)
- 오프라인 환경 사용자는 ESLint LSP 로딩 실패 가능

## 승인 필요 사항
- 사용자 "(A-1) 진행"으로 승인 완료. `semver` 의존성은 이미 존재하므로 추가 승인 불필요.
