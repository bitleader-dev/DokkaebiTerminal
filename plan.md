# 라이선스 준수 및 상류 Zed 잔재 최종 정리 (진행 중)

## 현재 상태 체크포인트 (2026-04-13 저장)
전체 범위는 `전체 진행` 승인됨. 조사 후 추가 결정 필요한 3개 항목에서 대기 중.

## 완료 작업

### 1. ✅ NOTICE 파일 생성 (GPL §5(a) 변경 고지)
- 파일: `NOTICE` (루트)
- 내용: 원본 Zed 저작권 + Dokkaebi 수정 범위 7개 카테고리 요약 + Third-Party Attribution + Trademark Notice

### 2. ✅ `crates/zed/Cargo.toml:2` description 교체
- 변경 전: `"The fast, collaborative code editor."`
- 변경 후: `"Dokkaebi - a Windows-focused terminal workspace for AI coding agents, based on Zed."`

### 3. ✅ `.cloudflare/` 삭제
- `.cloudflare/docs-proxy/` (Worker + wrangler.toml)
- `.cloudflare/open-source-website-assets/` (Worker + wrangler.toml)
- `.cloudflare/README.md`

### 4. ✅ `tooling/xtask` workflow 생성 코드 제거
- `tooling/xtask/src/tasks/workflows/` 디렉토리 전체 (19개 파일)
- `tooling/xtask/src/tasks/workflow_checks.rs`
- `main.rs`에서 `Workflows`·`CheckWorkflows` enum/match arm 제거
- `tasks.rs`에서 `pub mod workflows;`, `pub mod workflow_checks;` 제거

### 5. ✅ `crates/explorer_command_injector/` 삭제
- 크레이트 전체 디렉토리 제거 (Appx 매니페스트 3개 포함)
- `Cargo.toml` workspace members에서 라인 제거
- 참조 크레이트 없음을 사전 Grep으로 확인

### 6. ✅ `assets/licenses.md` 생성 (2026-04-14)
- `cargo install cargo-about@0.8.2` (release 3m 33s) → `C:\Users\jongc\.cargo\bin\cargo-about.exe`
- `pwsh -NoProfile -ExecutionPolicy Bypass -File script/generate-licenses.ps1` 실행
- 결과: 28,526줄 / 1.5MB, 13개 라이선스 · 약 1,369개 크레이트 attribution
- `OpenLicenses` 액션(`crates/zed/src/zed.rs:184-195`)의 `asset_str::<Assets>("licenses.md")` 경로 정상 로드 가능
- 설정 팝업 메뉴에 "오픈소스 라이선스" 항목 추가 + `Quit` 앞 separator (`app_menus.rs`, `ko/en.json`)

### 7. ✅ dead script 8개 + docs/ 생태계 삭제 (2026-04-14, B 항목 확장)
- 스크립트 8개: `docs-strip-preview-callouts`, `docs-suggest`, `docs-suggest-publish`, `test-docs-suggest-batch`, `update_top_ranking_issues/`, `zed-local`, `squawk`, `generate-action-metadata`
- `docs/` 디렉토리 전체 (57 md, 2.0MB) — Zed 공식 mdBook 문서, Dokkaebi 빌드·배포와 무관
- `crates/docs_preprocessor/` 크레이트 — docs 빌드 전용, 연쇄 dead
- 메타 정리: `Cargo.toml` workspace members, `typos.toml` 스펠체크 제외 규칙, `.gitignore` actions.json 규칙

### 8. ✅ C 항목 옵션 3 — 사용자 노출 Zed URL 제거 (2026-04-14)
- `crates/zed/src/zed.rs`: Linux inotify / Windows ReadDirectoryChangesW 파일 감시 실패 프롬프트, GPU emulation 경고에서 `zed.dev/docs/linux|windows` URL 전부 제거. 프롬프트 버튼 라벨도 단순화 (`Troubleshoot and Quit` → `Quit`). About 크레딧 `ZED_REPO_URL`(github.com/zed-industries/zed)은 유지
- `crates/zed/src/main.rs`: `fail_to_open_window` stderr/ashpd notification body URL 제거, `Args` doc 코멘트 `<https://zed.dev>` 참조 제거. stderr 브랜드명도 Zed → Dokkaebi로 교체

### 9. ✅ D 항목 옵션 a — ZED_REPL_DOCUMENTATION 빈 문자열화 (2026-04-14)
- `crates/zed/src/zed/quick_action_bar/repl_menu.rs:18` 상수값 `""`로 교체, 사용처 `format!` 제거
- Jupyter 커널 안내 버튼 UI는 유지, 클릭 시 no-op

### 10. ✅ 키맵 편집기 정리 ①②③ (2026-04-14)
- ①`zed_actions/src/lib.rs` deprecated_aliases 속성 5줄 제거 (OpenSettings/OpenSettingsFile/OpenProjectSettings/OpenKeymapFile/OpenKeymap)
- ②`OpenDocs` 액션 전체 제거: zed_actions 정의 + `zed.rs` import·handler · `onboarding::DOCS_URL` 상수 · `vim::command.rs`의 `:h`/`:help` 매핑
- ③`KERNEL_DOCS_URL` 값 `""`로 교체 (D 옵션 a 패턴)
- 빌드 중 vim 크레이트에서 `OpenDocs` import 발견 → 같은 커밋에 정리

### 16. ✅ 액션 네임스페이스 zed → dokkaebi 이관 (방안 A) (2026-04-14)
- Rust: `zed_actions/lib.rs`(13건 + actions! 호출), `onboarding.rs`(2건), `component_preview.rs`(1건) namespace 속성 변경
- 하드코딩 문자열: `keymap_editor.rs:4078`, `vim/command.rs:1794` 수정
- 키맵 JSON 7개 파일 `"zed::"` → `"dokkaebi::"` 총 68건 일괄 치환 (사전에 NoAction/Unbind 없음 확인)
- i18n en/ko 각 47개 `action.zed::*` 키 → `action.dokkaebi::*`, NoAction/Unbind 키·값은 예약어 보존 위해 원복
- 예약어 보존: `zed::NoAction`, `zed::Unbind`, `zed::main`은 GPUI 프레임워크 레벨 하드코딩이라 유지
- 결과: 키맵 편집기에서 모든 액션이 `dokkaebi:` 네임스페이스로 표시

### 15. ✅ About 다이얼로그 크레딧 문구 정비 (2026-04-14)
- i18n(`ko/en.json`) 크레딧 문구 교체: 독립 프로젝트 + Zed Industries 비제휴 고지로 재작성
- `ZED_REPO_URL`(`github.com/zed-industries/zed`) → `ZED_PROJECT_URL`(`https://zed.dev/`)로 링크 교체·상수 이름 변경
- 업스트림 버전 `(v0.231.2)` 표시 제거: `AboutDialog.upstream_version` 필드 + `env!("DOKKAEBI_UPSTREAM_VERSION")` + `build.rs` 파싱 로직 + `Cargo.toml [package.metadata.dokkaebi]` 연쇄 삭제

### 14. ✅ 라이선스/브랜드 마감 3단계 — Zed 상표 최종 정리 (2026-04-14)
- **1단계 에러 메시지 브랜드 교체**: `main.rs` "Zed failed to launch" × 2 / "Zed System Specs" → "Dokkaebi"; `zed.rs` GPU 경고 "Zed uses..." → "Dokkaebi uses..."
- **2단계 Zed cloud 로그인 차단**: `default.json` `server_url: ""` 기본값 + `authenticate()` no-op → 앱 시작 자동 재로그인 차단, HTTP 요청 단계에서 실패
- **3단계 SaaS UI 브랜드 일반화 (방안 B)**: `ai_onboarding`·`edit_prediction_button`·`end_trial_upsell`·`thread_view`·`language_models/cloud`·`settings_content/language`에서 "Zed AI/Pro/Business/Student/Trial" 약 18건 → "AI/Pro/Business/Student/Trial" 브랜드 중립; `billing-support@zed.dev` 이메일 제거
- **범위 밖 스킵**: test-only 문자열, About 크레딧 `ZED_REPO_URL`(공정 사용), `zed://` 내부 스킴 파서
- 결과: 사용자 노출 경로에서 Zed 상표는 About 크레딧 1곳만 남음

### 13. ✅ zed_urls 남은 2개 함수 no-op + server_url/관련 import 제거 (2026-04-14)
- `edit_prediction_docs`·`acp_registry_blog` → `String::new()` 반환, doc 코멘트 Dokkaebi 컨텍스트
- 5개 함수 모두 no-op으로 `server_url` helper가 dead → 함수·settings/ClientSettings import 제거
- 사용처 3곳(extensions_ui, edit_prediction_button, agent_registry_ui)은 UI 구조 유지, 클릭 시 no-op
- 모듈 최종: `shared_agent_thread_url`(zed:// 내부 스킴)만 실제 기능, 나머지 5개 no-op

### 12. ✅ REPL/Jupyter 커널 안내 버튼 제거 (2026-04-14)
- `repl_menu.rs` `ZED_REPL_DOCUMENTATION` + IconButton 체이닝 제거, `render_repl_setup`은 kernel_selector만 노출
- `kernel_options.rs` `use KERNEL_DOCS_URL` + `render_footer` 전체 제거 (Picker trait default `None` 반환으로 fallback)
- `repl.rs` `KERNEL_DOCS_URL` 상수 완전 삭제
- D 옵션 a/③에서 no-op 상태였던 두 버튼을 "버튼 자체 제거(②)"로 전환. Jupyter는 외부 공식 문서에 위임

### 11. ✅ E 라운드 — OpenAccountSettings 연쇄 + zed_urls 계정 함수 no-op (2026-04-14)
- `zed_actions::OpenAccountSettings` 액션 정의 삭제 + `zed.rs` import·handler 제거
- `zed.rs`의 `use client::zed_urls;` import도 연쇄 제거 (OpenAccountSettings가 유일한 사용처였음)
- `client::zed_urls`의 `account_url`·`start_trial_url`·`upgrade_to_zed_pro_url` 3개 함수를 `String::new()` 반환으로 교체 (D 옵션 a + ③ 패턴 일관)
- UI 호출처 9곳은 UI 구조 유지, 클릭 시 no-op (SaaS UsageLimit/구독 상태 전제라 Dokkaebi에서 실제 노출 가능성 낮음)
- 범위 외 후속 검토 대상: `zed_urls`의 `edit_prediction_docs`·`acp_registry_blog` (zed.dev/docs·/blog 리다이렉트)

### 빌드 검증
- `cargo build -p Dokkaebi -p xtask` 성공 (5m 32s, 2026-04-13)
- `cargo build -p Dokkaebi` 성공 (2m 12s, 2026-04-14 A 항목 후)
- `cargo build -p Dokkaebi` 성공 (2.22s, 2026-04-14 B + docs 삭제 후)
- `cargo build -p Dokkaebi` 성공 (1m 49s, 2026-04-14 C 항목 옵션 3 후)
- `cargo build -p Dokkaebi` 성공 (1m 19s, 2026-04-14 D 항목 옵션 a 후)
- `cargo build -p Dokkaebi` 성공 (2m 04s, 2026-04-14 ①②③ 라운드 후)
- `cargo build -p Dokkaebi` 성공 (1m 23s, 2026-04-14 E 라운드 후)
- `cargo build -p Dokkaebi` 성공 (1m 41s, 2026-04-14 REPL 커널 안내 버튼 제거 후)
- `cargo build -p Dokkaebi` 성공 (2m 42s, 2026-04-14 zed_urls 5개 함수 no-op 완성 후)
- `cargo build -p Dokkaebi` 성공 (각 단계별 1m 14s / 1m 16s / 1m 42s / 2m 54s, 2026-04-14 3단계 마감 작업)
- `cargo build -p Dokkaebi` 성공 (1m 39s, 2026-04-14 About 크레딧 정비 후)
- 경고 6건은 기존 unused import로 이번 작업과 무관

---

## 대기 중 — 결정 요청

### A. ✅ 완료 — 상단 "완료 작업 §6" 참조

### B. ✅ 완료 — 상단 "완료 작업 §7" 참조 (docs/·docs_preprocessor 연쇄 포함)

**유지 권장 (향후 Dokkaebi에서도 쓸 수 있음)**:
- `generate-licenses`, `generate-licenses.ps1`, `generate-licenses-csv`, `licenses/` (템플릿)
- `clippy`, `clippy.ps1`
- `bootstrap`, `bootstrap.ps1`
- `clear-target-dir-if-larger-than`, `clear-target-dir-if-larger-than.ps1`
- `check-links`, `check-keymaps`, `check-todos`
- `lib/`, `prettier`, `shellcheck-scripts`
- `cargo`, `cargo-timing-info.js`
- `new-crate`, `crate-dep-graph`
- `update-json-schemas`, `import-themes`
- `get-crate-version`, `get-crate-version.ps1`
- `download-wasi-sdk`
- `histogram`, `analyze_highlights.py` — Zed 개발 도구, 영향 평가 보류
- `install-rustup.ps1` — 개발 환경 셋업
- `prompts/` — Zed prompts 데이터

**결정 필요**: dead 8개 일괄 삭제 진행할지?

### C. ✅ 완료 (옵션 3) — 상단 "완료 작업 §8" 참조

### D. ✅ 완료 (옵션 a) — 상단 "완료 작업 §9" 참조

### E. ✅ 완료 — 상단 "완료 작업 §11" 참조

---

## 다음 세션에서 이어받기

### 빠른 재개 가이드
1. 이 파일(`plan.md`)의 "대기 중" 섹션 참조
2. 3개 결정 사항(A, B, C) 사용자에게 선택 요청
3. 결정된 내용 실행 → 빌드 검증 → `notes.md` 갱신
4. 필요 시 메모리 `project_license_cleanup.md` 업데이트

### 관련 참고 파일
- `NOTICE` — 현재까지의 변경 범위 요약이 있음 (다음 작업 시 업데이트 필요)
- `notes.md` — 작업 일자별 상세 기록
- 이전 대화의 `plan.md` 히스토리: 옵션 D → 옵션 4 → 현재 최종 정리

### 검증 명령
- 빌드: `cargo build -p Dokkaebi -p xtask`
- Zed URL grep: `grep -rn "zed\.dev\|zed-industries" crates/zed/src/`
- 남은 script: `ls script/`
