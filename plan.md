# Zed v0.233.5 + v0.233.6 Dokkaebi 백포트 — 계획

> **현재 단계**: 계획 작성, 사용자 승인 대기.
> **작성일**: 2026-04-23
> **상류 기준**: `zed-industries/zed` v0.233.5 (stable) + v0.233.6 (hotfix, 현재 Latest)
> **Dokkaebi 기준**: v0.4.0 (최근 백포트 기준선 v0.232.2)

---

## 1. 목표·범위
- Zed v0.233.5 stable + v0.233.6 hotfix의 공개 PR을 Dokkaebi에 선별 백포트.
- v0.232.3 stable(단일 PR #54105)은 v0.233.5의 상위 fix #54106에 포함되므로 단독 이식 불필요.
- PR 단위 순차 이식 → `cargo check -p <crate>` → 다음 PR 원칙 준수 (CLAUDE.md 백포트 절차 6단계).

## 2. 방침 결정 사항 (2026-04-23, 사용자 확정)
| 항목 | 결정 | 비고 |
|---|---|---|
| #53521 Fix with Assistant 제거 | **B. 상류대로 제거** | inline_assistant.rs에서 `AssistantCodeActionProvider` 삭제. `assistant: inline assist` 액션은 유지 |
| #53663 `cli_default_open_behavior` | **C. 전체 skip** | Dokkaebi CLI 경로가 Claude Code 알림 브리지 전용으로 재정의되어 상류 `zed <path>` UX 개선 무관. dialoguer 의존성 회피 |
| #53941 새 worktree UX (37f) | **적용 (승인)** | 단독 Phase로 분리 진행 |
| #54399 / #54125 기본 모델 자동 선택 | **제외** | Dokkaebi sign-in dead 영역과 cloud.rs 간섭 회피 |
| #53951 remote cross-compile | **완전 제외** | Dokkaebi dev container cross-compile 비대상 |

## 3. 자동 제외 (정책 + 방침 합계 9건)
- **Linux 전용**: #40096 (X11 window icon), #53582 (XInput 2.4)
- **dev_container 파일 부재**: #53557 (metadata JSON), #53538 (multi-stage dockerfile)
- **방침 결정 제외**: #53663, #54399, #54125, #53951
- **메타 커밋 중복**: #54439 (Cherry picks for v0.233.x — 개별 PR 릴리즈 노트에 포함되므로 별도 이식 불요)

---

## 4. Phase 1 — 긴급 (Windows 특화·심각 버그, 3건) — ✅ 완료 (2026-04-23)
- [x] **#53904** `gpui_tokio` Tokio runtime shutdown → WASI panic on exit (Windows)
  - 파일: `crates/gpui_tokio/src/gpui_tokio.rs`
  - `RuntimeHolder` enum → `Option<Runtime>` + `Drop` impl의 `shutdown_background()` 호출 구조
- [x] **#54445** Windows git state stale
  - 파일: `crates/worktree/src/worktree.rs`
  - `is_dot_git_changed` 조건 블록 제거 (Dokkaebi 쪽 독자 구현도 함께 제거)
- [x] **#54561** git worktree error 중복 borrow 크래시 (v0.233.6 hotfix)
  - 파일: `crates/git_ui/src/git_panel.rs`
  - `show_error_toast` 본문을 `cx.defer()` 로 래핑. Dokkaebi의 `ToastIcon::new(...)` 독자 아이콘 스타일 보존

**검증**: `cargo check -p gpui_tokio` / `-p worktree` / `-p git_ui` / `-p Dokkaebi` 모두 통과, 신규 경고·에러 0건.

---

## 5. Phase 2 — 기능 핵심 (2건) — ✅ 완료 (2026-04-23)
- [x] **#54190 + #54557** Claude Opus 4.7 BYOK
  - 적용: `crates/anthropic/src/anthropic.rs`, `crates/bedrock/src/models.rs`, `crates/language_models/src/provider/anthropic.rs`, `crates/opencode/src/opencode.rs`
  - skip: `language_models_cloud/language_models_cloud.rs` (Dokkaebi 파일 부재)
  - v0.233.6 hotfix #54557 반영: bedrock request_id 최종 `anthropic.claude-opus-4-7`
- [x] **#53575** stale remote connection modal dismiss — **대상 UX 부재로 skip**
  - `crates/agent_ui/src/agent_panel.rs::find_or_create_workspace` 경로 부재
  - `crates/sidebar` 크레이트 자체가 Dokkaebi에 없음
  - `dismiss_connection_modal` 공개 함수 추가해도 호출처 0건 (dead code)이라 함께 skip
  - 사용자 체감 regression 없음 — Dokkaebi agent panel에서 remote project 생성 UX 자체가 부재

**검증**: `cargo check -p anthropic -p bedrock -p language_models -p opencode` 통과, 최종 `cargo check -p Dokkaebi` 통과, 신규 경고·에러 0건.

---

## 6. Phase 3 — 소규모 bug fix 묶음 (약 30건)
파일 수 1~4 범위 fix를 카테고리별 커밋으로 묶음.

### 6-1. Markdown Preview — 부분 완료 (2026-04-23)
- [~] #53184 헤딩 앵커 링크 (5f) — **보류**: Dokkaebi `ParsedMarkdownData`에 `heading_slugs` 필드 부재(인프라 선행 백포트 필요)
- [~] #53086 각주 지원 (2f) — **보류**: 동일 patch가 `heading_slugs`·`mermaid_diagrams` 필드를 전제로 구조체 리터럴 구성. 인프라 선행 백포트 필요
- [x] #50934 underline/strikethrough 스케일 아티팩트 (1f) — gpui `paint_line` 3곳 wrap boundary 가드

### 6-2. Editor 소규모 — ✅ 완료 (2026-04-23)
- [x] #53979 ctrl-right fold·@mention skip (1f) — `movement.rs` 2곳 변환 체인 교체
- [x] #53185 breadcrumb syntax 테마 갱신 (2f) — `editor.rs::theme_changed` 에 `refresh_outline_symbols_at_cursor` 추가
- [x] #52539 renamed file stale session path (2f) — `EditorEvent::FileHandleChanged` 신설 + `should_serialize` 반영
- [x] #53279 tab switcher last tab 유지 (2f) — `workspace.rs::remove_pane` focus fallback 에 modal 분기
- [x] #52611 regex 검색 occurrence highlight (3f) — `last_selection_from_search` + `SelectionEffects::from_search` + BufferSearchHighlights 검사 early return
- [x] #53712 multibuffer semantic token (3f) — `needs_initial_data_update` + `update_data_on_scroll(debounce)` 분기 + `do_update_data_on_scroll` 분리
- [x] #53146 file duplicate rename stem 선택 (2f) — `disambiguation_range` 시작 인덱스 0으로 변경
- [x] #53484 `-` 시작 path subcommand `--` 분리 (4f) — git blame/update-index/stash/status args 에 `--` 추가 (Dokkaebi 파일 부재인 `docker.rs`·macOS 전용 `gpui_macos/platform.rs` 는 정책 제외)

### 6-3. UI / Workspace 소규모 — 부분 완료 (2026-04-23)
- [x] #53916 settings search UX (1f) — `apply_match_indices` + `open_best_matching_nav_page` + `scroll_content_to_best_match`
- [x] #52970 비포커스 창 30fps throttle (1f) — `min_frame_interval` 분기 재구조화
- [~] #53552 update indicator title bar (1f) — **이미 달성**: Dokkaebi `GithubUpdater` 가 Downloading 상태 항상 렌더
- [x] #53662 deleted folders recent 1주일 grace (1f) — 7일 유예 조건 호출처 이전
- [x] #54056 welcome tab project path (1f) — `project_name` 헬퍼 + i18n `welcome.remote_project` 고아 키 제거
- [~] #53808 BGRA8 WGPU panic (3f) — **보류**: +190/-24 renderer 대규모 재구조화, 실사용자 영향 제한적, 별도 Phase로
- [~] #53998 flexible dock widths (3f) — **보류**: `width_fraction_for_pane` → `full_height_column_count` 전면 교체, Dokkaebi 독자 호출처 존재, 별도 작업
- [~] #53915 CLI activate window 타이밍 (2f) — **skip**: #53663 skip과 연쇄 + Dokkaebi CLI 용도상 자동 창 활성화 부작용

### 6-4. Agent / ACP 소규모 — 부분 완료 (2026-04-23)
- [x] #53216 opencode ACP 반복 prompt (1f)
- [~] #53791 full branch name in picker (1f) — **skip**: `thread_branch_picker.rs` Dokkaebi 부재
- [x] #53859 open_thread duplicate sessions (1f)
- [~] #53657 zoomed agent panel scroll (1f) — **skip**: `max_content_width` Dokkaebi 부재, 해당 zoom 이슈 경로 없음
- [x] #52975 anthropic custom 모델 thinking 보존 (1f)
- [~] #54431 ACP replay events drop (1f) — **보류**: Dokkaebi 이미 사전 등록 패턴 적용(main 이슈 해결), pending_sessions 인프라는 별도 대규모 작업
- [x] #54116 focused tool call 보존 (2f)
- [x] #54134 thread title 실패 표시 (4f 중 sidebar.rs 제외 3f)
- [x] #54138 ACP 프로세스 종료 double borrow (6f 중 핵심 fix만, test_support 모듈 skip)
- [~] #53884 action_log commit race (4f) — **보류**: `buffer_diff` diverge로 `SetSnapshotResult` 구조 변경 단순 이식 불가
- [x] #53696 agent panel UI fix (4f 중 핵심 3건: Panel::min_size + 큐 메시지 포커스 + MIN_PANEL_WIDTH, max_content_width 변경 skip)

### 6-5. AI 모델 — ✅ 완료 (2026-04-23)
- [x] #53543 Ollama 컨텍스트 길이 (1f) — ModelShow Deserialize 에 parameters num_ctx 우선 파싱
- [~] #54106 Copilot reasoning effort (1f) — **이미 적용됨** (v0.232.2 백포트 시 상위 fix)
- [~] #54191 Google cloud model RefCell panic (2f) — **skip**: Dokkaebi cloud.rs 에 `CloudLlmTokenProvider`·`to_async()` 경로 부재

### 6-6. Git — 부분 완료 (2026-04-23)
- [x] #52965 git panel 트리 Enter 토글 (1f)
- [~] #53803 Git Graph 디자인 (2f) — **보류**: Dokkaebi `get_selected_repository` 경로·색상 스타일 diverge
- [x] #53929 diff hunk staging race (2f)
- [~] #53669 worktree naming regression (2f) — **skip**: Dokkaebi `resolve_worktree_branch_target` 부재(구조 재편)
- [x] #52996 bare repo recent projects (3f) — `original_repo_path_from_common_dir` 반환 타입 Option 변경
- [x] #53444 ANSI escape strip (3f) — alacritty_terminal vte ansi 의존성 추가 + GitOutputHandler

### 6-7. Languages / 기타 — 부분 완료 (2026-04-23)
- [x] #53546 TopoJSON 하이라이팅 (1f) — json/config.toml path_suffixes 에 topojson 추가
- [~] #54201 tsgo LSP fix (2f) — **보류**: Dokkaebi lsp-types rev diverge

**검증**: 각 카테고리 커밋 단위로 `cargo check -p <crate>` + Phase 끝에서 `cargo check -p Dokkaebi`.

---

## 7. Phase 4 — 중규모 기능 (8건) — 부분 완료 (2026-04-23)
- [x] #53452 mouse_wheel_zoom 설정 (2f) — Ctrl+스크롤 폰트 크기 조정 (`event.modifiers.secondary()` 분기)
- [x] #53504 hover_popover_sticky / hover_popover_hiding_delay (9f) — 4파일만 이식(settings_ui skip), core fix 완료
- [~] #53710 `workspace: format and save` 액션 (11f) — **보류**: trait 메서드 추가 Dokkaebi 11파일 diverge 가능성
- [~] #54316 `limit_content_width` 설정 (8f) — **skip**: Dokkaebi `max_content_width` 부재
- [~] #54318 favorite 모델 thinking/effort/fast 저장 (6f) — **보류**: Dokkaebi agent 영역 diverge, 구조 변경 많음
- [~] #48752 toggle block comment (22f) — **보류**: vim diverge + grammars 7종 대규모
- [x] #54256 Netpbm (PNM) 이미지 프리뷰 (5f) — `ImageFormat::Pnm` variant 추가(gpui + project + agent_ui)
- [~] #54224 unsaved scratch buffer 세션 유지 (9f) — **보류**: sidebar 크레이트 부재 + workspace 구조 변경 대규모

**검증**: `cargo check -p Dokkaebi` 통과, 신규 경고 0건.

---

## 8. Phase 5 — 구조 변경 / 방침 결정 항목 (4건) — 부분 완료 (2026-04-23)
- [x] **#53521** Fix with Assistant 제거 (방침 B)
  - `AssistantCodeActionProvider` struct + impl, `register_workspace_item`, `ItemAdded` 분기, 관련 import 삭제
- [~] **#53941** 새 worktree UX 개편 (37f) — **보류**: 신규 파일 2개(`thread_worktree_archive.rs`, `thread_worktree_picker.rs`) + Dokkaebi worktree 구조 diverge, Phase 5 단독 범위 초과
- [x] **#53560** ACP npm `--prefix` 시그니처 변경 (2f)
  - `npm_command` 에 `prefix_dir` 파라미터 추가, `SystemNodeRuntime::global_node_modules` 필드 제거
  - agent_server_store.rs 호출처 `None` prefix 로 migrate
- [~] **#48003** HTTP `context_servers` deprecated `settings` 필드 제거 (Breaking, 3f) — **보류**: Dokkaebi migrator 가 `m_2026_04_*` 계열 선행 백포트 지연 상태라 단독 추가 위험

**검증**: `cargo check -p Dokkaebi` 통과(37.60s, 신규 경고/에러 0건).

---

## 9. 검증 방법
1. PR 단위: 이식 직후 해당 crate `cargo check`.
2. Phase 단위: 해당 Phase 종료 시 `cargo check -p Dokkaebi` — 경고/에러 0건.
3. 전체 완료 후:
   - `cargo check -p Dokkaebi` 최종
   - Dokkaebi 런타임 스모크 테스트 (실행 → 워크스페이스 열기 → Claude Code 알림 트리거 → git panel 조작)
4. 각 Phase 완료 시 `notes.md` 적용·미적용 내역 기록, `release_notes.md` 사용자 체감 변경 반영.

## 10. 승인 필요 사항 (코드 작업 착수 전 확인)
- [x] **#53521 Fix with Assistant 제거** — 2026-04-23 사용자 B 결정
- [x] **#53941 worktree UX (37f)** — 2026-04-23 사용자 승인
- [x] **#53663 skip** — 2026-04-23 사용자 C 결정
- [x] **#54399 / #54125 / #53951 제외** — 2026-04-23 사용자 결정
- [x] **#53560 npm_command 시그니처 변경** — 2026-04-23 사용자 승인
- [x] **#48003 HTTP context_servers Breaking change** — 2026-04-23 사용자 "제거" 결정 (상류대로 deprecated `settings` 필드 제거, `release_notes.md` 에 breaking notice 기재)
- [x] 의존성 추가: **없음** (dialoguer는 #53663 skip으로 회피)
- [ ] Phase 1 착수 승인 대기

## 11. 진행 방식 (작업 분할)
백포트 규모가 크므로 한 번에 모든 Phase를 끝내지 않고 **Phase별 착수 승인**을 받아 단계 진행:
- 1차 세션 Phase 1 완료 후 보고 → 승인 → Phase 2
- 이하 동일하게 Phase 단위로 진행

긴 세션 중단·재개 시 이 `plan.md` 체크박스가 진행 상태 기준점.

## 11.5. Phase 6 — 보류 항목 재착수 (3건) — 부분 완료 (2026-04-23)
- [x] **#53710 workspace: format and save 액션** (11f)
  - `SaveOptions.force_format: bool` + `SaveIntent::FormatAndSave` + `Workspace` action `FormatAndSave` 신설
  - `editor::items.rs` 에 `format_trigger = if force_format { Manual } else { Save }` 분기
  - 기존 `SaveOptions { format, autosave }` 호출처 전부에 `force_format: false` 추가
- [~] **#54224 unsaved scratch buffer 세션 유지** (9f) — **보류 유지**: `workspace/persistence.rs` +310 대규모 DB schema 변경, Dokkaebi 독자 schema 와 충돌 위험
- [x] **#48752 toggle block comment** (22f 중 core 10f 적용)
  - `editor::actions.rs` 에 `ToggleBlockComments` action 추가
  - `editor::editor.rs::toggle_block_comments` 함수 신설(+193 라인): block_comment markers 가 선택을 감싸거나 선택이 감싸는 경우 둘 다 처리, 공백 패딩 제거, insert 시 prefix/suffix 자동 공백 추가
  - `editor::element.rs` 에 register_action 1줄 추가
  - Windows keymap 에 `ctrl-k ctrl-/` + `shift-alt-a` 바인딩 추가 (macOS/Linux keymap 은 정책상 skip)
  - grammars 7종(c/cpp/go/javascript/jsonc/markdown/python/rust/tsx)의 config.toml 에 `block_comment` 필드 추가 또는 `tab_size` 보정
  - vim 변경(+254) 및 editor_block_comment_tests.rs(+293) 은 **skip**: vim 모드 전용 + 테스트 케이스는 Dokkaebi test util 호환 확인 부담. 필요 시 별도 작업

**검증**: `cargo check -p Dokkaebi` 통과(26.46s, 신규 경고/에러 0건).

## 11.6. Phase 7 — 남은 보류 항목 전체 이식 시도 (1건 추가 적용 / 나머지 확정 보류)
사용자 "전체적으로 적용해" 지시로 Phase 6 이후 남은 보류 항목 전수 재검토.
- [x] **#53803 Git Graph 디자인** (2f) — Dokkaebi `get_selected_repository` 경로 유지하면서 `is_head_ref` static 헬퍼 + `render_chip` 시그너처 확장(is_head 파라미터 + chip.icon + 배경 opacity 분기) + `render_table_rows` 시작에 `head_branch_name: Option<SharedString>` 사전 계산(repository.snapshot().branch.name()) + ref_names iter 에서 is_head 계산해 render_chip 호출. 커밋 상세 뷰(select_entry_idx 기반) 도 동일 적용.
- [~] **#53998 flexible dock widths** (3f) — **확정 보류**: Dokkaebi `workspace.rs` 에 상류 `dock_size`/`dock_flex` + `opposite_dock_panel_and_size_state` 인프라 자체가 **부재**. Dokkaebi 의 flex dock 계산은 `default_flexible_dock_ratio` 함수 하나로 간소화된 독자 구조라 상류 patch 의 수정 대상이 존재하지 않음. 전체 flex dock 영역 재이식 필요.
- [~] **#53669 worktree naming regression** — 확정 보류 (Phase 3 §6-6 이후 변화 없음, `resolve_worktree_branch_target` 부재)
- [~] **#54201 tsgo LSP** — 확정 보류 (lsp-types rev Dokkaebi `a4f41...` 독자 유지, 상류 `f4dfa89...` 로 rev 변경 시 다른 영역 회귀 리스크)
- [~] **#54318 favorite 모델 thinking/effort/fast** — 확정 보류 (`LanguageModelSelection::speed: Option<Speed>` 필드 선행 PR 백포트 필요)
- [~] **#54224 unsaved scratch buffer 세션 유지** — 확정 보류 (`workspace/persistence.rs` +310 DB schema 대규모 변경)
- [~] **#48003 HTTP context_servers deprecated 제거** — 확정 보류 (migrator `m_2026_03_30`/`m_2026_04_01`/`m_2026_04_10`/`m_2026_04_15`/`m_2026_04_17` 5건 선행 + 각 설정 구조 동기화)
- [~] **#53086 마크다운 각주 지원** — 확정 보류 (`ParsedMarkdownData::heading_slugs`·`mermaid_diagrams` 인프라 선행 필요)
- [~] **#53184 마크다운 헤딩 앵커 링크** — 확정 보류 (동일 인프라 선행)
- [~] **#53808 WGPU BGRA8 panic** — 확정 보류 (renderer 3파일 +190 대규모 + BGRA8 미지원 Windows GPU 실사용자 영향 제한적)
- [~] **#53941 새 worktree UX 개편** (37f, 신규 파일 2개) — 확정 보류 (사실상 신규 기능 재이식 수준, 별도 단독 Phase 필요)
- [~] **#54431 ACP replay events drop** — 확정 보류 (`pending_sessions` + `AcpSession.ref_count` 동시성 인프라 선행 필요. Dokkaebi 이미 sessions 사전 등록 패턴 적용으로 main 이슈는 해결된 상태)
- [~] **#53884 action_log commit race** — 확정 보류 (`buffer_diff::BufferDiffEvent::BaseTextChanged` variant + `set_snapshot_with_secondary_inner` 반환 타입 `DiffChanged` → `SetSnapshotResult` 구조 변경 선행, Dokkaebi 3곳에서 독자 DiffChanged emit)

**Phase 7 검증**: `cargo check -p Dokkaebi` 통과(3.45s, 신규 경고/에러 0건).

## 11.7. Phase 8 — 전체 보류 항목 적용 계획 (선행 인프라 역추적 + 본 PR 이식)

> **배경**: 사용자 결정 — "업데이트가 쌓이면 미백포트 부분 때문에 이식 불가능해지는 문제 방지" 위해 현재 시점에 **보류 12건 전부 적용**. 단 Dokkaebi 가 의도적으로 삭제한 영역과 관련된 부분은 제외.
> **현재 단계**: 계획 작성, 사용자 승인 대기.

### 11.7.1. Dokkaebi 제거 영역 (이식 대상 **제외** 기준)
CLAUDE.md "Dokkaebi 재구성으로 적용 불가한 상류 변경" 규정 + 본 작업 추가 확인 사항:
- **REPL/Jupyter**: `crates/repl` 제거 → Jupyter·REPL·ipykernel·notebook 관련 상류 PR 전부 제외
- **Collab (RTC/채널/온라인)**: `server_url=""` cloud 비활성 → `crates/collab*`·협업·채널·Sign-in UI·collab 테스트 제외
- **Dev Container (일부)**: `docker.rs`·`devcontainer_json.rs`·`devcontainer_manifest.rs` 부재 → 해당 파일 변경 제외 (필요 시 `devcontainer_api.rs` 로 발췌 이식)
- **language_model_core 크레이트 제거**: 관련 제외
- **language_models_cloud 크레이트 제거**: 관련 제외
- **macOS/Linux 전용 키맵·파일**: `default-macos.json`, `default-linux.json`, `#[cfg(target_os = "macos|linux")]` 분기 제외
- **Notification Panel 크레이트 제거**: 해당 PR 전체 제외
- **Dev 채널 single-instance skip 제거 (Dokkaebi 독자)**: 유지, 상류가 관련 변경을 해도 Dokkaebi 수정분 보존

### 11.7.2. 보류 12건 → 선행 인프라 역추적 결과

#### Group A: ACP 인프라 (#52997 "acp: Use new Rust SDK" 선행)
대상:
- **#54431 ACP replay events drop** — `pending_sessions` + `AcpSession.ref_count` 인프라 필요
- **#54138 (Phase 3 §6-4 완료)** 의 추가 test_support 모듈 (skip 된 부분)
- 기타 ACP 관련 후속 fix 들

선행 PR: **#52997** (2026-04-22, `acp: Use new Rust SDK`). Dokkaebi v0.232.2 기준 미적용 확정. ACP 클라이언트·서버·연결·세션 상태 전반을 새 Rust SDK 로 교체하는 대규모 리팩터링. **추가 조사 필요**: #52997 의 세부 파일 변경 규모, 이전 SDK 와의 비호환성, Dokkaebi acp_thread 구조와의 차이.

#### Group B: Favorite 모델 (#53356 "Persist fast mode across new threads" 선행)
대상:
- **#54318 favorite 모델 thinking/effort/fast 저장**

선행 PR: **#53356** (2026-04-08, `Persist fast mode across new threads`). `LanguageModelSelection::speed: Option<Speed>` 필드 도입 + fast mode 지속 로직. Dokkaebi 에 필드 자체가 부재 확정. **추가 조사 필요**: Speed enum 도입 여부, 관련 language_model trait 메서드 (`supports_fast_mode`), fast mode UI 토글.

#### Group C: Markdown 인프라 (#52008 "Refactor to use shared markdown crate" 선행)
대상:
- **#53086 마크다운 각주 지원**
- **#53184 마크다운 헤딩 앵커** (이미 보류 해제 가능. heading_slugs 는 이 PR 이 도입. 단 mermaid_diagrams 선행 필요)

선행 PR: **#52008** (2026-03-26, `markdown_preview: Refactor to use shared markdown crate`). `mermaid_diagrams: BTreeMap<usize, ParsedMarkdownMermaidDiagram>` 필드 도입 + `extract_mermaid_diagrams` 함수 + shared markdown crate 재구성. Dokkaebi `crates/markdown/src/mermaid.rs` 파일은 존재하지만 `ParsedMarkdownData` 구조체 통합 미완. **추가 조사 필요**: #52008 실제 변경 파일, Dokkaebi markdown_preview 크레이트와의 차이.

#### Group D: Workspace flex dock (선행 PR 역추적 중)
대상:
- **#53998 flexible dock widths**

선행 필요: `workspace::dock_flex` + `opposite_dock_panel_and_size_state` 함수 도입 PR. Dokkaebi `default_flexible_dock_ratio` 함수와 상류 구조 차이. **추가 조사 필요**: 상류 workspace flex dock 도입 PR (`opposite_dock_panel_and_size_state` 로 blame 역추적). Dokkaebi 간소화 버전에서 상류 전체 flex 인프라로 확장.

#### Group E: Migrator + 설정 구조 (migrator 5건 + 관련 PR 선행)
대상:
- **#48003 HTTP context_servers deprecated `settings` 제거**

선행 migrator (각각 대응 PR 역추적 필요):
1. **m_2026_03_30** `make_play_sound_when_agent_done_an_enum` — `play_sound_when_agent_done: Option<bool>` → enum 전환, Dokkaebi 설정 구조 동기화
2. **m_2026_04_01** `restructure_profiles_with_settings_key` — 프로필 구조 변경
3. **m_2026_04_10** `rename_web_search_to_search_web` — 도구 리네임
4. **m_2026_04_15** `remove_settings_from_http_context_servers` — #48003 본체
5. **m_2026_04_17** (이름 조사 필요)

**추가 조사 필요**: 각 migration 의 대응 상류 PR, 설정 구조 변경 범위.

#### Group F: lsp-types rev 업데이트
대상:
- **#54201 tsgo LSP fix**

Dokkaebi `lsp-types` rev `a4f41...` 독자 유지. 상류 rev `f4dfa89...`. **추가 조사 필요**: Dokkaebi 독자 rev 가 fork 인지, 상류의 이전 버전인지. fork 가 아니라면 rev 업데이트 가능. 상류와 Dokkaebi rev 간 변경 사항 조사.

#### Group G: buffer_diff BaseTextChanged
대상:
- **#53884 action_log commit race**

선행 필요: `BufferDiffEvent::BaseTextChanged` variant + `SetSnapshotResult { change, base_text_changed }` 구조. #53884 자체가 도입 PR 이지만 Dokkaebi `buffer_diff` 가 3곳에서 독자 `DiffChanged` emit 하는 경로 재작업 필요. **Dokkaebi diverge 재작업 범위 조사** 필요.

#### Group H: Workspace persistence scratch
대상:
- **#54224 unsaved scratch buffer 세션 유지**

Dokkaebi v0.4.0 워크스페이스 그룹·서브에이전트 탭 persistence 독자 수정(`workspace_group_panel` 등)과 `workspace/src/persistence.rs` +310 충돌 검토. **추가 조사 필요**: 상류 patch 가 건드리는 DB schema 영역과 Dokkaebi 독자 영역 비교. 동시 적용 가능 여부.

#### Group I: 새 Worktree UX
대상:
- **#53941 새 worktree UX 개편**
- **#53669 worktree naming regression** (#53941 의 일부)

신규 파일 2개(`thread_worktree_archive.rs`, `thread_worktree_picker.rs`) + 신규 아이콘 6개 + 기존 37f 수정 (collab 테스트 제외). **추가 조사 필요**: 선행 PR 의존성(agent_ui worktree 관련 이전 PR), 아이콘 SVG 추가, 키맵 바인딩.

#### Group J: WGPU 다중 포맷
대상:
- **#53808 WGPU BGRA8 panic**

`crates/gpui_wgpu/src/{wgpu_atlas.rs, wgpu_context.rs, wgpu_renderer.rs}` 3파일 +190. **선행 없음** (renderer 단독 확장). 하지만 `color_texture_format` 필드·`swizzle_upload_data`·`from_context` 생성자 신설로 구조 변경 큼.

### 11.7.3. Phase 8 Sub-Phase 진행 순서 (의존성 기반)

| 순서 | Sub-Phase | 선행 PR | 본 PR | 예상 규모 |
|---|---|---|---|---|
| 1 | 8A ACP SDK | #52997 | #54431 | **초대형** (ACP 전체 재편) |
| 2 | 8B Favorite 모델 | #53356 | #54318 | 중 |
| 3 | 8C Markdown 인프라 | #52008 | #53184, #53086 | 대 |
| 4 | 8D Flex Dock | (역추적) | #53998 | 대 |
| 5 | 8E Migrator + 설정 | m_2026_03_30/04_01/04_10/04_17 대응 PR | #48003 | 대 |
| 6 | 8F lsp-types rev | (조사) | #54201 | 중 |
| 7 | 8G buffer_diff BaseText | 없음 | #53884 | 중 |
| 8 | 8H Persistence Scratch | 없음 (diverge 검토) | #54224 | 중 |
| 9 | 8I Worktree UX | (선행 역추적) | #53941, #53669 | **초대형** |
| 10 | 8J WGPU 포맷 | 없음 | #53808 | 중 |

각 Sub-Phase 는 다음 단계로 진행:
1. **조사**: 선행 PR 의 merge SHA·파일 목록·Dokkaebi 파일 존재 확인·Dokkaebi diverge 범위 체크
2. **선행 인프라 백포트**: 각 선행 PR 의 patch 를 Dokkaebi 에 이식, Dokkaebi 제거 영역 관련 부분 skip
3. **본 PR 이식**: 선행 완료 상태에서 본 PR 이식
4. **검증**: `cargo check -p <crate>` → 최종 `cargo check -p Dokkaebi`
5. **문서 갱신**: `notes.md`·`release_notes.md` 해당 Sub-Phase 기록

### 11.7.4. 예상 작업 규모
- **선행 인프라 PR**: 확인된 3개(#52997, #53356, #52008) + 역추적 필요 최소 10개 이상
- **본 보류 PR**: 12건
- **Dokkaebi 제거 영역 skip 분**: 각 Sub-Phase 별 ~20~30% 분량
- **총 수정 파일**: 수백 파일 (대규모 PR 인 #52997, #53941 이 각각 수십~수백 파일)
- **총 세션**: 최소 5~10 세션 분량 (현재 세션 크기 기준)

### 11.7.5. 승인 필요 사항
- [ ] **전체 Phase 8 진행 방침 승인** — 선행 인프라 포함 전체 적용
- [ ] **Sub-Phase 진행 방식** — 다음 중 선택:
  - **(1) 순차 승인**: 8A 조사·계획 → 승인 → 이식 → 검증 → 8B 조사 …
  - **(2) 일괄 자율**: 모든 Sub-Phase 를 이전 "계속 진행" 지시처럼 자율 연속 진행
- [ ] **Dokkaebi 제거 영역 skip 기준** — §11.7.1 확정 (추가 예외 있으면 사전 알림)
- [ ] **선행 PR 의 추가 선행 발견 시 처리 방침** — 재귀적 역추적 허용 여부
- [ ] **세션 범위 초과 시 처리** — 단일 Sub-Phase 가 한 세션 분량 초과 시 중단·계속 기준

### 11.7.6. 작업 흐름 내 위치
- 이 Phase 8 은 **v0.233.5 백포트의 확장**. Phase 1~7 에서 이식된 ~38건을 보존한 상태에서 보류 12건 + 각 선행 인프라 추가 이식.
- `notes.md` 는 Sub-Phase 단위로 상세 기록, `release_notes.md` 는 사용자 체감 기능 단위로 집계.
- 본 계획 진행 중 상류에서 v0.234.x·v0.233.6 이후 추가 릴리즈가 나오면 별도 백포트는 **Phase 8 완료 후** 처리.

---

## 11.8. Phase 8 실행 전략 — 브랜치 분리 (B 전략 확정)

> **사용자 결정 (2026-04-23)**: Phase 8 ACP SDK 이식은 단일 세션 완료 불가 규모(acp.rs +1102 재작성 등). 브랜치 분리로 중간 빌드 실패 상태 수용.

### 11.8.1. Sub-Phase 8A 사전 조사 완료 (현재 세션)
4개 에이전트 병렬 조사로 확정된 사항:
- **SDK 버전**: `agent-client-protocol` `=0.10.2` → `=0.11.1` 업그레이드 필수 (crates.io 2026-04-21 stable 존재)
- **선행 점검 통과**:
  - `claude_subagent_view` 에 `agent_client_protocol` 직접 참조 **0건** (SDK 업그레이드 직접 영향 없음)
  - Dokkaebi `Diff::finalized(path, old_text, new_text, language_registry, cx)` 시그너처와 상류 post-#52997 호출 **완전 일치** (`diff.rs` 보존 확정)
- **이식 규모**: 62파일 (대규모 1 + 중규모 1 + 소규모 56 + skip 5)
- **핵심 API 전환 5패턴**:
  1. `use agent_client_protocol as acp` → `use agent_client_protocol::schema as acp`
  2. `Rc<ClientSideConnection>` → `ConnectionTo<Agent>`
  3. `impl acp::Client for ClientDelegate` 삭제 → 빌더 `on_receive_request!()`/`on_receive_notification!()` 매크로 + foreground dispatch queue
  4. `FlattenAcpResult` trait 신설 (`Result<Result<T>, _>` → `Result<T, acp::Error>`)
  5. transport `async_pipe::pipe()` → `Channel::duplex()`
- **Dokkaebi 충돌 영역**:
  - `agent_ui/` 16파일: **충돌 위험 낮음** (#52997 patch 는 import 치환 중심, Phase 4 retry 버튼과 라인 겹침 없음)
  - `claude_subagent_view`: 직접 영향 없음
  - `acp_thread/diff.rs`: 보존 가능
  - Dokkaebi 독자 tool 5개 중 `spawn_agent_tool.rs` 만 `AgentTool` impl → 상류 15개와 동일 패턴 migrate 필요, 나머지 4개는 간접 영향
- **Dokkaebi 독자 수정**: `connection.rs` 아이콘 1줄, `mention.rs`/`acp_thread.rs` 이전 백포트 흔적. 매우 작음

### 11.8.2. Sub-Phase 8A 실행 플랜 (다음 세션)

#### Git 브랜치 전략
```
main (04b68ff196 v0.4.0 baseline + 현재 세션 64 미커밋 변경분)
  │
  ├─ phase-8-acp-sdk (새 브랜치)
  │    8A-1 Cargo + import 치환
  │    8A-2 acp_tools 확장
  │    8A-3 acp_thread 이식
  │    8A-4 agent_servers/acp.rs 재작성
  │    8A-5 #54431 본 PR 이식
  │    8A-6 검증 + merge
  │
  ↓ (merge 성공 시)
main (8A 완료 상태)
```

#### 실행 단계 (다음 세션 1회)
1. **전제**: 현재 64 미커밋 변경분을 main 에 commit (사용자 승인 필요) 또는 브랜치로 이동
2. **브랜치 생성**: `git checkout -b phase-8-acp-sdk`
3. **이식 착수**: 에이전트 5~6 병렬 실행
   - 에이전트 1: Cargo.toml 3건 업그레이드
   - 에이전트 2: `acp_thread/*.rs` 4파일 + 기타 50+ 파일 import 치환 (기계적)
   - 에이전트 3: `acp_tools/src/acp_tools.rs` +349 `StreamMessage` 내재화
   - 에이전트 4: `agent_servers/src/acp.rs` +1102/-576 SDK 전환 본체 재작성
   - 에이전트 5: `Dokkaebi diff.rs` 보존 + `spawn_agent_tool.rs` migrate + 기타 Dokkaebi 독자 정렬
   - 본 세션: 조정·검증
4. **중간 빌드 상태**: 각 에이전트 작업 중 빌드 실패 수용
5. **최종 검증**: `cargo check -p Dokkaebi` 성공까지 에러 순차 해결
6. **#54431 이식**: 8A-1~5 완료 후 본 PR 이식
7. **Merge**: main 으로 병합

### 11.8.3. 다음 세션 시작 프롬프트 (참고용)
```
Phase 8A ACP SDK 이식 착수. plan.md §11.8 기준.
1. phase-8-acp-sdk 브랜치 생성
2. 에이전트 5 병렬 실행
3. 각 단계 완료 후 cargo check
4. 최종 merge
```

### 11.8.4. 현재 세션 종료 상태 (2026-04-23)
- Dokkaebi 빌드 정상 (`cargo check -p Dokkaebi` 1.61s)
- Phase 7 이후 추가 이식 없음
- 64 파일 미커밋 변경 (Phase 1~7 + Phase 8 Git Graph #53803 + 문서)
- **다음 세션 시작 전 사용자 결정 필요**:
  - (a) 64 변경분 main commit 후 `phase-8-acp-sdk` 브랜치 생성
  - (b) 64 변경분을 `phase-8-acp-sdk` 브랜치로 이동 (main 은 baseline 유지)

## 11.9. Phase 9 — #53094 `git_graph: Refresh UI when stash/branch list has changed` 이식 (2026-04-23 착수)

### 11.9.1. 배경 및 결정
- **상류 PR**: #53094 (2026-04-06 머지, merge SHA `7748047`). 상류 최초 포함 stable = **v0.232.2**.
- **누락 사유**: v0.232.2 Dokkaebi 선별 백포트 시 검토 대상에 포함되지 않아 누락 (명시적 보류가 아닌 단순 미이식).
- **v0.233.7 검토 중 재발견**: v0.233.7 `#54575 git: Fix remote branch picker`(SSH remote 한정)가 #53094 인프라에 종속. Dokkaebi 로컬 1인 모드에서 #54575 가치는 낮으나, #53094는 로컬 git 체감 버그 수정.
- **사용자 결정 (2026-04-23)**: #53094 단독 이식. #54575는 별도 판단.

### 11.9.2. PR #53094 수정 내용 요약
상류 patch: 6 파일, +186 / -20.
1. **`RepositoryEvent::BranchChanged` → `HeadChanged` 리네임** — HEAD 포인터 변경 의미 명확화.
2. **`RepositoryEvent::BranchListChanged` 신규** — 브랜치 목록 변경 분리 이벤트.
3. **`RepositorySnapshot.branch_list: Arc<[Branch]>` 필드 신규** — 이전까지 브랜치 목록은 별도 보관 없이 `branches()` 호출마다 새로 가져옴.
4. **`compute_snapshot()` 재작성** — branches 를 `branch_list` 로 보관, `head_changed` / `branch_list_changed` 분리 감지 → 각각 이벤트 발행.
5. **`git_graph` cache 무효화 확장** — `HeadChanged | BranchListChanged` 수신 시 graph 재로드, `StashEntriesChanged` 는 `LogSource::All` 만 무효화.
6. **`Repository::initial_graph_data` 정리** — `StashEntriesChanged` 시 `LogSource::All` 키만 retain 제거(부분 정리).
7. **`FakeGitRepositoryState.stash_entries`** 추가 + `branches()` ref_name 정렬.

### 11.9.3. Dokkaebi 이식 대상 (9 지점)

**crates/project/src/git_store.rs** (6 지점)
| Dokkaebi 위치 | 변경 내용 |
|---|---|
| `pub struct RepositorySnapshot` (L279~297) | `pub branch_list: Arc<[Branch]>,` 추가 |
| `pub enum RepositoryEvent` (L429~436) | `BranchChanged` → `HeadChanged` 리네임 + `BranchListChanged` 신규 |
| `RepositorySnapshot::new()` 생성자 (L3574~) | `branch_list: Arc::from([])` 초기화 |
| `subscribe_self` match (L3942~3950) | `HeadChanged \| BranchListChanged` 공통 분기 + `StashEntriesChanged` retain 분기 추가 |
| `Repository::paths_changed` emit (L5494) | `BranchChanged` → `HeadChanged` |
| `apply_remote_update` emit (L6258) | `BranchChanged` → `HeadChanged` |
| `compute_snapshot` (L7178~7225) | `branch_list` 계산, `head_changed`/`branch_list_changed` 분리, 각 이벤트 발행 |

**crates/git_graph/src/git_graph.rs** (2 지점)
| Dokkaebi 위치 | 변경 내용 |
|---|---|
| Event handler (L1080~1088) | `BranchChanged` → `HeadChanged \| BranchListChanged` + `StashEntriesChanged if log_source == All` 분기 추가 |
| 테스트 assertion (L3605~3606) | `BranchChanged` → `HeadChanged` |

**crates/git_ui/src/git_panel.rs** (1 지점)
| 위치 | 변경 |
|---|---|
| L785 matching pattern | `BranchChanged` → `HeadChanged` |

**crates/project/src/git_store/branch_diff.rs** (1 지점)
| 위치 | 변경 |
|---|---|
| L73 matching pattern | `BranchChanged` → `HeadChanged` |

**crates/project/tests/integration/project_tests.rs** (1 지점)
| L11158 | `BranchChanged,` → `HeadChanged,` |

**crates/fs/src/fake_git_repo.rs** (3 지점)
| 위치 | 변경 |
|---|---|
| use 블록 (L5~17) | `stash::GitStash` import 추가 |
| `FakeGitRepositoryState` 구조체 (L39~57) | `stash_entries: GitStash` 필드 추가 (Dokkaebi 독자 `worktrees` 필드 뒤) |
| 생성자 (L59~78) | `stash_entries: Default::default()` 추가 |
| `stash_entries()` 메서드 (L382) | 상태 기반 구현으로 변경 |
| `branches()` 메서드 | 끝에서 ref_name 오름차순 정렬 추가 |

### 11.9.4. 리스크 및 충돌 지점
- **`compute_snapshot` Dokkaebi 독자 변경 없음 확인됨** — 상류 변경과 그대로 병합 가능.
- **`FakeGitRepositoryState` 에 Dokkaebi 독자 필드 `worktrees: Vec<Worktree>` 존재** — 상류 patch 는 `graph_commits` 뒤에 `stash_entries` 를 삽입하지만, Dokkaebi 는 `graph_commits` 뒤에 `worktrees` 가 이미 있음. `stash_entries` 는 `worktrees` 뒤로 배치.
- **`cargo check -p project -p git_graph -p git_ui`** 3 크레이트 영향. 빌드 성공 후 `-p Dokkaebi` 전체 검증.
- **신규 테스트 (`test_graph_data_reloaded_after_stash_change`)** 는 이식하지 않는다. 이유: Dokkaebi 는 상류 테스트 코드를 최소한으로만 이식(릴리즈 노트 범위 외) + test-only 추가는 로컬 이식 부담. `BranchChanged → HeadChanged` 문자열 리네임만 반영.

### 11.9.5. 이식 순서 (순차)
1. `RepositoryEvent` enum 및 `RepositorySnapshot` 필드 추가
2. `RepositorySnapshot::new()` 초기화
3. `subscribe_self` match 확장
4. 모든 `BranchChanged` emit → `HeadChanged`
5. `compute_snapshot` 분기 분리 및 `branch_list` 계산
6. `git_graph` event handler 및 assertion
7. `git_panel.rs`, `branch_diff.rs` matching pattern
8. `project_tests.rs` assertion
9. `fake_git_repo.rs` 구조/생성자/메서드
10. `cargo check -p project` → `-p git_graph` → `-p git_ui` → `-p Dokkaebi`
11. `notes.md` 최근 변경 추가
12. `release_notes.md` 에 "브랜치 목록·stash 변경 시 Git Graph UI 실시간 반영" 항목 추가

### 11.9.6. 검증 방법
- `cargo check -p project` → `-p git_graph` → `-p git_ui` → `-p Dokkaebi` 경고·에러 0 건 확인.
- 빌드 후 Dokkaebi 실행하여 다음 시나리오 수동 확인은 선택(사용자 요청 시):
  - 새 브랜치 생성 → git graph 에 즉시 반영
  - stash push/pop → git graph 에 즉시 반영
  - HEAD 이동 → git graph 재로드

### 11.9.7. 진행 체크리스트
- [x] 사용자 승인 (2026-04-23 "#53094 적용")
- [x] plan.md §11.9 작성
- [x] `RepositoryEvent` / `RepositorySnapshot` 구조 수정
- [x] `compute_snapshot` + emit 호출처 이식
- [x] `git_graph` / `git_panel` / `branch_diff` 호출처
- [x] `fake_git_repo.rs` stash_entries 추가
- [x] `cargo check -p project -p git_graph -p git_ui -p Dokkaebi` 성공 (각 3m08s / 1m32s / 37s)
- [x] `notes.md` 갱신
- [x] `release_notes.md` 갱신 (버그 수정 2 항목)

---

## 11.10. Phase 10 — #53941 `agent_ui: Improve the new thread worktree UX` 잔여 이식 (2026-04-23 착수)

### 11.10.1. 배경 및 결정
- **상류 PR**: #53941 (agent_ui: Improve the new thread worktree UX, +1645/-2990/37f, merge 6beecae, 2026-04-15). 상류 최초 포함 stable = v0.232.2.
- **Dokkaebi 선행 완료분** (Phase 8I 1~2차, 커밋 `acb729cf32`): git 인프라 11 파일 (`is_bare` 필드 왕복, `Worktree::branch_name()`, `worktree_picker.rs` `is_bare` 리터럴), 신규 파일 2개 소스 복사 (`thread_worktree_picker.rs` 1037 라인 / `thread_worktree_archive.rs` 1032 라인), 아이콘 6개, 키맵 1건, 타입 인프라 3종(`CreateWorktree` / `SwitchWorktree` / `NewWorktreeBranchTarget`) + `ToggleWorktreeSelector` action 등록.
- **사용자 결정 (2026-04-23)**: 옵션 A (agent_panel refactor) + 옵션 B (archive 인프라 + 활성화) 동시 적용. main 브랜치 직접 작업. multi_workspace 경로 연결은 Dokkaebi 독자 `AgentV2FeatureFlag` 구조 차이로 skip.
- **이전 오판 정정**: 이전 분석에서 "agent_panel.rs 는 Dokkaebi 독자 서브에이전트 뷰 탭/워크스페이스 그룹 색상 보존 조건과 겹침" 주장은 **근거 없음** (전수 grep 0건 확인). 순수 refactor 작업이며 Dokkaebi 독자 기능과 무관.

### 11.10.2. PR #53941 사용자 체감 변경 (Release Notes 인용)
1. **Agent**: "Improved and simplified the UX of creating threads in Git worktrees."
2. **Git**: "Fixed a bug where worktrees in a detached HEAD state wouldn't show up in the worktree picker."
   - 2번은 Phase 8I 1차 `is_bare` 이식으로 이미 동등 달성 (검증 시 별도 확인).

### 11.10.3. 이식 범위 3 Part

#### Part A — agent_panel.rs `StartThreadIn` 제거 refactor (+602/-1017)
- **대상 파일**:
  - `crates/agent_ui/src/agent_panel.rs` 37 참조 (L30, 33, 140, 410, 417, 606, 751, 883, 884, 1084, 2244, 2250, 2254, 2259~2299, 2545, 3641, 3648, 3649, 3678, 3702, 4956, 5891~6230)
  - `crates/agent_ui/src/agent_ui.rs` — `mod thread_worktree_picker` 활성화, `StartThreadIn` enum 제거, action 리네이밍 최종화
  - `crates/agent_ui/src/conversation_view/thread_view.rs` -45 — `FirstSendRequested` 이벤트 경로 제거 (`StartThreadIn::NewWorktree` 지연 worktree 생성 메커니즘과 짝)
  - `crates/recent_projects/src/recent_projects.rs` +2 — `find_or_create_local_workspace` 호출 경로 변경
- **변환 규칙**:
  - `StartThreadIn::LocalProject` → `CreateWorktree { branch_target: CurrentBranch }` 또는 `SwitchWorktree` 로 맥락별 분기
  - `StartThreadIn::NewWorktree` → `CreateWorktree { branch_target: NewBranch }` 기반
  - `CycleStartThreadIn` action → `ToggleWorktreeSelector` action (이미 agent_ui.rs 에 등록됨)
  - eager worktree 생성: "first send 시 생성" 경로 → "선택 시 즉시 생성" 경로로 재배선
- **Dokkaebi 독자 기능 보존 검증** (재확인 완료):
  - `agent_panel.rs` 내 `subagent` / `서브에이전트` / `workspace_group` / `group_color` grep **0건** → 충돌 없음
  - Dokkaebi 서브에이전트 뷰 탭 관련 코드는 `conversation_view/thread_view.rs`, `thread_metadata_store.rs`, `entry_view_state.rs`, `conversation_view.rs` 분포. thread_view.rs 의 -45 는 **FirstSendRequested 이벤트 경로**만 제거하고 서브에이전트 뷰 탭 코드는 별도 영역이라 병렬 공존 가능
  - Dokkaebi `AgentV2FeatureFlag` 기반 `start_thread_in` 로직은 제거된 enum 과 함께 사라지지만, 해당 flag 의 `has_flag::<AgentV2FeatureFlag>()` 체크는 다른 경로에서 유지

#### Part B-1 — 신규 git API 이식 (`thread_worktree_archive` 활성화 선행 인프라)

**방침 확정 (2026-04-23 사용자 승인 "경로 3 — 최소 이식, local-only stub, 상류 시그너처 유지")**

상류 PR #53941 의 `thread_worktree_archive.rs` 가 호출하는 `Repository` wrapper 는 실제로 **5종의 backend trait 메서드 + 관련 enum/proto** 에 의존한다. 조사 결과 Dokkaebi 에는 이들 전부가 부재 상태. 최소 이식 방침으로 범위 축소:

**① 추가할 trait 메서드 (`crates/git/src/repository.rs::GitRepository`)**
1. `checkout_branch_in_worktree(&self, branch_name: String, worktree_path: PathBuf, create: bool) -> BoxFuture<'_, Result<()>>` — 기존 worktree 에서 브랜치 checkout
2. `update_ref(&self, ref_name: String, commit: String) -> BoxFuture<'_, Result<()>>` — ref 를 커밋으로 업데이트
3. `delete_ref(&self, ref_name: String) -> BoxFuture<'_, Result<()>>` — ref 삭제
4. `create_archive_checkpoint(&self) -> BoxFuture<'_, Result<(String, String)>>` — (staged_sha, unstaged_sha) 쌍 생성
5. `restore_archive_checkpoint(&self, staged_sha: String, unstaged_sha: String) -> BoxFuture<'_, Result<()>>` — checkpoint 복원

**② 추가할 Repository wrapper (`crates/project/src/git_store.rs`)**
- `pub fn create_worktree_detached(&mut self, path: PathBuf, commit: String) -> oneshot::Receiver<Result<()>>` — `create_worktree_detached` 를 **신규 독립 wrapper** 로 도입 (기존 `create_worktree` 시그너처 유지). 내부에서 `CreateWorktreeTarget::Detached` 를 쓰지 않고 Dokkaebi 독자 경로로 `backend.create_worktree(path, Some(commit), detached=true)` 패턴 또는 새 backend 메서드 `create_worktree_detached_at` 호출. 구현 시 Dokkaebi `create_worktree` 시그너처 확인 후 최종 결정
- `pub fn checkout_branch_in_worktree(...)` / `pub fn update_ref(...)` / `pub fn delete_ref(...)` / `pub fn create_archive_checkpoint(...)` / `pub fn restore_archive_checkpoint(...)` — 상류 시그너처 그대로

**③ Remote 분기 처리 — `anyhow::bail!("not supported in remote repositories")`**
- Dokkaebi 는 collab 비활성 + SSH remote 에서 agent thread worktree archive 비지원 정책. RepositoryState::Remote 분기에서 `log::warn!` 후 `bail!` 로 명시적 에러. `unreachable!` 은 crash 유발 가능성 있어 회피.

**④ proto 추가 skip**
- `GitEditRef`, `GitRestoreArchiveCheckpoint`, `GitCreateArchiveCheckpoint`, `GitGetHeadSha`, `GitRepairWorktrees` 전부 skip. Dokkaebi 는 collab off 로 proto 경로 불필요.

**⑤ `create_worktree_detached` 세부 전략**
Dokkaebi 의 현재 `create_worktree` trait 시그너처 확인 필요. 두 가지 경우:
- (a) 상류와 호환 형태 → `CreateWorktreeTarget::Detached` enum 만 도입하고 내부 로직 재활용
- (b) Dokkaebi 독자 형태 → `create_worktree_detached` 를 **완전 별개 trait 메서드**로 추가 (시그너처 breaking change 회피)
Step 1 착수 첫 작업에서 Dokkaebi `create_worktree` 시그너처 전수 확인 후 (a)/(b) 최종 결정.

**⑥ FakeGitRepository stub (`crates/fs/src/fake_git_repo.rs`)**
- 모든 신규 메서드 `anyhow::Ok(())` 또는 `Ok(("".into(), "".into()))` no-op 반환. 실제 fs 조작 없음. 테스트 컴파일만 통과 목표.

**⑦ 이미 존재 (이식 불필요)**
- `head_sha` (`repository.rs:722`), `create_worktree` (`repository.rs:752, 1697`)

**작업량 재추정**: 약 350~500 라인 추가 (trait 5개 + RealGitRepository impl 5개 git2/CLI 호출 + Repository wrapper 6개 + FakeGitRepository stub 5개). 단일 세션 완주 목표.

**검증**: `cargo check -p git -p project -p fs` 통과, 신규 경고·에러 0.

#### Part B-2 — `thread_worktree_archive.rs` 활성화
- **대상 파일**: `crates/agent_ui/src/agent_ui.rs` 에 `pub mod thread_worktree_archive` 등록
- **호출처 연결**: `agent_panel.rs` refactor 에서 archive/restore 플로우 와이어링
- **사전 조건**: Part B-1 git API 4종이 컴파일 통과해야 `thread_worktree_archive.rs` 의 44 에러가 해소됨
- **검증**: `cargo check -p agent_ui` 통과

### 11.10.4. 작업 순서 (순차 진행, 각 단계 후 cargo check)
1. **Step 1 — Part B-1 (git API 4종 이식)**: 다른 Part 독립. `crates/git/src/repository.rs` 에 trait 메서드 + 실제 구현 추가, FakeGitRepository stub 추가. 1회 커밋 권장.
2. **Step 2 — Part A (agent_panel.rs refactor)**: 37 참조 재배선. `thread_view.rs` -45 + `recent_projects.rs` +2 동반. 1회 커밋 권장 (단일 주제).
3. **Step 3 — Part B-2 (thread_worktree_archive 활성화)**: mod 등록 + archive/restore 경로 배선. 1회 커밋 권장.
4. **Step 4 — 문서 갱신**: plan.md §11.10 체크리스트 완료 표시 + notes.md 최상단 기록 + release_notes.md 사용자 체감 항목 추가. 1회 커밋.

각 Step 완료 후 `cargo check -p <대상 크레이트> -p Dokkaebi` 통과 확인. 중간 빌드 실패가 길어지면 즉시 분할 또는 롤백.

### 11.10.5. 검증 방법
- `cargo check -p git -p project -p fs` (Part B-1 후)
- `cargo check -p agent_ui -p recent_projects` (Part A 후)
- `cargo check -p agent_ui` (Part B-2 후)
- `cargo check -p Dokkaebi` (최종)
- 신규 경고·에러 0 목표
- 빌드 후 수동 시나리오 (사용자 요청 시):
  - 새 스레드 → `Ctrl+Shift+T` 로 worktree 선택기 오픈
  - "현재 브랜치" / "main 에서 새로" / "기존 브랜치" 3 옵션 각각 시도 → 깜박임 없이 worktree 생성 확인
  - 스레드 archive → worktree 보존 확인 → unarchive → 복원 확인
  - detached HEAD worktree 가 picker 에 표시되는지 확인

### 11.10.6. 미이식 skip 영역 (의도적 제외)
- `crates/workspace/src/multi_workspace.rs` (+19/-3), `persistence.rs` (+10/-1), `workspace.rs` (+2/-2) — Dokkaebi 독자 `AgentV2FeatureFlag` 기반 `multi_workspace_enabled` 구조 + `RemoteConnectionIdentity` 부재로 상류 patch 의 수정 대상이 정확히 매칭되지 않음. 이식 시 독자 경로 우회 발췌가 필요해 작업량 대비 체감 가치 낮음.
- `crates/sidebar/src/sidebar.rs` (+9/-1), `sidebar_tests.rs` (+36) — Dokkaebi `sidebar` 크레이트 부재 (자동 제외)
- `crates/collab/tests/integration/git_tests.rs` (+3) — Dokkaebi `collab` 크레이트 부재
- `crates/remote_server/src/remote_editing_tests.rs` (+1), `worktree/tests/integration/main.rs` (+2) — 상류 신규 API (`add_linked_worktree_for_repo`, `root_repo_common_dir`) 부재로 테스트 컴파일 불가
- `crates/zed/src/visual_test_runner.rs` -639 — Dokkaebi 동등 파일 상태 확인 필요 (대부분 자동 제외 추정)
- `assets/keymaps/default-macos.json`, `default-linux.json` — Windows 전용 정책

### 11.10.7. 리스크 및 롤백 전략
- **리스크 1**: agent_panel.rs refactor 중 Dokkaebi 서브에이전트 뷰 탭 통합 경로가 예상 외 간섭 — **완화**: grep 전수 검증 완료, 간섭 없음 확인. Step 2 실패 시 해당 커밋만 revert.
- **리스크 2**: thread_worktree_archive.rs 가 Dokkaebi 에 없는 archive thread 이벤트 훅을 전제 — **완화**: Part B-1 완료 후 `cargo check -p agent_ui` 로 남은 에러 전수 파악, 필요 시 stub 처리.
- **리스크 3**: multi_workspace.rs skip 으로 worktree 상태가 workspace 재시작 시 복원 안 됨 — **완화**: 이는 애초 상류 동일 PR 범위라서 Dokkaebi 영향 제한적. 후속 단독 PR 로 처리 가능.

### 11.10.8. 진행 체크리스트
- [x] 사용자 승인 (2026-04-23 "옵션A, 이어받기 → main 브랜치로 작업, 옵션 B 추가")
- [x] 선행 #53094 이식 완료 및 단독 commit (`d7f148a813`)
- [/] plan.md §11.10 작성
- [ ] 상류 PR #53941 patch 의 git API 4종 시그너처 상세 조사 (Step 1 착수 직전)
- [ ] **Step 1**: Part B-1 git API 4종 이식 + `cargo check -p git -p project -p fs`
- [ ] **Step 2**: Part A agent_panel.rs refactor (`StartThreadIn` 제거 37곳, `thread_view.rs` -45, `recent_projects.rs` +2) + `cargo check -p agent_ui -p recent_projects`
- [ ] **Step 3**: Part B-2 `thread_worktree_archive.rs` 활성화 + `cargo check -p agent_ui`
- [ ] 최종 `cargo check -p Dokkaebi` 통과
- [ ] `notes.md` 최상단에 기술 세부 기록
- [ ] `release_notes.md` v0.4.0 `### UI/UX 개선` 또는 `### 새로운 기능` 섹션에 사용자 체감 항목 추가

### 11.10.9. 예상 작업 규모
- Part B-1: git API 4종 × (trait 메서드 + git2 실구현 + fake stub) ≈ 250~400 라인 추가, 1 세션 중반
- Part A: agent_panel.rs 37 참조 재배선 + thread_view.rs -45 + recent_projects +2 ≈ 600~1000 라인 변경, 1~2 세션
- Part B-2: mod 등록 + 와이어링 ≈ 50~150 라인, 0.5 세션
- 총 예상: **2~3 세션** (단일 세션 완주 가능성 중간)

---

## 12. 영향 범위 외 (변경 없음)
- `README.md` (CLAUDE.md 프로젝트 규칙: 수정 금지)
- `assets/keymaps/default-macos.json`, `default-linux.json` (Windows 전용 정책)
- `crates/repl`, `crates/dev_container/src/{docker,devcontainer_json,devcontainer_manifest}.rs`, `crates/language_models_cloud/` (파일 부재/비대상)
- Dokkaebi 독자 수정: `crates/zed/src/zed/windows_only_instance.rs` 좀비 감지 로직, `crates/zed/src/main.rs` Dev 채널 skip 제거 이력, `crates/cli/Cargo.toml` bin name `dokkaebi-cli`
