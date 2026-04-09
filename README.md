# Dokkaebi

[![CI](https://github.com/zed-industries/zed/actions/workflows/run_tests.yml/badge.svg)](https://github.com/zed-industries/zed/actions/workflows/run_tests.yml)

Dokkaebi는 [Zed](https://github.com/zed-industries/zed) 기반으로 개발된 고성능 터미널로, AI 코딩 에이전트 및 멀티태스킹 워크플로우에 최적화되어 있습니다.

---

### 주요 기능

- **AI 코딩 에이전트 통합** — Claude Code, Gemini CLI, Codex, OpenCode 지원
- **멀티태스킹** — 여러 에이전트 세션을 동시에 운용
- **Zed 기반** — Zed 에디터의 고성능 렌더링 및 멀티플레이어 아키텍처 활용

---

### 설치

> 현재 개발 중입니다. 빌드 방법은 아래 개발 가이드를 참고하세요.

### 개발 환경 구성

- [Windows에서 빌드](./docs/src/development/windows.md)


### 라이선스

서드파티 의존성 라이선스 정보는 CI 통과를 위해 정확히 명시되어야 합니다.

[`cargo-about`](https://github.com/EmbarkStudios/cargo-about)를 사용하여 오픈소스 라이선스를 자동으로 관리합니다. CI 실패 시 다음을 확인하세요:

- 직접 생성한 크레이트에서 `no license specified` 오류 → 해당 크레이트의 `Cargo.toml` `[package]` 아래에 `publish = false` 추가
- 의존성에서 `failed to satisfy license requirements` 오류 → 라이선스 확인 후 `script/licenses/zed-licenses.toml`의 `accepted` 배열에 SPDX 식별자 추가
- `cargo-about`가 라이선스를 찾지 못하는 경우 → `script/licenses/zed-licenses.toml` 끝에 clarification 필드 추가 ([cargo-about 문서](https://embarkstudios.github.io/cargo-about/cli/generate/config.html#crate-configuration) 참고)

---

> Dokkaebi는 [Zed](https://zed.dev) 오픈소스 프로젝트를 기반으로 합니다.
