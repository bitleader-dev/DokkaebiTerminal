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
- [x] #53184 헤딩 앵커 링크 (5f) — **보류**: Dokkaebi `ParsedMarkdownData`에 `heading_slugs` 필드 부재(인프라 선행 백포트 필요)
- [x] #53086 각주 지원 (2f) — **보류**: 동일 patch가 `heading_slugs`·`mermaid_diagrams` 필드를 전제로 구조체 리터럴 구성. 인프라 선행 백포트 필요
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
- [x] #53552 update indicator title bar (1f) — **이미 달성**: Dokkaebi `GithubUpdater` 가 Downloading 상태 항상 렌더
- [x] #53662 deleted folders recent 1주일 grace (1f) — 7일 유예 조건 호출처 이전
- [x] #54056 welcome tab project path (1f) — `project_name` 헬퍼 + i18n `welcome.remote_project` 고아 키 제거
- [x] #53808 BGRA8 WGPU panic (3f) — **보류**: +190/-24 renderer 대규모 재구조화, 실사용자 영향 제한적, 별도 Phase로
- [x] #53998 flexible dock widths (3f) — **보류**: `width_fraction_for_pane` → `full_height_column_count` 전면 교체, Dokkaebi 독자 호출처 존재, 별도 작업
- [x] #53915 CLI activate window 타이밍 (2f) — **skip**: #53663 skip과 연쇄 + Dokkaebi CLI 용도상 자동 창 활성화 부작용

### 6-4. Agent / ACP 소규모 — 부분 완료 (2026-04-23)
- [x] #53216 opencode ACP 반복 prompt (1f)
- [x] #53791 full branch name in picker (1f) — **skip**: `thread_branch_picker.rs` Dokkaebi 부재
- [x] #53859 open_thread duplicate sessions (1f)
- [x] #53657 zoomed agent panel scroll (1f) — **skip**: `max_content_width` Dokkaebi 부재, 해당 zoom 이슈 경로 없음
- [x] #52975 anthropic custom 모델 thinking 보존 (1f)
- [x] #54431 ACP replay events drop (1f) — **보류**: Dokkaebi 이미 사전 등록 패턴 적용(main 이슈 해결), pending_sessions 인프라는 별도 대규모 작업
- [x] #54116 focused tool call 보존 (2f)
- [x] #54134 thread title 실패 표시 (4f 중 sidebar.rs 제외 3f)
- [x] #54138 ACP 프로세스 종료 double borrow (6f 중 핵심 fix만, test_support 모듈 skip)
- [x] #53884 action_log commit race (4f) — **보류**: `buffer_diff` diverge로 `SetSnapshotResult` 구조 변경 단순 이식 불가
- [x] #53696 agent panel UI fix (4f 중 핵심 3건: Panel::min_size + 큐 메시지 포커스 + MIN_PANEL_WIDTH, max_content_width 변경 skip)

### 6-5. AI 모델 — ✅ 완료 (2026-04-23)
- [x] #53543 Ollama 컨텍스트 길이 (1f) — ModelShow Deserialize 에 parameters num_ctx 우선 파싱
- [x] #54106 Copilot reasoning effort (1f) — **이미 적용됨** (v0.232.2 백포트 시 상위 fix)
- [x] #54191 Google cloud model RefCell panic (2f) — **skip**: Dokkaebi cloud.rs 에 `CloudLlmTokenProvider`·`to_async()` 경로 부재

### 6-6. Git — 부분 완료 (2026-04-23)
- [x] #52965 git panel 트리 Enter 토글 (1f)
- [x] #53803 Git Graph 디자인 (2f) — **보류**: Dokkaebi `get_selected_repository` 경로·색상 스타일 diverge
- [x] #53929 diff hunk staging race (2f)
- [x] #53669 worktree naming regression (2f) — **skip**: Dokkaebi `resolve_worktree_branch_target` 부재(구조 재편)
- [x] #52996 bare repo recent projects (3f) — `original_repo_path_from_common_dir` 반환 타입 Option 변경
- [x] #53444 ANSI escape strip (3f) — alacritty_terminal vte ansi 의존성 추가 + GitOutputHandler

### 6-7. Languages / 기타 — 부분 완료 (2026-04-23)
- [x] #53546 TopoJSON 하이라이팅 (1f) — json/config.toml path_suffixes 에 topojson 추가
- [x] #54201 tsgo LSP fix (2f) — **보류**: Dokkaebi lsp-types rev diverge

**검증**: 각 카테고리 커밋 단위로 `cargo check -p <crate>` + Phase 끝에서 `cargo check -p Dokkaebi`.

---

## 7. Phase 4 — 중규모 기능 (8건) — 부분 완료 (2026-04-23)
- [x] #53452 mouse_wheel_zoom 설정 (2f) — Ctrl+스크롤 폰트 크기 조정 (`event.modifiers.secondary()` 분기)
- [x] #53504 hover_popover_sticky / hover_popover_hiding_delay (9f) — 4파일만 이식(settings_ui skip), core fix 완료
- [x] #53710 `workspace: format and save` 액션 (11f) — **보류**: trait 메서드 추가 Dokkaebi 11파일 diverge 가능성
- [x] #54316 `limit_content_width` 설정 (8f) — **skip**: Dokkaebi `max_content_width` 부재
- [x] #54318 favorite 모델 thinking/effort/fast 저장 (6f) — **보류**: Dokkaebi agent 영역 diverge, 구조 변경 많음
- [x] #48752 toggle block comment (22f) — **보류**: vim diverge + grammars 7종 대규모
- [x] #54256 Netpbm (PNM) 이미지 프리뷰 (5f) — `ImageFormat::Pnm` variant 추가(gpui + project + agent_ui)
- [x] #54224 unsaved scratch buffer 세션 유지 (9f) — **보류**: sidebar 크레이트 부재 + workspace 구조 변경 대규모

**검증**: `cargo check -p Dokkaebi` 통과, 신규 경고 0건.

---

## 8. Phase 5 — 구조 변경 / 방침 결정 항목 (4건) — 부분 완료 (2026-04-23)
- [x] **#53521** Fix with Assistant 제거 (방침 B)
  - `AssistantCodeActionProvider` struct + impl, `register_workspace_item`, `ItemAdded` 분기, 관련 import 삭제
- [x] **#53941** 새 worktree UX 개편 (37f) — **보류**: 신규 파일 2개(`thread_worktree_archive.rs`, `thread_worktree_picker.rs`) + Dokkaebi worktree 구조 diverge, Phase 5 단독 범위 초과
- [x] **#53560** ACP npm `--prefix` 시그니처 변경 (2f)
  - `npm_command` 에 `prefix_dir` 파라미터 추가, `SystemNodeRuntime::global_node_modules` 필드 제거
  - agent_server_store.rs 호출처 `None` prefix 로 migrate
- [x] **#48003** HTTP `context_servers` deprecated `settings` 필드 제거 (Breaking, 3f) — **보류**: Dokkaebi migrator 가 `m_2026_04_*` 계열 선행 백포트 지연 상태라 단독 추가 위험

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
- [x] Phase 1 착수 승인 (2026-04-23) → Phase 1~25 모두 완료(2026-04-24)

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
- [x] **#54224 unsaved scratch buffer 세션 유지** (9f) — **보류 유지**: `workspace/persistence.rs` +310 대규모 DB schema 변경, Dokkaebi 독자 schema 와 충돌 위험
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
- [x] **#53998 flexible dock widths** (3f) — Phase 25 영구 삭제 결정(2026-04-24). Dokkaebi `dock.rs` 가 `resize_active_panel` + `resize_all_panels` 두 메서드로 **이미 분리** + `pane_group.rs` 의 `width_fraction_for_pane` → `full_height_column_count` 변환은 이미 적용된 상태. 상류 patch 의 dock.rs 변경은 Dokkaebi 의 다른 구조와 1:1 매핑 불가 + 동일 의도 이미 다른 방식으로 구현 → 적용 불필요로 확정
- [x] **#53669 worktree naming regression** — Phase 25 영구 삭제 결정(2026-04-24). 옵션 B 가 random `branch_names::generate_branch_name` 으로 자동 생성하므로 회귀 패턴 자체가 발현되지 않아 적용 불필요로 확정
- [x] **#54201 tsgo LSP** — 이미 적용 완료. 커밋 `16b77fed44` Phase 8F 에서 lsp-types rev `f4dfa89...` 로 갱신(상류 동일). plan.md 의 "Dokkaebi 독자 rev 유지" 기록은 outdated
- [x] **#54318 favorite 모델 thinking/effort/fast** — 이미 적용 확인(2026-04-24, Phase 24 검증). `favorite_models.rs:9-35` `language_model_to_selection` 헬퍼로 thinking/effort/speed 함께 저장 + `language_model/src/request.rs:472` `Speed` 필드 완비
- [x] **#54224 unsaved scratch buffer 세션 유지** — Phase 24 적용 완료(2026-04-24). `recent_workspaces_on_disk` → `recent_project_workspaces` + `garbage_collect_workspaces` 분리, session_id 기반 현재/이전 세션 보존, 신규 테스트 7건 통과
- [x] **#48003 HTTP context_servers deprecated 제거** — Phase 25 Part A 적용 완료(2026-04-24). 사용자 확답 "HTTP MCP 사용". m_2026_04_15 신설(19줄) + migrator 등록 + 테스트 1건 통과
- [x] **#53086 마크다운 각주 지원** — 이미 적용 확인(2026-04-24, Phase 24 검증). `markdown/src/parser.rs:40,300,504,523` `footnote_definitions`/`FootnoteDefinition`/`FootnoteReference`/`build_footnote_definitions` 모두 존재
- [x] **#53184 마크다운 헤딩 앵커 링크** — 이미 적용 확인(2026-04-24, Phase 24 검증). `markdown/src/parser.rs:88,156,518` `heading_slugs`/`build_heading_slugs`/`parse_heading_slugs` + `markdown.rs:494,646` 렌더 동작
- [x] **#53808 WGPU BGRA8 panic** — 이미 적용 확인(2026-04-24, Phase 25 검토). `gpui_wgpu/src/wgpu_atlas.rs:36,50,57,65,92,186` `color_texture_format`/`from_context`/`Bgra8Unorm | Rgba8Unorm` 분기 완비. release_notes.md v0.4.0 에 "구형·가상 GPU 기동 크래시 방지" 항목 존재
- [x] **#53941 새 worktree UX 개편** — Phase 10 옵션 B 적용(2026-04-23) + 잔여 Part B-2/B-3 = Phase 25 Part B 영구 삭제(2026-04-24). thread_worktree_archive.rs 1032 라인 + git API 6종 + SwitchWorktree action + 토스트 i18n 모두 제거
- [x] **#54431 ACP replay events drop** — 이미 적용 확인(2026-04-24, Phase 24 검증). `agent_servers/src/acp.rs:1120-1132` pre-register session 패턴 + `pending_sessions` 인프라 완비
- [x] **#53884 action_log commit race** — 이미 적용 확인(2026-04-24, Phase 24 검증). `action_log/src/action_log.rs:289-294` `BaseTextChanged` 만 commit signal 로 사용 + `buffer_diff.rs:1955,2038` `BaseTextChanged` emit

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
- [x] **전체 Phase 8 진행 방침** — Phase 24/25 로 통합 처리(2026-04-24). 보류 12건 모두 적용/이미 완료/영구 삭제 결정 완료
- [x] **Sub-Phase 진행 방식** — 단일 Phase 8A(ACP SDK)만 별도 브랜치로 진행 후 main 머지 완료. 나머지는 Phase 24(#54224) + Phase 25(#48003 + thread_worktree_archive 영구 삭제) 로 일괄 처리
- [x] **Dokkaebi 제거 영역 skip 기준** — §11.7.1 확정 그대로 적용
- [x] **선행 PR 의 추가 선행 발견 시 처리 방침** — 추가 선행 발견 0건. 모든 보류가 단독 적용 가능 또는 영구 삭제로 종결
- [x] **세션 범위 초과 시 처리** — Phase 8A 만 한 세션 분량 초과로 브랜치 분리 처리. 다른 Phase 는 단일 세션 내 완료

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
- `crates/recent_projects/src/recent_projects.rs` (+2) — 옵션 B(2026-04-23) 결정. Dokkaebi 에 `MultiWorkspace::find_or_create_local_workspace` 가 부재해 상류 patch 가 추가하는 `Some(init), OpenMode::Activate` 2 인자를 적용할 대상 호출이 없음. Part A 옵션 A 전면 상류화(workspace API 이식) 시점에 함께 처리.
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
- [x] plan.md §11.10 작성
- [x] 상류 PR #53941 patch 의 git API 4종 시그너처 상세 조사 (Step 1 착수 직전)
- [x] **Step 1**: Part B-1 git API 6종 이식 + `cargo check -p git -p project -p fs` 9.54s 통과 → commit `a499230169`
- [x] **Step 2**: Part A agent_panel.rs refactor → **옵션 B(표면 치환) 로 완료** (§11.10.10 · §11.10.11 참조)
- [x] **Step 3**: Part B-2 `thread_worktree_archive.rs` 활성화 → **2026-04-24 보류 결정** (§11.10.12 참조). Part B-3 이식 규모(1000~1600 라인) · 옵션 A 승격 비용 대비 바이브 모드 방향성에 비추어 체감 가치 낮음
- [x] 최종 `cargo check -p Dokkaebi` 통과 (2026-04-23 옵션 B 완료 시점)
- [x] `notes.md` 최상단에 기술 세부 기록 (2026-04-23 항목)
- [x] `release_notes.md` v0.4.0 `### UI/UX 개선` 에 "에이전트 워크트리 선택기 통합" 추가

### 11.10.10. Part A 다음 세션 재개 포인트 (2026-04-23 결정)

**배경**: Part A 의 agent_panel.rs refactor 가 당초 예상보다 훨씬 큰 작업으로 판명됨.
- 상류 PR #53941 의 agent_panel.rs patch 규모: **40+ hunks / 2134 diff 라인 / +602/-1017**
- Dokkaebi agent_panel.rs: 6288 라인 (상류 v0.232.2 base + Phase 1~8J 누적 수정 + i18n + AgentV2FeatureFlag 독자 분기 4곳 + "AI Agent" 리브랜딩)
- 상류 v0.233.7 agent_panel.rs: 6929 라인
- 라인·구조·식별자 모두 다르므로 `git apply` 불가. 수동 hunk 매핑 필요.

**Step 2-a/2-b 분리안 검토 결과**: 빌드 통과 최소 refactor 조차 37 참조 중 대부분(필드·핸들러·렌더링·테스트)을 건드려야 하므로 "최소 refactor" 라는 분리점이 실질적으로 성립하지 않음. Step 2-a 시도 시 28 컴파일 에러 발생 확인.

**결정 (사용자 2026-04-23 "a")**: 이번 세션은 imports rollback + 문서 기록으로 마감. Part A 는 다음 세션에 **전체를 통째로** 착수.

**다음 세션 재개 조건**
- 현재 main HEAD: `a499230169` (Step 1 Part B-1 완료)
- agent_panel.rs imports: 커밋 `d7f148a813` 시점으로 복원 완료 (StartThreadIn/CycleStartThreadIn 사용 유지)
- agent_ui.rs: `CreateWorktree`/`SwitchWorktree`/`NewWorktreeBranchTarget`/`ToggleWorktreeSelector`/`thread_worktree_picker` mod 이미 등록됨 (Phase 8I 1차 유산)
- 빌드 상태: 전체 통과 (cargo check -p Dokkaebi)

**다음 세션 작업 순서 (Part A + Part B-2 묶음)**
1. agent_panel.rs imports 변경 (`CycleStartThreadIn` + `StartThreadIn` → `CreateWorktree` + `SwitchWorktree` + `NewWorktreeBranchTarget` + `ToggleWorktreeSelector` + `thread_worktree_picker::ThreadWorktreePicker`)
2. SerializedAgentPanel 필드 `start_thread_in: Option<StartThreadIn>` 제거
3. AgentPanel 필드 `start_thread_in: Option<StartThreadIn>` 제거
4. init() 내부 register_action 4개 처리: `StartThreadIn` 제거, `CycleStartThreadIn` 제거, `ToggleWorktreeSelector` 추가, `CreateWorktree`/`SwitchWorktree` 핸들러 추가
5. impl StartThreadIn 블록 제거
6. 로직 영역 37 참조 중 AgentPanel 내부 메서드 재작성:
   - `set_start_thread_in` → 제거 또는 내부 전용 유지 (상류는 완전 제거)
   - `cycle_start_thread_in` → 제거
   - 렌더링에서 `StartThreadIn::LocalProject` / `NewWorktree` 분기 → title bar branch picker 로 위임
   - eager worktree 생성 경로 (`FirstSendRequested` 이벤트 → worktree 즉시 생성)
7. Dokkaebi 독자 `AgentV2FeatureFlag` 분기 (L23 import, L881, L2254, L4041) 보존 — 상류와 별개 feature flag 이므로 유지
8. conversation_view/thread_view.rs -45 — FirstSendRequested 이벤트 경로 제거
9. recent_projects/src/recent_projects.rs +2 — find_or_create_local_workspace 호출 경로 변경
10. 테스트 영역 L5891~6230 재작성 (`set_start_thread_in_for_tests` 등)
11. agent_ui.rs 에서 `StartThreadIn` enum + `CycleStartThreadIn` action 제거 (agent_panel 에서 참조 완전 제거 후)
12. Part B-2: agent_ui.rs 에 `pub mod thread_worktree_archive` 등록, agent_panel archive/restore 와이어링
13. `cargo check -p agent_ui -p recent_projects` 통과
14. `cargo check -p Dokkaebi` 통과
15. notes.md 최상단 기록 + release_notes.md 사용자 체감 항목 추가
16. commit

**다음 세션 시작 프롬프트 (참고용)**
```
Phase 10 Step 2 (Part A + Part B-2) 착수. plan.md §11.10.10 기준.
main HEAD = a499230169 (Step 1 완료).
agent_panel.rs 현재 이전 상태 복원됨. StartThreadIn 37참조 제거 + CreateWorktree/SwitchWorktree 재배선 + thread_worktree_archive 활성화.
```

**리스크 주의 (다음 세션)**
- AgentV2FeatureFlag 분기 4곳 보존 필수 — Dokkaebi 독자 기능
- i18n `t("키", cx)` 호출 패턴 유지 (상류는 영문 하드코딩)
- 서브에이전트 뷰 탭 관련 conversation_view/thread_view.rs 의 `FirstSendRequested` 이벤트 경로가 Dokkaebi 독자 로직과 엮여 있는지 재검증 (이전 Phase 8I 기록 참조: "StartThreadIn::NewWorktree 지연 worktree 생성 메커니즘과 짝")
- "AI Agent" 리브랜딩 문자열 보존

### 11.10.11. Part A 옵션 B 완료 기록 (2026-04-23)

**사용자 결정**: 2026-04-23 "b 진행". plan.md 제시 3 옵션 중 옵션 B(표면 치환) 선택.

**실제 수정 요약**
- `crates/agent_ui/src/agent_panel.rs`: StartThreadIn enum/CycleStartThreadIn action 참조 전수 제거 + ThreadWorktreePicker 기반 PopoverMenu 로 재작성. `start_thread_in_menu_handle` 타입 변경, `create_worktree`/`switch_to_worktree`/`toggle_worktree_selector` 3 핸들러 추가, `handle_first_send_requested` 제거, `current_worktree_label` 유틸 추가.
- `crates/agent_ui/src/conversation_view/thread_view.rs`: `AcpThreadViewEvent::FirstSendRequested` variant + intercept 블록 -45 제거. enum 자체는 빈 상태로 유지(향후 상류 동기화 여지).
- `crates/agent_ui/src/agent_ui.rs`: `StartThreadIn` enum + `CycleStartThreadIn` action 제거. `thread_worktree_archive` mod 는 비활성 유지.
- `crates/zed/src/visual_test_runner.rs`: `StartThreadIn` import → `CreateWorktree`/`NewWorktreeBranchTarget`. dispatch 치환, `set_start_thread_in_for_tests` 호출 제거, screenshot 3 시나리오(New Worktree 선택) 은 StartThreadIn 부재로 captured state 재조정.
- `crates/recent_projects/src/recent_projects.rs`: Dokkaebi 에 `find_or_create_local_workspace` 부재로 skip (§11.10.6 기록).

**옵션 B 의 기능 제약 (알려진 퇴화)**
1. picker 의 `CreateWorktree { branch_target }` payload 는 현재 무시. 어떤 branch_target(CurrentBranch/ExistingBranch/NewBranch)을 선택해도 Dokkaebi 는 `branch_names::generate_branch_name` 으로 자동 브랜치명을 생성하는 기존 경로로 귀결됨.
2. `SwitchWorktree` 는 미구현 토스트만 표시. linked worktree 전환은 `MultiWorkspace::find_or_create_workspace` 등 워크스페이스 크레이트 신규 API 이식 이후로 연기.
3. 기존 "빈 편집기에 메시지 작성 → Enter → worktree 생성" 경로(FirstSendRequested) 제거. picker 경유만 트리거 가능. 상류 PR #53941 UX 와 일치.
4. `worktree_name` 지정(picker 의 CreateNamed 항목) 역시 payload 로 전달되지만 무시됨. 생성 자체는 작동하나 지정된 이름은 적용되지 않음.

**검증 결과**
- `cargo check -p agent_ui` ✅ (신규 에러 0, 신규 경고 0)
- `cargo check -p recent_projects` ✅
- `cargo check -p Dokkaebi` ✅

**후속 작업 (별도 Phase 로 분리)**
- **Part B-3** (신규): `crates/agent_ui/src/thread_metadata_store.rs` 에 `ThreadMetadataStore`/`ThreadId`/`ArchivedGitWorktree` 타입 이식 + `Project::wait_for_worktree_release` + `Repository::repair_worktrees` 이식 → 그 후 `thread_worktree_archive` mod 재활성화(Part B-2).
- **Part A 옵션 A 승격** (선택): workspace 크레이트 신규 API (`OpenMode`, `MultiWorkspace::find_or_create_workspace`, `Workspace::run_create_worktree_tasks`, `PreviousWorkspaceState` 등) 이식 후 agent_panel 의 `handle_worktree_creation_requested` 를 상류 `handle_worktree_requested(previous_workspace_state)` 로 교체. SwitchWorktree 실동작·path remapping 다중 파일·dock 복원·Loading 상태 획득.

### 11.10.12. Part B-2/B-3 및 옵션 A 승격 보류 결정 (2026-04-24)

**배경**: 2026-04-24 세션에서 Part B-3 착수 시 규모 재조사. thread_worktree_archive.rs 가 요구하는 실제 의존성은 plan.md §11.10.3 Part B-1 목록(git API 6종)을 크게 초과함이 확인됨.

**Part B-3 실제 요구 범위 (재조사 결과)**
- `crates/agent_ui/src/thread_metadata_store.rs`: 상류 3640 라인 파일 vs Dokkaebi 의 `SidebarThreadMetadataStore` 1252 라인 — 이름·`ThreadMetadata` 필드 구성·primary key 체계(uuid `ThreadId` vs `acp::SessionId`)·archive 플래그 유무가 모두 다른 별개 스토어. 공존 도입 시 DB 도메인 분리 + 같은 thread 에 대한 이중 저장 구조 필요
- `Project::wait_for_worktree_release` 신규 이식
- `git_store::Repository::{head_sha, repair_worktrees}` wrapper 이식 (`head_sha` trait 메서드는 Part B-1 에 이식됐으나 wrapper 누락, `repair_worktrees` 는 trait + wrapper 양쪽 부재)
- thread_worktree_archive.rs 호출처 수정 (`MultiWorkspace::workspaces()` 결과의 `.cloned()` 직접 호출 불가, 반환 타입 E0282 다수)
- 추정 총 규모: **1000~1600 라인, 1.5~2.5 세션** (이전 plan.md §11.10.11 의 "50~150 라인, 0.5 세션" 추정은 오판)

**옵션 A 승격 실제 요구 범위**
- `workspace` 크레이트 신규 API 이식: `OpenMode::{Activate, Add}`, `MultiWorkspace::find_or_create_workspace(path_list, ..., init, open_mode)`, `MultiWorkspace::find_or_create_local_workspace`, `Workspace::run_create_worktree_tasks`, `PreviousWorkspaceState` (dock_structure/open_file_paths/active_file_path 캡처)
- agent_panel 의 Dokkaebi 독자 `handle_worktree_creation_requested` → 상류 `handle_worktree_requested(previous_workspace_state)` 전면 교체 (자동 브랜치명 생성 등 기존 동작 변경 위험)
- plan.md §11.10.6 의 workspace/multi_workspace/persistence skip 정책 부분 해제 필요

**보류 결정 (사용자 2026-04-24 "B 진행" = 옵션 β)**
- SwitchWorktree 실동작, archive/restore 경로, path remapping 다중 파일, dock 복원, Loading 상태 등 상류 PR #53941 의 미이식 잔여 항목은 **전부 보류**.
- 근거:
  1. 핵심 UX(`ThreadWorktreePicker` 통합)는 옵션 B 로 이미 달성. CreateWorktree 는 정상 동작.
  2. `notes.md` Phase 0 리서치에 따른 Dokkaebi 바이브 모드 방향성(코드 지식 없는 1인 사용자 · 채팅창 하나만 노출 · 개발자 어휘 비노출)과 linked worktree 간 전환·archive/restore 같은 상급 git 워크플로우가 충돌.
  3. 잔여 항목 구현 비용(Part B-3 1000~1600 라인 + 옵션 A 승격 수천 라인) 대비 바이브 모드 사용자 체감 가치가 낮음.
  4. 사용자가 다른 브랜치 파일을 보려면 기존 branch picker / git panel 경로로 대체 가능.

**상태**
- `thread_worktree_archive` mod 는 비활성 유지. 재활성화 트리거는 사용자 명시적 요청이 있을 때만.
- `crates/agent_ui/src/agent_panel.rs::switch_to_worktree` 는 "기존 워크트리로 전환은 아직 지원되지 않습니다" 토스트 계속 유지. 향후 제거 금지(picker 가 dispatch 한 액션을 swallow 하면 사용자가 무반응으로 혼란).

**영구 삭제 확정 (2026-04-24, 사용자 결정 "2 B")**: Phase 25 Part B 에서 `thread_worktree_archive.rs` (1032 라인) + `SwitchWorktree` action + `switch_to_worktree` 메서드 + 토스트 i18n + git API 6 종(`create_worktree_detached`/`checkout_branch_in_worktree`/`update_ref`/`delete_ref`/`create_archive_checkpoint`/`restore_archive_checkpoint`) 전량 영구 삭제. picker 의 기존 worktree entry 도 미표시. 재이식이 필요한 경우 git history(커밋 `a499230169`) 에서 복구 가능.

**이번 세션 코드 변경**: 없음 (archive mod 재비활성화로 원복, 임시 조사 디렉터리 정리). 문서만 갱신.

### 11.10.9. 예상 작업 규모
- Part B-1: git API 4종 × (trait 메서드 + git2 실구현 + fake stub) ≈ 250~400 라인 추가, 1 세션 중반
- Part A: agent_panel.rs 37 참조 재배선 + thread_view.rs -45 + recent_projects +2 ≈ 600~1000 라인 변경, 1~2 세션
- Part B-2: mod 등록 + 와이어링 ≈ 50~150 라인, 0.5 세션
- 총 예상: **2~3 세션** (단일 세션 완주 가능성 중간)

---

## 11.11. Phase 11 — 협업·클라우드 잔재 정리 Part 1 (2026-04-24 계획)

> **성격**: Zed 백포트와 별개의 Dokkaebi 독자 최적화. 사용자 요청 기반(1인 CLI 중심 사용자 관점 불필요 잔재 제거).
> **범위 제한**: 저리스크 dead code·미사용 의존성만 1차 처리. 대규모 기능 제거/정책 결정 항목은 Phase 12+ 로 분리.
> **상태**: 계획 작성, 사용자 승인 대기.

### 11.11.1. 목표
1인 개인 사용자 · CLI 중심(주로 Claude Code CLI) 사용 환경에서 협업/클라우드 잔재 중 **빌드 영향 0 또는 conditional cfg 확장만으로 해결되는 항목**만 제거. 동작 변화 없이 코드 베이스·의존성 트리 축소.

### 11.11.2. 포함 범위 (Part 1)
- **B-1** `crates/collab_ui/` orphan 디렉터리 삭제 — workspace 비소속 dead directory
- **B-2** `libwebrtc` / `webrtc-sys` 의존성 완전 제거 — Windows MSVC 에서도 `EchoCanceller` fake 구현으로 통일

### 11.11.3. 범위 외 (Phase 12+ 로 분리, 별도 승인)
- `client::SignIn`/`SignOut`/`Reconnect` 액션 제거: UI 호출 **51 개 파일** 영향(검증: grep `SignIn|SignOut|Reconnect|request_sign_out`), 단계적 접근 필요
- `cloud_api_client`·`cloud_api_types`·`cloud_llm_client` 정리: `edit_prediction`·`language_models/provider/cloud.rs`·`ai_onboarding`·`web_search_providers/cloud.rs` 와 상호 의존 → edit_prediction 제거 선행 필요
- `edit_prediction*` 4개 + `copilot`/`copilot_chat` 제거: `default.json` `edit_predictions` 섹션 panic 위험(CLAUDE.md 규칙 §백포트 실전 주의), 초기화 경로 수정 동반
- `vim`/`vim_mode_setting` 제거: 사용자 vim 사용 여부 답변 대기
- `task`/`tasks_ui`, `snippet`/`snippet_provider`/`snippets_ui`: 사용자 답변 대기
- `journal`/`image_viewer`/`markdown_preview`/`svg_preview`/`web_search`: 소형 GUI 기능, 사용자 답변 대기
- Anthropic 외 LLM 프로바이더 크레이트(`lmstudio`, `ollama`, `mistral`, `deepseek`, `codestral`, `google_ai`, `open_router`, `vercel`, `x_ai`, `bedrock`, `opencode`): 사용자 선택 대기
- `feedback`/`telemetry`: 이미 no-op/empty URL, 우선순위 낮음 → 보류

### 11.11.4. Part 1 Step 1 — `crates/collab_ui/` orphan 디렉터리 삭제

**대상 파일**
- `crates/collab_ui/src/collab_panel.rs` (3,383 라인)
- `crates/collab_ui/src/notification_panel.rs` (728 라인)
- `crates/collab_ui/` 디렉터리 전체 (Cargo.toml 부재)

**근거 (코드 확인)**
- `Cargo.toml` workspace members 214 개 목록에 `crates/collab_ui` 없음 → 빌드되지 않는 파일 시스템상 orphan
- 내부 `use` 구문이 참조하는 `call::ActiveCall`, `channel::{Channel, ChannelEvent, ChannelStore}` 크레이트 역시 workspace 비소속 → 설령 workspace 에 되돌려도 의존 그래프 불완전
- `grep "crates/collab_ui"` 로 참조 0 건 확인 필요

**작업 단계**
- [x] `grep -r "crates/collab_ui\|collab_ui::\|use collab_ui" .` 결과 재확인 — 코드 참조 0 건(파일 `file_finder_tests.rs` 의 fixture 문자열 "collab_ui" 는 fuzzy finder 테스트 가상 파일명, 실제 크레이트 참조 아님)
- [x] `git rm -r crates/collab_ui` 로 디렉터리 전체 제거(`collab_panel.rs`, `notification_panel.rs`)

**검증**
- [x] `cargo check -p Dokkaebi` 통과 (의존 그래프 변화 없음, 신규 경고·에러 0)
- [x] `git grep -l collab_ui` 에서 코드 참조 0 건(문서·tests fixture 제외)

**리스크**: 0 (workspace 비소속 · 빌드 미포함 · 참조 없음)

### 11.11.5. Part 1 Step 2 — `libwebrtc` / `webrtc-sys` 의존성 제거

**현재 상태 (코드 확인 완료)**
- `Cargo.toml` L576 `libwebrtc = "0.3.26"`, L763 `webrtc-sys = "0.3.23"` workspace deps 정의
- `Cargo.toml` L834-835 `[patch.crates-io]` 섹션에서 `zed-industries/livekit-rust-sdks` git rev 로 재정의
- `crates/audio/Cargo.toml` L30-31 `[target.'cfg(not(any(all(target_os = "windows", target_env = "gnu"), target_os = "freebsd")))'.dependencies]` 블록에서 `libwebrtc.workspace = true`
- `crates/audio/src/audio_pipeline/echo_canceller.rs`:
  - L1-36 `real_implementation` (Windows MSVC · Linux · macOS): `libwebrtc::native::apm::AudioProcessingModule`
  - L38-48 `fake_implementation` (Windows GNU · FreeBSD): 이미 no-op 구현 제공, API 호환 검증된 상태
- `crates/audio/src/audio_pipeline.rs`: `Audio` 구조체 필드 `echo_canceller: EchoCanceller`, `open_output_stream` 과 `input_processing_loop` 에서 사용

**협업 잔재 근거**
- `echo_canceller.rs` L33 에러 메시지: `"livekit audio processor error"` (LiveKit 통화 용도 명시)
- `audio_pipeline.rs` L27-28 주석: `"echo canceller and livekit want 10ms of audio"` (LiveKit 기원 확인)
- `Cargo.toml` patch 소스: `github.com/zed-industries/livekit-rust-sdks` (협업 서버 음성/영상 통화)
- Dokkaebi 에서 `call`·`collab_ui`·`channel` 크레이트 이미 제거 → LiveKit 실사용 경로 0 건
- `audio` 크레이트 외부 사용처(`grep "use audio::"`): `crates/zed/src/main.rs`, `zed.rs`, `visual_test_runner.rs`, `zed/visual_tests.rs`, `agent_ui/src/conversation_view.rs` — 모두 `Sound::AgentDone` 재생 용도

**작업 단계**
- [x] `crates/audio/src/audio_pipeline/echo_canceller.rs` 수정 — `real_implementation` mod 삭제, cfg 가드 제거, fake 단일 구현으로 통일(55 → 13 라인)
- [x] `crates/audio/Cargo.toml` L30-31 `[target...]` 블록 제거
- [x] `Cargo.toml` L576 `libwebrtc = "0.3.26"` 워크스페이스 dep 제거
- [x] `Cargo.toml` L763 `webrtc-sys = "0.3.23"` 워크스페이스 dep 제거
- [x] `Cargo.toml` L834-835 `[patch.crates-io]` livekit-rust-sdks 패치 2 줄 제거
- [x] `Cargo.lock` 재생성 — `cargo check` 실행으로 자동. libwebrtc / webrtc-sys / webrtc-sys-build 항목 전량 삭제 확인

**영향 분석**
- **제거되는 기능**: AEC (Acoustic Echo Cancellation). 마이크 입력을 `open_input_stream` 경로로 가져올 때 스피커에서 재생된 원본 오디오가 함께 녹음되는 현상 억제 로직
- **Dokkaebi 실사용 여부**: `open_input_stream` 호출처 `grep` 결과 0 건(마이크 입력 소비 UI 없음). Dokkaebi 는 통화/녹음/음성 인식 기능 미제공
- **사용자 체감**: 변화 0 (마이크 관련 UI 자체가 없음)
- **바이너리 감축**: libwebrtc 네이티브 라이브러리 링크 제거 → Windows MSVC 빌드 크기 감소 예상(수 MB 단위 추정, 확정 수치는 빌드 후 측정)
- **LiveKit Rust SDK patch 제거**: `[patch.crates-io]` 정리로 공급망 검증 단순화

**검증**
- [x] `cargo check -p audio` 통과 (2m05s, 신규 경고·에러 0)
- [x] `cargo check -p Dokkaebi` 통과 (3m05s, 신규 경고·에러 0 — 기존 agent_ui 72 경고 및 zed main.rs unused import 등은 Phase 10 이전 누적분, 이번 변경 무관)
- [x] `Cargo.lock` grep `libwebrtc|webrtc-sys|webrtc_sys` 결과 0 건 (cargo tree 대체 확인)
- [x] `git grep -i "libwebrtc\|webrtc-sys"` 결과 code 0 건(남은 매치는 plan.md / notes.md 문서 기록과 `.cargo/config.toml`·`crates/zed/build.rs` 의 Linux 전용 주석 2 건 — 상류 호환 유지 정책상 본 범위 외)
- [ ] Dokkaebi 실행 → `Sound::AgentDone` 재생 정상(에이전트 완료 사운드) 수동 확인 — **사용자 몫**

**리스크**: 낮음
- 이미 windows-gnu/freebsd 에서 사용 중인 fake 구현을 MSVC 로 확장하는 형태. API 시그니처 호환 기존 cfg 로 검증됨
- `audio_pipeline.rs` 의 `echo_canceller.clone()` · `process_reverse_stream` · `process_stream` 호출은 fake 구현에서 no-op 반환 → 컴파일·런타임 모두 호환

### 11.11.6. Part 1 문서 갱신
- [x] `notes.md` 최상단 `## 최근 변경` 에 항목 추가
- [x] `assets/release_notes.md` 반영: **제외 확정**. 내부 dead code + 미사용 협업 의존성 제거, 사용자 체감 변화 0 → CLAUDE.md 릴리즈 노트 규칙 "제외 대상: 내부 리팩토링(동작 변화 없음)" 에 해당
- [x] MEMORY 갱신 불필요 (기존 `project_license_cleanup.md` 에 포함되는 후속 정리, 별도 메모리 신설 없음)

### 11.11.7. Part 1 승인 필요 사항 (CLAUDE.md §1단계)
- [x] **구조 변경 / 삭제**: `crates/collab_ui/` 디렉터리 재귀 삭제 — 적용 완료
- [x] **의존성 제거**: `libwebrtc`, `webrtc-sys` workspace deps + `[patch.crates-io]` 2 줄 — 적용 완료
- [x] **기존 동작 변경 가능성**: Windows MSVC EchoCanceller `real_implementation` → `fake_implementation` — 적용 완료 (체감 0 검증)

### 11.11.8. 후속 Phase 후보 목록 (2026-04-24 사용자 확답 반영)

**사용자 확답 (2026-04-24)**
- "설정 > LLM 프로바이더" 의 15 개 항목(Amazon Bedrock, Anthropic, GitHub Copilot Chat, DeepSeek, Google AI, LM Studio, Mistral, Ollama, OpenAI, OpenCode Zen, OpenRouter, Vercel, Vercel AI Gateway, xAI) **전부 사용 중**
- 편집 예측(Tab 자동완성) **사용 중**

**폐기 확정**
- ~~**Phase 12** `edit_prediction*` + `copilot*` 제거~~ — 자동완성·Copilot Chat 사용으로 전면 불가. 의존 그래프(`edit_prediction_ui` → `copilot_ui`/`copilot_chat`, `editor` → `edit_prediction_types`, `language_models` → `copilot*`)상 부분 제거도 비실용
- ~~**Phase 17** Anthropic 외 LLM 프로바이더 선택 제거~~ — 15 개 전부 사용으로 해당 크레이트(`bedrock`/`deepseek`/`google_ai`/`lmstudio`/`mistral`/`ollama`/`open_ai`/`opencode`/`open_router`/`vercel`/`x_ai`/`aws_http_client`) 전부 유지

**진행 가능 / 폐기 Phase**
1. ~~**Phase 13**~~ — cloud 잔재 정리 Step 1/2/3 — ✅ **완료** (2026-04-24). 총 -2,605 줄. §11.12.3~§11.12.12 참조
2. ~~**Phase 14**~~ `vim`/`vim_mode_setting` 제거 — ❌ **폐기 확정** (2026-04-24 사용자 확답 — vim 모드 사용 중). 43,765 줄 규모 크레이트 유지
3. **Phase 15** — `task`/`tasks_ui`/`snippet*` 제거
   - ❌ `task`/`tasks_ui` 부분 폐기 확정 (2026-04-24 사용자 확답 — Task 패널 사용 중). `Shift+Alt+T` 모달·`.zed/tasks.json` 사용
   - `snippet`/`snippet_provider`/`snippets_ui` 부분은 사용자 답변 대기
4. **Phase 16** — 소형 GUI (`journal`, `image_viewer`, `markdown_preview`, `svg_preview`, `web_search`) 선택 제거
   - **사용자 답변 대기**: 각각 사용 여부

### 11.11.9. Part 1 예상 작업 규모
- Step 1 (collab_ui 삭제): 파일 2 개 + 디렉터리 제거, 0.2 세션
- Step 2 (libwebrtc 제거): 파일 3 개 수정 + echo_canceller.rs 축소(약 -40 라인) + Cargo.lock 재생성, 0.5 세션
- 문서 갱신: 0.2 세션
- **총**: 약 1 세션 이내

---

## 11.12. Phase 13 — cloud 잔재 정리 (2026-04-24 계획)

> **성격**: Dokkaebi 독자 최적화. Phase 12·17 폐기 결과 `cloud_api_*` 3 크레이트 통째 제거는 불가, Zed Cloud 전용 dead 경로만 축소.
> **상태**: Step 1 축소판 A 진행 중 (사용자 2026-04-24 승인).

### 11.12.1. 목표
`language_models/src/provider/cloud.rs` 의 Zed Cloud LLM 프로바이더가 스크린샷 "설정 > LLM 프로바이더" 목록에 노출되지 않고 사용자 확답상 실사용 0 임을 확인, 해당 프로바이더 본체 + 동반 ZedDotDev 설정 타입을 제거해 코드베이스 축소.

### 11.12.2. 범위 외 (명시적 제외)
- `cloud_api_client` / `cloud_api_types` / `cloud_llm_client` 크레이트 자체 — 17 개 크레이트가 의존, 제거 불가
- `language_model/src/model/cloud_model.rs` — `LlmApiToken`/`PaymentRequiredError`/`NeedsLlmTokenRefresh` 등 범용 타입. 12 개 파일이 사용 중 (파일명은 오도적), 유지
- `client::SignIn`/`SignOut`/`Reconnect` 액션 — Copilot 로그인 + SSH 원격 + edit_prediction 로그인 공용, 유지
- `ai_onboarding` 크레이트 — Step 2 로 분리
- `client.rs` 의 `ZED_IMPERSONATE`/`ZED_WEB_LOGIN`/`ZED_ADMIN_API_TOKEN` dead 환경변수 — Step 3 로 분리

### 11.12.3. Step 1 축소판 A — Zed Cloud LLM 프로바이더 + ZedDotDev 설정 타입 제거

**파일 삭제**
- [x] `crates/language_models/src/provider/cloud.rs` (1,429 줄) — `git rm` 완료. `CloudLanguageModelProvider`, `CloudLanguageModel`, `ZedDotDevSettings`, `State`, `ModelMode` 전량 제거

**`language_models/src/language_models.rs` 수정**
- [x] L18 `use crate::provider::cloud::CloudLanguageModelProvider;` 제거
- [x] L169-176 `registry.register_provider(Arc::new(CloudLanguageModelProvider::new(...)))` 블록 제거. 함수 시그너처의 `user_store: Entity<UserStore>` 파라미터는 다른 프로바이더가 받지 않으므로 `_user_store` 로 리네이밍해 unused 경고 회피(호출자 API 는 그대로 유지)

**`language_models/src/provider.rs` 수정**
- [x] L3 `pub mod cloud;` 제거

**`language_models/src/settings.rs` 수정**
- [x] L7 import 에서 `cloud::ZedDotDevSettings` 제거
- [x] L31 `pub zed_dot_dev: ZedDotDevSettings` 필드 제거
- [x] L53 `let zed_dot_dev = language_models.zed_dot_dev.unwrap();` 라인 제거
- [x] L127-129 default 구성의 `zed_dot_dev: ZedDotDevSettings { ... }` 블록 제거

**`settings_content/src/language_model.rs` 수정**
- [x] L26-27 `#[serde(rename = "zed.dev")]` + `pub zed_dot_dev: Option<ZedDotDevSettingsContent>` 필드 2 줄 제거
- [x] L382-423 `ZedDotDevSettingsContent`, `ZedDotDevAvailableModel`, `ZedDotDevAvailableProvider` 타입 정의 3 개(약 42 줄) 제거

**`assets/settings/default.json` 수정**
- [x] L2307 `"zed.dev": {}` 섹션 1 줄 제거. 주변 JSON 구조(앞: `"x_ai": {...},`, 뒤: `},` language_models 섹션 닫힘) 문법 정상 확인
- [x] L1002, L1025 edit_predictions.provider 예시 주석의 `"zed.dev"` 는 건드리지 않음(edit_prediction 보존 결정에 따라 provider 목록 자체는 별도 조사 대상, 본 Step 범위 외)

**panic 동반 처리**: `settings.rs` L53 `zed_dot_dev.unwrap()` 과 `default.json` L2307 `"zed.dev": {}` 를 **같은 세션에서 함께 제거**하여 panic 회피.

### 11.12.4. 검증
- [x] `cargo check -p settings_content` 통과 (26.91s, 신규 경고·에러 0)
- [x] `cargo check -p language_models` 통과 (2m 09s, 신규 경고·에러 0 — 남은 경고는 Phase 10 이전 누적 `workspace` try_next deprecated 등)
- [x] `cargo check -p Dokkaebi` 통과 (2m 00s, 신규 경고·에러 0 — 남은 8 개 경고 모두 기존 누적분)
- [ ] Dokkaebi 실행 → 설정 > LLM 프로바이더 패널 열어 14 개 프로바이더(Bedrock/Anthropic/Copilot Chat/DeepSeek/Google AI/LM Studio/Mistral/Ollama/OpenAI/OpenCode Zen/OpenRouter/Vercel/Vercel AI Gateway/xAI) 정상 렌더 수동 확인 — **사용자 몫**

### 11.12.5. 문서 갱신
- [x] `notes.md` 최상단 항목 추가
- [x] `assets/release_notes.md`: **제외 확정** (내부 dead 경로 정리, 사용자 체감 변화 0)

### 11.12.6. 예상 규모
-1,430 ~ -1,700 줄, 6 파일 수정 + 1 파일 삭제. 0.3~0.5 세션.

### 11.12.7. 후속 Step 후보
- Step 2 (**진행 중**): `ai_onboarding` Zed 구독·업셀 UI 전면 제거 (옵션 A 확정)
- Step 3 (별도 승인): `client.rs` ZED_IMPERSONATE/ZED_WEB_LOGIN/ZED_ADMIN_API_TOKEN dead 환경변수 제거

### 11.12.8. Step 2 옵션 A — ai_onboarding 업셀 UI 전면 제거 (2026-04-24 승인)

**대상 범위 확정 (조사 완료)**
사용자 확답상 Dokkaebi `server_url=""` 로 Zed cloud 비활성, Plan 진입 불가 → 모든 업셀 UI 가 dead 또는 broken UX(작동 안 하는 "Try Pro for Free" 버튼 노출). `ZedPredictModal` 은 `ZedPredictUpsell::dismissed` 플래그로 한 번만 표시, 이미 설정 완료한 사용자에게는 영향 0. 새 사용자는 broken UX 대신 Copilot 설정 버튼만 노출되어 오히려 개선.

**파일 삭제 (5 + 1 = 6 파일)**
- [x] `crates/ai_onboarding/src/ai_upsell_card.rs` (408 줄) — Pro vs Free 비교 카드
- [x] `crates/ai_onboarding/src/young_account_banner.rs` (22 줄) — `billing-support@zed.dev` 하드코딩 포함 30일 미만 계정 경고
- [x] `crates/ai_onboarding/src/agent_panel_onboarding_card.rs` (83 줄)
- [x] `crates/ai_onboarding/src/agent_panel_onboarding_content.rs` (89 줄) — `AgentPanelOnboarding` 본체
- [x] `crates/ai_onboarding/src/plan_definitions.rs` (56 줄) — 외부 참조 0
- [x] `crates/agent_ui/src/ui/end_trial_upsell.rs` (115 줄) — Pro 체험 종료 유도 카드

**대폭 축소**
- [x] `crates/ai_onboarding/src/ai_onboarding.rs` (383 → 5 줄) — `ZedAiOnboarding` 구조체·render_* 메서드 전부·`SignInStatus` enum·Component impl 제거. `EditPredictionOnboarding` / `ApiKeysWithProviders` / `ApiKeysWithoutProviders` re-export 만 유지

**시그너처 축약**
- [x] `crates/ai_onboarding/src/edit_prediction_onboarding_content.rs` (81 → 60 줄) — `cloud_api_types::Plan`·`ZedAiOnboarding` import 제거, `EditPredictionOnboarding::new()` 에서 `continue_with_zed_ai: Arc<dyn Fn...>` 파라미터 제거, `is_free_plan` 조건 제거(Copilot 버튼 항상 노출), render 에서 `ZedAiOnboarding` child 제거. 안내 문구 "Alternatively..." 제거하고 "You can use GitHub Copilot..." 단순 문구로 교체
- [x] `crates/edit_prediction/src/onboarding_modal.rs` — `continue_with_zed_ai` 콜백 정의(L63-73) 제거, `EditPredictionOnboarding::new(...)` 호출에서 해당 인자 제거 (4 번째 인자)

**agent_ui 광범위 정리**
- [x] `crates/agent_ui/src/agent_ui.rs` `ResetTrialUpsell`, `ResetTrialEndUpsell` action 정의 제거
- [x] `crates/agent_ui/src/ui.rs` `mod end_trial_upsell;` + `pub use end_trial_upsell::*;` 제거
- [x] `crates/agent_ui/src/agent_panel.rs` 전면 정리 완료:
  - L33 import 목록에서 `ResetTrialEndUpsell, ResetTrialUpsell` 제거
  - L40 `ui::EndTrialUpsell` import 제거
  - L57 `use ai_onboarding::AgentPanelOnboarding;` 제거
  - L278-290 `ResetTrialUpsell`/`ResetTrialEndUpsell` action 핸들러 2 개 블록 제거
  - `onboarding: Entity<AgentPanelOnboarding>` 필드 제거
  - `on_boarding_upsell_dismissed: AtomicBool` 필드 제거
  - `AgentPanelOnboarding::new(...)` 생성 블록(`let weak_panel` 포함) 제거
  - `onboarding,` shorthand 필드 초기화 제거 (L1023)
  - `on_boarding_upsell_dismissed: AtomicBool::new(OnboardingUpsell::dismissed(cx))` 초기화 제거
  - `should_render_trial_end_upsell` · `should_render_onboarding` · `render_onboarding` · `render_trial_end_upsell` 메서드 4 개 전부 제거 (약 85 줄)
  - `.children(self.render_onboarding(window, cx))` 렌더 제거
  - `if !self.should_render_onboarding(cx) && let Some(err) = ...` 를 `if let Some(err) = ...` 로 축약
  - `.children(self.render_trial_end_upsell(...))` 렌더 제거
  - `struct OnboardingUpsell` + `struct TrialEndUpsell` + Dismissable impl 제거

**범위 외 (이번 Step 에서 건드리지 않음)**
- `crates/agent_ui/src/agent_configuration.rs` L513-517 `Plan::Dokkaebi*` 매치 — 에이전트 패널 하단의 현재 플랜 칩 UI. Dokkaebi 에서 dead 이지만 Plan enum 자체 제거는 별도 Phase 필요. 이번엔 유지
- `crates/ai_onboarding/src/agent_api_keys_onboarding.rs` (149 줄) — `ApiKeysWithProviders`/`ApiKeysWithoutProviders`. 현재 외부 사용처 0 이지만 향후 API 키 입력 UI 재활용 가능성 감안 유지. ai_onboarding 크레이트 자체도 유지
- `crates/agent_ui/Cargo.toml` L34 `ai_onboarding.workspace = true` — agent_ui 에서 `EditPredictionOnboarding` 사용 없으므로 제거 가능하나 추가 조사 후 결정

### 11.12.9. Step 2 검증
- [x] `cargo check -p ai_onboarding` 통과 (47.49s, 신규 경고·에러 0)
- [x] `cargo check -p agent_ui` 통과 (초기 E0425/E0560 2 건 — `AgentPanel` 구조체의 `onboarding,` shorthand 필드 초기화자 L1023 누락 발견, 추가 제거 후 재검증 통과 7.68s, 신규 경고·에러 0. `user_store` 필드가 이제 never-read 경고 1 건 추가 — agent_panel 내 다른 경로에서 쓰일 수도 있어 이번엔 유지)
- [x] `cargo check -p Dokkaebi` 통과 (12.28s, 신규 경고·에러 0 — 남은 8 개 경고 모두 Phase 10 이전 누적분)
- [ ] Dokkaebi 실행 → 에이전트 패널 정상 진입 + 에디트 예측 모달(조건 부합 시) `ZedAiOnboarding` 없이 Copilot 설정 안내만 노출 수동 확인 — **사용자 몫**

### 11.12.10. Step 2 문서 갱신
- [x] `notes.md` 최상단 항목 추가
- [x] `assets/release_notes.md`: **제외 확정** (내부 dead UI 정리, 체감 변화는 broken UX 제거 수준)

### 11.12.11. Step 2 예상 규모
- 6 파일 삭제 + 7~8 파일 수정. 약 **-1,050 줄** (실측). 1 세션.

## 11.13. Phase 15 — `snippets_ui` 제거 (2026-04-24 옵션 A)

> **성격**: 사용자 정의 snippet 편집 UI 만 제거. `snippet` (LSP placeholder 파서 - 필수), `snippet_provider` (사용자 정의 JSON snippet 로드, project.rs 의존성) 는 유지.
> **상태**: 계획 작성, 승인 완료 (2026-04-24).

### 11.13.1. 배경·판단 근거
- 사용자 확답: "LSP 자동완성만 사용, 사용자 정의 snippet 안 씀" (2026-04-24)
- 조사 결과 `snippet` 크레이트는 LSP completion 의 `${1:name}`/`$0` placeholder 파싱 엔진으로 `editor.rs`·`project/lsp_store.rs`·`languages/rust.rs` 에서 필수 사용 → **제거 불가**
- `snippet_provider` 는 project.rs 가 의존 + 사용자 정의 snippet 로드 담당 → 제거 시 project.rs 수정 필요, 가성비 낮음 → **유지**
- `snippets_ui` (364 줄) 는 snippet 편집 UI 단독 크레이트. `main.rs:787` `snippets_ui::init(cx)` 호출 1 곳만 외부 참조. **제거 가능**

### 11.13.2. 작업 단계
- [x] `crates/snippets_ui/` 디렉터리 `git rm -r` 제거 (1 파일, 364 줄)
- [x] `crates/zed/src/main.rs` L787 `snippets_ui::init(cx);` 호출 제거
- [x] `crates/zed/Cargo.toml` L184 `snippets_ui.workspace = true` 의존성 제거
- [x] 루트 `Cargo.toml`:
  - L169 `"crates/snippets_ui",` workspace members 항목 제거
  - L411 `snippets_ui = { path = "crates/snippets_ui" }` path dep 제거
  - L889 `snippets_ui = { codegen-units = 1 }` profile 설정 제거
- [x] i18n 키 제거:
  - `assets/locales/ko.json` L1273-L1274 `"action.snippets::ConfigureSnippets"` + `"action.snippets::OpenFolder"` 2 건 제거 (초기 조사에서 ko.json 의 OpenFolder 키 누락했으나 작업 시 확인·제거)
  - `assets/locales/en.json` L2736-2737 `"action.snippets::ConfigureSnippets"` + `"action.snippets::OpenFolder"` 제거

### 11.13.3. 검증
- [x] `cargo check -p Dokkaebi` 통과 (4.02s 증분, 신규 경고·에러 0 — 남은 8 경고 모두 Phase 10 이전 누적분)
- [ ] 명령 팔레트에서 "snippets: configure snippets" 가 더 이상 노출되지 않음 수동 확인 — **사용자 몫**

### 11.13.4. 범위 외
- `crates/snippet` (334 줄) — LSP 자동완성 필수 인프라, 유지
- `crates/snippet_provider` (870 줄) — project.rs 의존, 유지
- `crates/task` / `crates/tasks_ui` — Task 패널 사용 중(2026-04-24 사용자 확답), 유지

### 11.13.5. 예상 규모
약 **-370 줄**, 6 파일 수정 + 1 디렉터리 삭제. 0.3 세션.

---

## 11.14. Phase 16 — 소형 GUI 선택 제거 (2026-04-24)

> **성격**: 사용자 미사용 확답 받은 소형 GUI 기능 제거. `image_viewer`/`markdown_preview`/`svg_preview` 는 **유지** (사용자 확답상 사용). `journal` + `web_search`/`web_search_providers` (및 에이전트 web_search tool) 만 제거.
> **상태**: 계획 작성, 사용자 승인 완료 (2026-04-24 — 1:O/2:X/3:X/4:X/5:O).

### 11.14.1. 배경·판단 근거
- 사용자 답변 (2026-04-24): journal 제거 승인, image_viewer/markdown_preview/svg_preview 사용 중 유지, web_search 제거 승인
- `journal` (300 줄): `NewJournalEntry` 액션 1 개, 명령 팔레트로만 실행. 외부 참조는 `main.rs::init` + `zed.rs` 로그 스코프 + `settings_content` 필드 3 곳
- `web_search` (72 줄) + `web_search_providers` (186 줄): Zed Cloud 웹 검색 API 레지스트리. Phase 13 Step 1 에서 Zed Cloud LLM 프로바이더 제거로 이미 broken 상태. 에이전트 `WebSearchTool`(186 줄) 이 이 레지스트리 사용 → 함께 제거 필요

### 11.14.2. Step A — `journal` 제거

**파일 삭제**
- [x] `crates/journal/` 디렉터리 전체 (`journal.rs` 300 줄 + `Cargo.toml` + `LICENSE-GPL`)

**수정**
- [x] `crates/zed/src/main.rs` L821 `journal::init(app_state.clone(), cx);` 제거
- [x] `crates/zed/src/zed.rs` L5118 로그 스코프 `"journal",` 한 줄 제거
- [x] `crates/zed/Cargo.toml` L132 `journal.workspace = true` 제거
- [x] 루트 `Cargo.toml`: workspace members `"crates/journal"` + path dep + `[profile.dev.package.journal]` (L866 `journal = { codegen-units = 1 }`) 3 곳 제거
- [x] `crates/settings_content/src/settings_content.rs` L160 `pub journal: Option<JournalSettingsContent>` 필드 제거 + L939-959 `JournalSettingsContent` 구조체 + `HourFormat` enum 제거
- [x] `crates/settings/src/vscode_import.rs` L196 `journal: None,` 초기화 제거 (조사에서 누락, 빌드 시 E0560 발견 후 추가 수정)
- [x] `assets/settings/default.json` L1658-1667 `"journal": { path, hour_format }` 섹션 10 줄 제거
- [x] `assets/locales/ko.json` L1077 + `en.json` L2540 `"action.journal::NewJournalEntry"` i18n 키 2 건 제거

**panic 동반 처리**: journal 크레이트가 `content.journal.clone().unwrap()` 호출 → 크레이트 자체 삭제 + settings_content 필드 제거를 같은 세션에 처리하므로 panic 위험 없음

### 11.14.3. Step B — `web_search` / `web_search_providers` / 에이전트 WebSearchTool 제거

**파일 삭제**
- [x] `crates/web_search/` 디렉터리 전체 (72 줄)
- [x] `crates/web_search_providers/` 디렉터리 전체 (186 줄)
- [x] `crates/agent/src/tools/web_search_tool.rs` (186 줄)

**수정 (agent)**
- [x] `crates/agent/src/tools.rs` L25 `mod web_search_tool;`, L51 `pub use web_search_tool::*;`, L140 `WebSearchTool,` 등록 3 곳 제거
- [x] `crates/agent/src/thread.rs` L7 import 에서 `WebSearchTool` 제거, L1555 `self.add_tool(WebSearchTool);` 호출 제거
- [x] `crates/agent/src/tests/mod.rs` L6104-6145 `test_web_search_tool_deny_rule_blocks_search` 테스트 제거 (41 줄)
- [x] `crates/agent/Cargo.toml` `web_search.workspace = true` 제거

**수정 (settings_ui)**
- [x] `crates/settings_ui/src/pages.rs` L15 `render_web_search_tool_config` re-export 제거
- [x] `crates/settings_ui/src/pages/tool_permissions_setup.rs`:
  - L71-76 `ToolInfo { id: "web_search", ... }` 블록 제거 (6 줄)
  - L312 `"web_search" => render_web_search_tool_config,` 매핑 제거
  - L1392 `tool_config_page_fn!(render_web_search_tool_config, "web_search");` 호출 제거

**수정 (zed 바이너리)**
- [x] `crates/zed/src/main.rs` L714-715 `web_search::init(cx);` + `web_search_providers::init(...)` 2 줄 제거
- [x] `crates/zed/src/zed.rs` L5352 · L5354 테스트 init 2 줄 제거
- [x] `crates/zed/Cargo.toml` `web_search.workspace = true`, `web_search_providers.workspace = true` 2 개 제거

**수정 (루트 Cargo.toml)**
- [x] workspace members `"crates/web_search"`, `"crates/web_search_providers"` 2 개 제거
- [x] path dep `web_search = { path = ... }`, `web_search_providers = { path = ... }` 2 개 제거

### 11.14.4. 검증
- [x] `cargo check -p Dokkaebi` 통과 (1m 14s, 신규 경고·에러 0 — 남은 경고 8 건 모두 Phase 10 이전 누적분). 초기 E0560 `settings_content::SettingsContent has no field named journal` 발견 후 `vscode_import.rs` L196 추가 수정으로 해결
- [ ] Dokkaebi 실행 — 설정 UI 의 도구 권한 페이지에서 "Web Search" 항목 사라짐 · 명령 팔레트에서 "journal: new journal entry" 사라짐 수동 확인 — **사용자 몫**

### 11.14.5. 문서 갱신
- [x] `notes.md` 최상단 항목 추가
- [x] `assets/release_notes.md`: **제외 확정** (내부 정리, 체감 변화는 사용 안 하던 기능 항목 감소 수준)

### 11.14.6. 범위 외 (유지)
- `crates/image_viewer` — 이미지 미리보기 (사용자 사용 중)
- `crates/markdown_preview` — 마크다운 미리보기 (사용자 사용 중)
- `crates/svg_preview` — SVG 미리보기 (사용자 사용 중)

### 11.14.7. 예상 규모
약 **-950 ~ -1,100 줄**, 3 디렉터리 삭제 + 10~12 파일 수정. 1 세션.

---

## 11.15. Phase 17 — Plan / ZED_CLOUD_PROVIDER_ID / start_trial_url dead 정리 (2026-04-24 승인)

> **성격**: 기 완료 Phase 13 Step 1 후속 — Zed Cloud 프로바이더 제거 이후 유효하지 않게 된 Plan 체크·provider ID 비교·trial URL 경로 정리
> **상태**: 계획 작성, 사용자 승인 완료 (2026-04-24)

### 11.15.1. 근거
- Dokkaebi `server_url=""` + Phase 13 Step 1 로 `CloudLanguageModelProvider` 제거 → `ZED_CLOUD_PROVIDER_ID="zed.dev"` 로 등록된 프로바이더 0
- `user_store.plan_info` 채움 경로(`get_authenticated_user` 응답) 실제 동작 불가 → `plan()` 은 `None` 고정
- `render_zed_plan_info`, `is_provided_by_zed`, auto_retry 분기, Copilot onboarding 필터 모두 dead

### 11.15.2. 대상

**Step A — `render_zed_plan_info` + `is_zed_provider` 분기**
- [x] `agent_ui/src/agent_configuration.rs` L27 `ZED_CLOUD_PROVIDER_ID` import 제거
- [x] L221-235 `is_zed_provider`/`current_plan`/`is_signed_in` 로컬 변수 블록 제거 (user_store.plan() 호출 포함)
- [x] L284-300 `.map(|this| { if is_zed_provider && is_signed_in { ... } else { ... } })` → `.when(provider.is_authenticated(cx) && !is_expanded, ...)` 로 단순화
- [x] L496-527 `render_zed_plan_info` 메서드 + `Plan::Dokkaebi*` 5 개 매치 제거

**Step B — `is_provided_by_zed` 메서드 제거**
- [x] `language_model/src/registry.rs` L103-105 `is_provided_by_zed()` 메서드 제거

**Step C — `handle_completion_error` plan 의존 제거**
- [x] `agent/src/thread.rs` L2133-2141 `auto_retry` 분기 + `if !auto_retry { return Err }` 블록 제거 (항상 true 로 수렴, let Some(model) = ... 도 `if self.model.is_none() { return Err }` 로 단순화)
- [x] `handle_completion_error` 함수 시그너처에서 `plan: Option<Plan>` 파라미터 제거
- [x] L2061 호출자에서 `user_store.plan()` 인자 제거, 클로저가 `user_store` 더 이상 필요없어 `_cx` 로 축소
- [x] L22 `cloud_api_types::Plan` import 제거 (이 파일 다른 사용처 0)
- [x] L42 `ZED_CLOUD_PROVIDER_ID` import 제거 (다른 사용처 0)

**Step D — Copilot onboarding 필터 단순화**
- [x] `ai_onboarding/src/agent_api_keys_onboarding.rs` L35 필터 `provider.is_authenticated(cx)` 만 남김
- [x] L2 `ZED_CLOUD_PROVIDER_ID` import 제거

**Step E — `start_trial_url` 함수 제거**
- [x] `client/src/zed_urls.rs` `start_trial_url` 함수 4 줄 제거

### 11.15.3. 검증
- [x] `cargo check -p Dokkaebi` 통과 (1m 06s, 신규 경고·에러 0 — 남은 8 경고 모두 Phase 10 이전 누적분)

### 11.15.4. 범위 외 (이번에 건드리지 않음)
- `Plan` enum 정의 자체 (`cloud_api_types::plan.rs`) — `client/src/user.rs`·`conversation_view/thread_view.rs:2129` 에서 여전히 참조 (thread.plan() 별도 메서드)
- `user_store.plan()` 메서드 자체 — `agent_ui/conversation_view/thread_view.rs:2129` 등 다른 경로 사용 유지
- `ZED_CLOUD_PROVIDER_ID` 상수 정의 (`language_model/src/language_model.rs:61`) — 다른 모듈 import 확인 후 별도 Phase 결정

### 11.15.5. 예상 규모
약 **-140 줄**, 5 파일 수정. 0.3 세션.

---

## 11.16. Phase 20 — `feedback` 크레이트 전면 삭제 (2026-04-24 승인)

> **성격**: URL 비활성화(empty string)된 feedback 액션·i18n·크레이트 전면 제거. 명령 팔레트에서 "feedback: email/file bug/request feature" 항목 완전 제거
> **상태**: 계획 작성, 사용자 승인 완료 (2026-04-24)

### 11.16.1. 근거
- `crates/feedback/src/feedback.rs` — 모든 URL 이 empty string (`ZED_REPO_URL=""`, `REQUEST_FEATURE_URL=""`, `file_bug_report_url→""`, `email_zed_url→""`)
- `register_action` 으로 4 개 액션 등록 중이나 실행 시 빈 URL 이라 아무 일도 일어나지 않음
- 명령 팔레트에는 "feedback: email dokkaebi/file bug report/request feature" 계속 노출 → broken UX
- 1 인 개인 앱에서 Zed 공식 피드백 창구 필요 없음

### 11.16.2. 대상

**파일 삭제**
- [x] `crates/feedback/` 디렉터리 전체 (`feedback.rs` 82 줄 + Cargo.toml + LICENSE-GPL)

**`crates/zed_actions/src/lib.rs` 수정**
- [x] L315-329 `pub mod feedback { actions!(feedback, [EmailZed, FileBugReport, RequestFeature]); }` 블록 전체 제거

**`crates/zed/src/main.rs` 수정**
- [x] `feedback::init(cx);` 호출 제거

**`crates/zed/Cargo.toml` 수정**
- [x] `feedback.workspace = true` 제거

**`crates/workspace/src/workspace.rs` 수정**
- [x] `use zed_actions::{feedback::FileBugReport, theme::ToggleMode}` → `use zed_actions::theme::ToggleMode` 로 단순화
- [x] DB 로드 실패 토스트의 `.primary_message(button).primary_icon(IconName::Plus).primary_on_click(|window, cx| { window.dispatch_action(Box::new(FileBugReport), cx) })` 블록 제거, `MessageNotification::new(message, cx)` 만 유지. `let button = i18n::t("workspace.file_issue", cx);` 도 함께 제거

**루트 `Cargo.toml` 수정**
- [x] workspace members `"crates/feedback"` + path dep 제거

**i18n 키 제거**
- [x] `ko.json` 3 건 (`EmailZed`/`FileBugReport`/`RequestFeature`) + `workspace.file_issue` 1 건 총 4 건 제거
- [x] `en.json` 동일 4 건 제거

**체크 (영향 없음)**
- [x] `CopySystemSpecsIntoClipboard` 액션은 `system_specs` 크레이트에 독립 정의, 영향 없음
- [x] `dokkaebi.OpenZedRepo` 액션은 `feedback.rs` 에서만 정의·등록되었으므로 크레이트 삭제로 자동 제거

### 11.16.3. 검증
- [x] `cargo check -p Dokkaebi` 통과 (1m 06s 초기 빌드 + 2.17s 증분, 신규 경고·에러 0)
- [ ] Dokkaebi 실행 → 명령 팔레트에서 "feedback:" 항목이 사라짐 수동 확인 — **사용자 몫**

### 11.16.4. 예상 규모
약 **-150 줄**, 1 디렉터리 삭제 + 6 파일 수정. 0.3 세션.

---

## 11.17. Phase 18 — Workspace 비참조 독립 바이너리/라이브러리 정리 (2026-04-24 옵션 A)

> **성격**: Dokkaebi 앱 바이너리에 포함되지 않고 다른 크레이트에서도 참조 0 인 9 개 크레이트 제거. `cargo build --workspace` 시간 단축이 주 효과 (앱 크기 영향 0)
> **상태**: 계획 작성, 사용자 승인 완료 (2026-04-24 옵션 A)

### 11.17.1. 대상 (전수 grep 검증 완료 — 외부 참조 0)

**Orphan 디렉터리 (workspace 비소속)**
- `crates/debugger_ui/` — Cargo.toml 부재, workspace members 목록에도 없음. `src/debugger_panel.rs` 1 파일만 존재. Phase 11 `collab_ui` 와 동일한 dead directory

**독립 바이너리**
- `crates/storybook` — Zed UI 컴포넌트 Storybook (`[[bin]] name = "storybook"`, ~1,229 줄)
- `crates/theme_importer` — VS Code 테마 → Dokkaebi 테마 마이그레이션 CLI (`main.rs` + `vscode/` 디렉터리, ~820 줄)
- `crates/schema_generator` — JSON schema 생성 CLI (43 줄)
- `crates/fs_benchmarks` — fs 크레이트 벤치마크 (34 줄)
- `crates/project_benchmarks` — project 크레이트 벤치마크 (233 줄)
- `crates/worktree_benchmarks` — worktree 크레이트 벤치마크 (52 줄)
- `crates/extension_cli` — 확장 개발자용 CLI (`[[bin]] name = "zed-extension"`)

**외부 publish 라이브러리 — ⚠️ 유지 결정 (작업 중 발견)**
- `crates/extension_api` — **`extension_host/build.rs` 가 `../extension_api/wit` 디렉터리의 `.rs` 파일을 `OUT_DIR` 로 복사하는 경로 의존**. 초기 grep 검증이 `.workspace = true` 와 `use xxx::` 만 확인해 이 파일 시스템 경로 참조를 놓침. 삭제 후 빌드 실패(`Os { code: 3, kind: NotFound }`) 로 발견 → 디렉터리 복원 및 workspace members 복귀

### 11.17.2. 작업 단계

**디렉터리 삭제 (8 — 초기 9 → extension_api 복원으로 8)**
- [x] `git rm -r crates/debugger_ui` (orphan, Cargo.toml 없음)
- [x] `git rm -r crates/storybook crates/theme_importer crates/schema_generator`
- [x] `git rm -r crates/fs_benchmarks crates/project_benchmarks crates/worktree_benchmarks`
- [x] `git rm -r crates/extension_cli`
- [x] `crates/extension_api/` — **유지 (복원)**

**루트 `Cargo.toml` workspace members 7 항목 제거** (extension_api 는 유지)
- [x] `"crates/extension_cli"`, `"crates/fs_benchmarks"`, `"crates/project_benchmarks"`, `"crates/schema_generator"`, `"crates/storybook"`, `"crates/theme_importer"`, `"crates/worktree_benchmarks"` 제거
- [x] `"crates/extension_api"` 복원 (빌드 의존성 때문)

**path dep / profile 설정**: 확인 후 수정 불필요 (해당 크레이트 등록 0 건)

### 11.17.3. 검증
- [x] `cargo check -p Dokkaebi` — 1 차 시도 시 extension_host/build.rs 의 `../extension_api/wit` 경로 부재로 빌드 실패 → extension_api 복원 후 재시도 통과 (12.79s, 신규 경고·에러 0)

### 11.17.4. 실제 감축 규모
8 디렉터리 삭제 + Cargo.toml workspace members 7 줄 제거. 약 **-3,000 ~ -5,000 줄**. 0.3 세션.

### 11.17.5. 교훈
`*.workspace = true` Cargo 의존성 grep 만으로는 **빌드 스크립트의 파일 시스템 경로 참조를 검증할 수 없음**. 향후 유사 작업 시 각 대상 크레이트에 대해 다음 추가 확인 필요:
- 다른 크레이트의 `build.rs` 에서 `../<target_name>` 경로 참조 grep
- `include_str!("...")` / `include_bytes!("...")` 매크로 경로 참조 grep

### 11.17.5. 상류 Zed 동기화 영향
이후 Zed 가 이 크레이트들에 변경을 가해도 Dokkaebi 는 해당 디렉터리 자체가 없으므로 cherry-pick 시 자동으로 "파일 부재 → skip" 분류. 상류 호환 유지 정책(CLAUDE.md) 의 "Dev Container"/"REPL" 제거와 동일한 패턴.

---

## 11.18. Phase 21 — user.rs 의 Plan 관련 dead 메서드/이벤트 정리 (2026-04-24 옵션 C)

> **성격**: `cloud_api_types::Plan`/`PlanInfo` enum·구조체와 `cloud_api_types::GetAuthenticatedUserResponse` API 스키마는 유지(상류 Zed 호환). `user.rs` 내부의 dead 메서드·이벤트만 선별 제거
> **상태**: 계획 작성, 사용자 승인 완료 (2026-04-24 옵션 C)

### 11.18.1. 대상 (외부 호출처 0 확인)

- [x] `crates/client/src/user.rs` L729-731 `pub fn plan_for_organization` 메서드 제거 — 외부 호출 0, user.rs 내부 `plan()` 에서 자기 호출만
- [x] `crates/client/src/user.rs` L733-755 `pub fn plan()` 메서드 제거 (ZED_SIMULATE_PLAN 디버그 분기 포함) — Phase 17 이후 외부 호출 0 확인
- [x] `crates/client/src/user.rs` L145 `PlanUpdated` 이벤트 variant 제거 — emit/subscribe 전역 0 건

### 11.18.2. 범위 외 (유지)
- `cloud_api_types::Plan` enum · `PlanInfo` 구조체 · `GetAuthenticatedUserResponse.plan` 필드 — cloud API 스키마, 상류 Zed 호환
- `client/src/user.rs` 의 `plan_info`·`plans_by_organization` 필드, `account_too_young()`·`has_overdue_invoices()`·`subscription_period()`·`trial_started_at()` 메서드 — `edit_prediction_ui` · `edit_prediction/zed_edit_prediction_delegate.rs` 등에서 여전히 사용 중 (항상 false/None 반환 경로지만 호출처 존재)
- `thread.plan()` (thread_view.rs:2129) — `AcpThread::plan()` 은 `acp::Plan` (TODO 리스트용) 으로 본 Plan enum 과 별개, 건드리지 않음
- `client/src/test.rs` fixture — `PlanInfo`/`Plan` 유지 필요 (GetAuthenticatedUserResponse mock)

### 11.18.3. 검증
- [x] `cargo check -p client` 통과 (21.61s, 신규 경고·에러 0)
- [x] `cargo check -p Dokkaebi` 통과 (31.80s, 신규 경고·에러 0)

### 11.18.4. 예상 규모
약 **-26 줄**, 1 파일 수정. 0.1 세션.

---

## 11.19. Phase 22 — submit_agent_feedback UI + cloud_api_client 메서드 정리 (2026-04-24 승인)

> **성격**: Dokkaebi cloud 비활성으로 실제 전송 dead 인 에이전트 피드백 전송 경로 제거
> **상태**: 계획 작성

### 11.19.1. 대상 (옵션 2 — UI 경로만 정리, cloud_api_client/types 는 유지)

**conversation_view.rs**
- [x] `ThreadFeedback` enum 제거 (Positive/Negative variant, 5 줄)

**conversation_view/thread_view.rs**
- [x] L9 `use cloud_api_types::{SubmitAgentThreadFeedbackBody, SubmitAgentThreadFeedbackCommentsBody}` import 제거
- [x] L22-165 `ThreadFeedbackState` 구조체 + impl 전체 제거 (submit/submit_comments/clear/dismiss_comments/build_feedback_comments_editor, 144 줄)
- [x] `thread_feedback: ThreadFeedbackState` 필드 제거
- [x] `thread_feedback: Default::default()` 초기화 제거
- [x] `self.thread_feedback.clear()` 호출 제거
- [x] `comments_editor = self.thread_feedback.comments_editor.clone()` + `.when_some(comments_editor, ...)` 렌더 체이닝 제거
- [x] `render_feedback_feedback_editor` 메서드 전체 제거 (~40 줄)
- [x] `if AgentSettings::get_global(cx).enable_feedback && ...` thumbs up/down 버튼 블록 제거 (~30 줄)
- [x] `render_feedback_button` + `handle_feedback_click` + `submit_feedback_message` 메서드 3 개 제거 (~60 줄)

**edit_prediction.rs**
- [x] L3 import 에서 `SubmitEditPredictionFeedbackBody` 제거
- [x] `rate_prediction` 메서드 내부 `cx.background_spawn({ ... submit_edit_prediction_feedback ... }).detach_and_log_err(cx)` 블록 제거 (~30 줄), `rated_predictions.insert` 와 `cx.notify()` 만 남김. 사용하지 않는 `rating`/`feedback` 파라미터는 `_rating`/`_feedback` 으로 리네이밍

**유지 (옵션 2 범위 외)**
- [x] `cloud_api_client::submit_agent_feedback` / `submit_agent_feedback_comments` / `submit_edit_prediction_feedback` 메서드 — cloud API 스키마 유지
- [x] `cloud_api_types::SubmitAgentThreadFeedbackBody` / `SubmitAgentThreadFeedbackCommentsBody` / `SubmitEditPredictionFeedbackBody` 구조체 — 동일

### 11.19.2. 검증
- [x] `cargo check -p Dokkaebi` 통과 (13.56s, 신규 경고·에러 0)

### 11.19.3. 실제 감축 규모
약 **-350 줄** (conversation_view.rs 5 + thread_view.rs ~315 + edit_prediction.rs ~30)

---

## 11.20. Phase 23 — client.rs cloud 전용 함수 재조사 (2026-04-24 승인)

> **성격**: Phase 13 Step 3 이후 client.rs 에 남은 dead 경로 (예: `connect_to_cloud`, `authenticate_with_browser` 중 일부, `sign_in_with_optional_connect` 경로) 재검증

### 11.20.1. 조사 결과 (2026-04-24)

**실제 dead 발견**
- `connect_to_cloud` (L915-938, 24 줄) — 외부 호출 0, `sign_in_with_optional_connect:966` 에서만 호출되며 `.log_err()` 처리로 server_url="" 실패 무시
- `authenticate_with_browser` 가시성 — 외부 호출 0, 내부 1 곳(L1168)만 → `pub` → `pub(crate)` 축소 가능

**조사했으나 dead 아님**
- `sign_in` / `sign_in_with_optional_connect` / `connect` / `reconnect` / `request_sign_out` / `sign_out` / `disconnect` — 외부 호출 있음 (onboarding, main.rs, cloud_model.rs 등)

### 11.20.2. Phase 23-minimal 작업 단계 (사용자 승인, 2026-04-24)
- [x] `client.rs` L915-938 `connect_to_cloud` 메서드 제거
- [x] `client.rs` L966 `self.connect_to_cloud(cx).await.log_err();` 호출 제거
- [x] `client.rs` L1329 `authenticate_with_browser` 가시성 `pub` → `pub(crate)` 축소

### 11.20.3. 검증
- [x] `cargo check -p Dokkaebi` 통과 (22.23s, 신규 경고·에러 0)

### 11.20.4. 실제 감축 규모
약 **-25 줄**, 1 파일 수정. 0.1 세션.

---

### 11.12.12. Step 3 — client.rs dead 환경변수 + 분기 제거 (2026-04-24 승인)

**제거 대상 (코드 확인 완료)**
- `crates/client/src/client.rs` L62-74: `IMPERSONATE_LOGIN`(`ZED_IMPERSONATE`), `USE_WEB_LOGIN`(`ZED_WEB_LOGIN`), `ADMIN_API_TOKEN`(`ZED_ADMIN_API_TOKEN`) 세 `pub static LazyLock` 정의 — 외부 크레이트 import 0 건
- L356-358: `if IMPERSONATE_LOGIN.is_some() { return None; }` — impersonate 모드 시 credentials 읽기 스킵 분기. 항상 false → 블록 통째 제거
- L878-883: `if IMPERSONATE_LOGIN.is_none() { write_credentials(...) }` — is_none() 항상 true → `if` 래퍼만 제거, body(write_credentials) 유지
- L1376-1386: `if let Some((login, token)) = IMPERSONATE_LOGIN.as_ref().zip(ADMIN_API_TOKEN.as_ref()) { if !*USE_WEB_LOGIN { authenticate_as_admin(...) } }` — `.zip` 결과 항상 None → 전체 블록 제거
- L1469-1514: `async fn authenticate_as_admin(...)` 메서드 — 호출처 L1383 뿐, L1376 블록 제거 시 dead. 함수 전체(약 46 줄) 제거
- L1475-1484 의 `ImpersonateUserBody` / `ImpersonateUserResponse` 내부 구조체도 연쇄 제거

**작업 단계**
- [x] L62-74 static 3 개 제거 (13 줄)
- [x] L356-358 블록 제거 (3 줄)
- [x] L878-883 `if` 래퍼 제거, body 유지 (들여쓰기 1 단계 축소)
- [x] L1376-1386 블록 제거 (11 줄)
- [x] L1469-1514 `authenticate_as_admin` 함수 제거 (46 줄, `ImpersonateUserBody`/`ImpersonateUserResponse` 내부 구조체 포함)
- [x] 부차 수정: `Ok(Credentials { ... })` 에 `Ok::<Credentials, anyhow::Error>` 타입 힌트 추가 — 제거된 `return this.authenticate_as_admin(...)` 분기가 이전에 타입 inference 를 도왔던 것이 사라져 async 클로저 타입 추론 실패. 명시적 타입 지정으로 E0282/E0283 해결

**검증**
- [x] `cargo check -p client` 통과 (초기 E0282/E0283 8 건 → 타입 힌트 추가 후 2.27s 통과, 신규 경고·에러 0. 남은 15 경고 전부 기존 telemetry dead code 누적분)
- [x] `cargo check -p Dokkaebi` 통과 (35.58s, 신규 경고·에러 0)

**예상 규모**: 약 **-80 줄**, 1 파일 수정, 0.3 세션.

**리스크**: 낮음. 외부 import 0, 모든 사용 분기가 항상 dead 로 수렴하는 상수 조건. `authenticate_as_admin` 은 Dokkaebi `server_url=""` 에서 호출 자체가 불가능한 admin impersonate 경로.

---

## 11.21. Phase 24 — #54224 unsaved scratch buffer 세션 유지 (2026-04-24 계획)

> **성격**: v0.233.5 보류 12건 중 사용자 가치 명확 + 미적용 단독 항목.
> **상태**: 계획 작성, 사용자 승인 대기.

### 11.21.1. 배경 및 결정

**사용자 답변 (2026-04-24)**: tsgo LSP 사용 / 마크다운 미리보기 footnote·anchor 사용 / 임시 버퍼(저장 안 한 새 파일) 사용 / favorite 모델 기능 사용 — 4 가지 모두 사용 중.

**검증 결과 (코드 grep + git log)**: 보류 12건 중 6건이 이미 적용 완료, 1건만 실제 작업 대상.

| PR | 상태 | 검증 근거 |
|---|---|---|
| #54201 tsgo LSP | ✅ 완료 | `git log` 커밋 `16b77fed44 Phase 8F` + `Cargo.toml:560` rev = `f4dfa89a21...` (상류 동일) |
| #54431 ACP replay events drop | ✅ 완료 | `agent_servers/src/acp.rs:1120-1132` pre-register 패턴 + `pending_sessions` 인프라 완비 |
| #53884 action_log race | ✅ 완료 | `action_log/src/action_log.rs:294` `if matches!(event, BufferDiffEvent::BaseTextChanged)` + `buffer_diff.rs:1955,2038` BaseTextChanged emit |
| #53086 마크다운 footnotes | ✅ 완료 | `markdown/src/parser.rs:40,300,504,523` `footnote_definitions`/`FootnoteDefinition`/`FootnoteReference`/`build_footnote_definitions` |
| #53184 마크다운 anchor | ✅ 완료 | `markdown/src/parser.rs:88,156,518` + `markdown.rs:494,646` `heading_slugs`/`build_heading_slugs`/`parse_heading_slugs` |
| #54318 favorite 모델 | ✅ 완료 | `favorite_models.rs:9-35` PR 패턴 적용 + `agent_settings.rs:105` `language_model_to_selection` 헬퍼 + `Speed` 필드 |
| **#54224 scratch buffer** | ❌ 미적용 | `workspace/src/persistence.rs:1710` 여전히 구 이름 `recent_workspaces_on_disk` |

→ **§11.7 보류 기록 6건은 outdated. plan.md 정리 별도 단계 필요.**

### 11.21.2. PR #54224 변경 요약 (코드 검증 완료)

상류 patch: 9 파일, +367/-66. 핵심 = listing 과 garbage collection 의 분리 + 현재/이전 세션 보존.

**핵심 구조 변경 (`crates/workspace/src/persistence.rs`)**

1. **`contains_wsl_path(paths)` 헬퍼 신설** — `cfg!(windows) && WslPath::from_path(...)`. Dokkaebi 가 inline 으로 가지고 있던 로직과 동일.
2. **`recent_workspaces_query` SQL 시그너처 확장**
   - `SELECT workspace_id, paths, paths_order, remote_connection_id, timestamp` → **+ session_id**
   - 반환 5튜플 → 6튜플 (`Option<String>` session_id 추가)
3. **`recent_workspaces` 함수 시그너처 확장** — 5튜플 → 6튜플
4. **`recent_workspaces_on_disk` → 두 함수로 분리**
   - **`recent_project_workspaces(fs)`** (read-only): UI 노출 전용. 빈 경로/WSL/missing dir **제외**, 삭제 안 함
   - **`garbage_collect_workspaces(fs, current_session_id, last_session_id)`** (write): 7일 stale 삭제 + 현재/이전 세션은 항상 보존
5. **`last_workspace`** 호출처 `recent_workspaces_on_disk` → `recent_project_workspaces`
6. **`last_session_workspace_locations` 단순화** — `paths.is_empty() || all_paths_exist(...)` 통일 분기. Dokkaebi 의 "Empty workspace with items" 주석 분기와 동일 의미 (구조만 단순)
7. **테스트 7개 추가** (+227 라인) — `test_scratch_only_workspace_restores_from_last_session`, `test_gc_*`, `test_last_session_*`

**핵심 사용자 가치**

| 시나리오 | 이전 (현재 Dokkaebi) | 이후 (PR 적용) |
|---|---|---|
| 임시 버퍼만 있는 워크스페이스 재시작 | 빈 경로 보존 분기로 일부 살아남으나 불안정 | `last_session_workspace_locations` 가 항상 복원 |
| 임시 버퍼가 recent projects UI 에 표시 | **표시됨** (Dokkaebi 빈 경로 보존 분기) | **표시 안 함** (PR 의도: scratch 는 'project' 가 아님) |
| 현재 세션 워크스페이스가 GC 로 삭제 | 발생 가능 ("Workspace not found") | session_id 비교로 항상 보존 |
| missing path 워크스페이스 | listing 도중 즉시 cascade delete | 7일 grace + listing 과 분리 |

**주의**: PR 적용 시 "임시 버퍼가 recent projects 목록에 더 이상 안 보임" = **사용자 체감 행동 변화**. 사용자 의도 확인 필요.

### 11.21.3. Dokkaebi 9 파일 영향 매핑 (전수 확인)

| 파일 | PR 변경 | Dokkaebi 보유 | 충돌 |
|---|---|---|---|
| `crates/agent_ui/src/thread_metadata_store.rs:191` | 호출처 1 줄 리네임 | ✓ | 없음 |
| `crates/agent_ui/src/threads_archive_view.rs:1078` | 호출처 1 줄 리네임 | ✓ | 없음 |
| `crates/recent_projects/src/recent_projects.rs:96, 610, 2039` | 호출처 3 줄 리네임 | ✓ | 없음 |
| `crates/recent_projects/src/sidebar_recent_projects.rs:73` | 호출처 1 줄 리네임 | ✓ | 없음 |
| `crates/workspace/src/history_manager.rs:47` | 호출처 1 줄 리네임 | ✓ | 없음 |
| `crates/workspace/src/welcome.rs:274` | 호출처 1 줄 리네임 | ✓ | 없음 |
| `crates/workspace/src/persistence.rs` | **+310/-46 핵심** | ✓ (1710 라인 `recent_workspaces_on_disk`) | Dokkaebi 빈 경로 보존 한글 주석 + WSL 처리 → 새 구조에 매핑 (의미 같음) |
| `crates/zed/src/main.rs` | +35/-12 (GC 호출 추가) | ✓ | `restore_or_create_workspace` 호출이 Dokkaebi 에 466·981 두 곳 존재 (상류는 한 곳). 매핑 결정 필요 |
| `crates/zed/src/zed.rs` | +14/-0 테스트 강화 | ✓ (MultiWorkspace 사용 + 동일 테스트 보유) | `database_id()` 검증 1줄 추가, 충돌 가능성 낮음 |

### 11.21.4. 핵심 검증 결과

1. **DB schema 의 `session_id` 컬럼 이미 존재** — `persistence.rs:530` `ALTER TABLE workspaces ADD COLUMN session_id TEXT DEFAULT NULL`. 즉 **schema migration 불필요**, SELECT 에 컬럼 추가만.
2. **Dokkaebi 빈 워크스페이스 보존 한글 주석 (L1740-1744)** — 새 구조의 `last_session_workspace_locations` 가 같은 역할 담당. 의미 보존됨.
3. **Dokkaebi WSL 검사 한글 주석 (L1746-1758)** — `garbage_collect_workspaces` 로 그대로 이동. 한글 주석 보존.
4. **workspace_group panel 충돌 없음** — `workspace_group_panel` 은 별도 테이블/파일, `workspaces` 테이블과 무관. patch 가 `workspaces` 테이블만 다룸.

### 11.21.5. 작업 단계 (순차) — 2026-04-24 완료

**Step 1 — `persistence.rs` 핵심 함수 분리** ✅
- [x] `contains_wsl_path` 헬퍼 함수 신설 (L66 직후)
- [x] `recent_workspaces_query` SELECT 에 `session_id` 추가 + 반환 튜플 6튜플로 확장
- [x] `recent_workspaces` 함수 시그너처 6튜플로 확장
- [x] `recent_workspaces_on_disk` 함수를 두 개로 분리
  - `recent_project_workspaces` (listing only, 빈 경로/WSL 제외)
  - `garbage_collect_workspaces(fs, current_session_id, last_session_id)` (cleanup only)
- [x] `last_workspace` 호출 갱신
- [x] `last_session_workspace_locations` 분기 단순화 (Dokkaebi 한글 주석 보존)
- [x] `set_timestamp_for_tests` query 추가 (테스트 의존성)

**Step 2 — 호출처 6 지점 일괄 리네임** ✅
- [x] `welcome.rs:370`, `history_manager.rs:47`, `sidebar_recent_projects.rs:72`, `recent_projects.rs` (97/588/1773 3 곳) 의 `recent_workspaces_on_disk` → `recent_project_workspaces` 치환
- agent_ui 의 `thread_metadata_store`/`threads_archive_view` 는 Dokkaebi `SidebarThreadMetadataStore` 별개 구조라 호출 0 건 — 적용 대상 외

**Step 3 — `zed/src/main.rs` GC 호출 추가 (옵션 B)** ✅
- [x] `gpui::Task` import 추가
- [x] `app_state.session.read(cx)` 에서 `current_session_id`/`last_session_id` 추출
- [x] `restore_task` 분리 (open_request 분기 + None 분기 모두 Task 반환)
- [x] 별도 `cx.spawn` 으로 `restore_task.await` 후 `garbage_collect_workspaces(...)` 호출
- [x] **옵션 B 채택**: L981 메인 진입점만 GC 호출 추가. L466 `app.on_reopen` 콜백은 미적용(lock 경합 회피)

**Step 4 — `zed/src/zed.rs` 테스트 강화** ✅ (필수로 격상)
- [x] `test_window_edit_state_restoring_enabled` 에 `workspace_database_id` 헬퍼 추가
- [x] 첫 open 직후 `initial_database_id.is_some()` assertion
- [x] close/reopen 후 동일 ID 검증 assertion 추가

**Step 5 — 신규 테스트 7 개 추가 (필수)** ✅
- [x] `pane_with_items`/`empty_pane_group`/`workspace_with` helper 3 개 (Dokkaebi 독자 필드 `workspace_groups`/`active_group_index` 포함)
- [x] 7 테스트: scratch_only_restores / gc_preserves_scratch_inside_window / gc_deletes_stale_outside_window / gc_preserves_directory_with_missing_path / gc_preserves_current_and_last_sessions / gc_deletes_empty_with_items / last_session_restores_with_missing_paths
- [x] **부수 작업**: 기존 `--tests` 빌드가 21 에러로 깨져있던 누적 문제 발견 (Dokkaebi 가 `SerializedWorkspace` 에 `workspace_groups`/`active_group_index` 필드 추가하면서 mod tests fixture 14 곳을 업데이트 누락) → 14 fixture 모두 두 필드 추가

**Step 6 — 검증** ✅
- [x] `cargo check -p workspace` ✅ (57.86s, 신규 경고 0)
- [x] `cargo check -p workspace --tests` ✅ (8.96s, 신규 경고 0)
- [x] `cargo check -p Dokkaebi` ✅ (14.78s, 신규 경고·에러 0)
- [x] `cargo test -p workspace --lib persistence::tests::test_*` ✅ (7 passed; 0 failed; 0 ignored, 0.57s)

**Step 7 — 문서 갱신** ✅
- [x] `notes.md` 최상단에 상세 기록
- [x] `assets/release_notes.md` v0.4.0 `### 버그 수정` 에 옵션 (a) 적용: "임시 버퍼 워크스페이스 재시작 후 복원"
- [x] release_notes.md 섹션 헤더 날짜 (2026-04-23) → (2026-04-24) 갱신 (memory 규칙)

---

## 11.22. Phase 25 — v0.233.5 백포트 마무리 (보류 0 화) (2026-04-24)

> **성격**: 사용자 "보류는 없다, 남은 작업 모두 완료가 목표" 결정에 따른 v0.233.5 백포트 잔여 정리. Phase 24 #54224 적용 후 검토 결과 보류 6 건 중 4 건은 이미 적용/회피됨, 사용자 결정으로 #48003 적용 + #53941 잔여 영구 삭제.
> **상태**: 계획 작성, 사용자 승인 완료 (1 사용 / 2 B / B-1 b / B-2 나).

### 11.22.1. 사용자 결정 (2026-04-24)
- **#48003 HTTP MCP `settings` 필드 제거**: **적용** (HTTP/SSE MCP context server 사용)
- **#53941 잔여 = Phase 10 Part B-2/B-3**: **(B) 영구 삭제** (옵션 β 영구 채택)
- **B-1 picker 의 기존 worktree 항목 처리**: **(b) Existing entry 미표시**
- **B-2 Phase 10 Part B-1 git API 6종**: **(나) 함께 제거**

### 11.22.2. Part A — #48003 HTTP MCP migrator m_2026_04_15 신설 (적용) ✅ 완료

**근거**
- HTTP MCP 사용자 영향: deprecated `settings` 필드가 자동 제거되어 향후 HTTP context server 설정 단순화
- 선행 migrator 4 건(`m_2026_03_30`, `04_01`, `04_10`, `04_17`) 모두 Dokkaebi 에 이미 적용 확인 (`crates/migrator/src/migrations/` 디렉터리 + `migrator.rs:243-258`)
- plan.md §11.7 의 "migrator 5건 선행 필요" 기록 outdated → 4 건 이미 적용, m_2026_04_15 만 신설

**작업 단계**
- [x] `crates/migrator/src/migrations/m_2026_04_15/settings.rs` 신규 파일 작성 (19 줄)
- [x] `crates/migrator/src/migrations.rs` 에 `pub(crate) mod m_2026_04_15` 4 줄 추가
- [x] `crates/migrator/src/migrator.rs` 에 `MigrationType::Json(... remove_settings_from_http_context_servers)` 등록
- [x] `migrator.rs::mod tests` 에 `test_remove_settings_from_http_context_servers` 추가 (~70 줄)

**검증** ✅
- [x] `cargo test -p migrator test_remove_settings_from_http_context_servers` ✅ (1 passed; 0 failed; 0 ignored, 0.04s)
- [x] `cargo check -p Dokkaebi` ✅ (48.84s, 신규 경고·에러 0)

**예상 규모**: +~100 줄, 4 곳 수정. 0.2 세션.

### 11.22.3. Part B — Phase 10 thread_worktree_archive 잔재 영구 삭제 (영구 삭제) ✅ 완료

**근거**
- Phase 10 Part B-1 으로 추가된 git API 6 종 + 비활성 `thread_worktree_archive.rs` 파일 + `SwitchWorktree` action + 토스트 → 모두 dead. 옵션 β 영구 채택 결정으로 정리.
- 옵션 b: picker 에서 기존 linked worktree entry 자체를 미표시 → 사용자가 "기존 worktree 선택" 시도 자체 불가 → 토스트 불필요.

**Part B-1: `thread_worktree_archive.rs` 본체 + 잔재 제거** ✅
- [x] `crates/agent_ui/src/thread_worktree_archive.rs` (1032 라인) `git rm` 으로 파일 삭제
- [x] `crates/agent_ui/src/agent_ui.rs`: import 의 `SwitchWorktree` 제거, Part B-3 TODO 코멘트 + 주석 mod 4 줄 제거, `pub struct SwitchWorktree` action 정의 9 줄 제거
- [x] `crates/agent_ui/src/agent_panel.rs`: import 의 SwitchWorktree 제거, `register_action(SwitchWorktree)` 분기 제거, `switch_to_worktree` 메서드 25 줄 제거

**Part B-2: picker 에서 Existing worktree entry 미표시 (옵션 b)** ✅
- [x] `crates/agent_ui/src/thread_worktree_picker.rs`: `SwitchWorktree` import 제거, `ThreadWorktreeEntry` enum 의 `Worktree`/`Separator` variant 2 종 제거, `update_matches` 의 `Worktree` push 코드 + fuzzy match 로직 + Separator push 모두 제거, `confirm`/`render_match`/`sync_selected_index` 의 dead arm 제거, `all_repo_worktrees` 헬퍼 + `project_worktree_paths` 필드 + 그 init/test helper 인자까지 모두 제거
- [x] `all_worktrees` 필드는 `has_named_worktree` 검사용으로 유지

**Part B-3: i18n 키 정리** ✅
- [x] `assets/locales/ko.json` `agent_panel.switch_worktree.unsupported_toast` 제거
- [x] `assets/locales/en.json` 동일 키 제거

**Part B-4: git API 6 종 제거 (옵션 나)** ✅
- [x] `crates/git/src/repository.rs`: trait 메서드 6 정의 제거 (`create_worktree_detached`/`checkout_branch_in_worktree`/`update_ref`/`delete_ref`/`create_archive_checkpoint`/`restore_archive_checkpoint`)
- [x] `crates/git/src/repository.rs`: RealGitRepository impl 6 메서드 제거 (~150 줄)
- [x] `crates/project/src/git_store.rs`: 6 wrapper 함수 제거 (~110 줄)
- [x] `crates/fs/src/fake_git_repo.rs`: 6 stub 메서드 제거 (~60 줄)
- [x] 외부 호출처 0 재확인 완료 (Phase 10 Part B-1 외 사용 0)

**검증** ✅
- [x] `cargo check -p agent_ui` ✅ (3.89s, 신규 경고 0)
- [x] `cargo check -p git -p project -p fs` ✅ (51.83s, 신규 경고 0)
- [x] `cargo check -p Dokkaebi` ✅ (48.84s, 신규 경고·에러 0)
- [x] **`cargo check -p agent_ui --tests`** ✅ (2026-04-24 `/simplify` 세션 사후 추가 검증) — 초기 17 컴파일 에러 발견 (`StartThreadIn` / `SwitchWorktree` / `project_worktree_paths` 잔여 참조 + 재작성된 `make_picker`/`entry_names` 시그너처 미반영) + 4 unused import 경고. 사용자 옵션 A (테스트 완전 삭제) 선택으로 `test_thread_target_local_project` / `test_thread_target_serialization_round_trip` 전체 제거 + `test_worktree_creation_preserves_selected_agent` 내 1 줄 제거 + `test_empty_query_entries` / `test_query_filtering_and_create_entries` / `test_multi_repo_hides_worktrees_and_disables_create_named` 호출 인자·assertion 축소 + unused imports 5 건 정리로 수정 완료. 재검증 통과(신규 에러 0).

**교훈 (향후 구조 변경 작업 적용)**: Phase 10·25 처럼 타입·enum·필드·메서드를 삭제하거나 시그너처를 바꾸는 작업은 **`cargo check -p <crate>` (lib 만) 통과가 완료 기준으로 불충분**. lib 빌드는 테스트 코드를 컴파일하지 않으므로 테스트 fixture·assertion·helper 에 남은 참조가 누락된다. 동일 규모의 작업에서는 반드시 `cargo check -p <crate> --tests` 까지 통과해야 "검증 완료" 로 간주한다. 본 Phase 25 는 사후 추가 검증으로 회귀 해결했으나 작업 단계 내에 포함됐어야 한다.

**예상 규모**: -1,032 (archive 본체) -~50 (agent_ui/agent_panel/picker 잔재) -~320 (git API 6종) -2 (i18n) ≈ **-1,400 라인**, 7 파일. 1 세션.

### 11.22.4. Part C — plan.md §11.7 / §11.10 outdated 정리

**§11.7 (Phase 8 보류 12건) 정리**
- 이미 적용 완료 6 건 표시 (#54201, #54431, #53884, #53086, #53184, #54318) — Phase 24 검증 결과 반영
- 영구 삭제 결정 3 건 표시 (#53998 dock widths, #53669 worktree naming, #53808 BGRA8 — Phase 25 검토 결과 반영)
- #54224 = Phase 24 완료 표시
- #48003 = Phase 25 Part A 완료 후 표시
- #53941 잔여 = Phase 25 Part B 영구 삭제 표시

**§11.10.10~§11.10.12 (Phase 10 Part B-2/B-3) 정리**
- 옵션 β "영구 채택" 표시
- "재검토 조건" 섹션 제거 또는 "영구 보류 확정" 으로 갱신
- Part B-3 / 옵션 A 승격 항목 영구 삭제 표시

**§11.7.2 / §11.7.3 / §11.7.4 / §11.7.5 등 outdated 보류 사유 섹션 정리**

### 11.22.5. 문서 갱신
- [x] `notes.md` 최상단에 Phase 25 항목 추가 완료 (2026-04-24)
- [x] `assets/release_notes.md` v0.4.0 `### 정리` 에 "HTTP MCP 컨텍스트 서버 설정 자동 정리" 항목 추가. Part B 는 내부 정리로 미반영
- [x] release_notes.md 섹션 헤더 날짜 갱신 — Phase 24 작업 시 이미 2026-04-24 로 갱신됨, 추가 변경 불필요

### 11.22.6. 승인 필요 사항 (2026-04-24 사전 승인 완료)
- [x] **공개 API 변경**: `crates/git/src/repository.rs` 의 `GitRepository` trait 에서 6 메서드 제거 — 사용자 결정 "B 나"
- [x] **파일 삭제 (1)**: `crates/agent_ui/src/thread_worktree_archive.rs` 1032 라인 — 사용자 결정 "2 B"
- [x] **체감 행동 변화 1**: picker 에서 기존 linked worktree 항목 자체가 안 보임 (이전: 클릭 시 토스트, 이후: 항목 자체 부재) — 사용자 결정 "B-1 b"
- [x] **migrator 신규 등록**: settings 마이그레이션 1 종 추가 — 사용자 결정 "1 사용"
- [x] **plan.md 대규모 정리**: §11.7 / §11.10 의 outdated 보류 기록 정리 — 진행 중 사용자 안내

### 11.22.7. 작업 순서 (순차)
1. **Step 1**: Part A — migrator 신설 (가장 작고 독립적)
2. **Step 2**: Part B-2 picker 정리 (Part B-1 의 SwitchWorktree 제거 전제 조건이지만 dispatch 부분만 먼저 제거)
3. **Step 3**: Part B-1 + B-3 — agent_ui/agent_panel 의 SwitchWorktree 제거 + thread_worktree_archive.rs 파일 삭제 + i18n 정리
4. **Step 4**: Part B-4 — git API 6종 제거 (외부 호출처 0 재검증 후)
5. **Step 5**: Part C — plan.md outdated 정리
6. **Step 6**: 검증 (`cargo check -p migrator -p agent_ui -p git -p project -p fs -p Dokkaebi` + Part A 테스트)
7. **Step 7**: 문서 갱신 (notes.md + release_notes.md)

### 11.22.8. 예상 작업 규모
- Part A: +100 라인
- Part B: -1,400 라인
- Part C: 문서만 (코드 변경 0)
- **순감축 약 -1,300 라인**, 8 파일 수정 + 1 파일 삭제 + 1 파일 신설. 1 세션.

### 11.21.6. 충돌·리스크

- **리스크 1 (체감 행동 변화)**: 임시 버퍼만 있는 워크스페이스가 더 이상 recent projects 목록에 표시 안 됨. 사용자가 이 동작을 원치 않으면 PR 적용 후 보완 필요.
- **리스크 2 (main.rs 두 호출처)**: GC 호출을 한 곳만 추가하면 다른 진입 경로에서는 GC 안 돌 수 있음. 옵션 B 채택 시 어느 진입점을 메인으로 볼지 결정 필요.
- **리스크 3 (테스트 영향)**: 신규 테스트 7개 모두 적용 시 Dokkaebi `WorkspaceDb::open_test_db` API 호환 확인 필요. 시간 부족하면 테스트는 skip (PR 본 동작은 테스트 없이도 적용 가능).
- **리스크 낮음**:
  - workspace_group panel 영향 없음 (별도 테이블)
  - schema migration 불필요 (session_id 컬럼 이미 존재)
  - DB 마이그레이션 위험 없음

### 11.21.7. 승인 필요 사항 (CLAUDE.md §1단계) — 2026-04-24 사용자 승인

- [x] **공개 API 시그너처 변경**: `recent_workspaces_on_disk` 제거, `recent_project_workspaces` + `garbage_collect_workspaces` 신설 — **진행**
- [x] **`recent_workspaces` SQL 컬럼 1 개 추가** (session_id, schema migration 불필요) — **진행**
- [x] **체감 행동 변화 1**: 임시 버퍼만 있는 워크스페이스는 recent projects 목록에 표시 안 함 (대신 last session 으로 복원) — **진행**
- [x] **`zed/src/main.rs` GC 호출 위치** — **옵션 B (메인 진입점만)**
- [x] **신규 테스트 7개 적용** — **적용 (필수로 격상)**
- [x] **release_notes.md 문구** — **(a) "임시 버퍼가 있는 워크스페이스가 재시작 시 사라지지 않도록 세션 복원 동작을 개선"**

### 11.21.8. 예상 규모
약 **+330 / -50 라인** (테스트 제외 시), 9 파일 수정. 0.5~1 세션.

테스트 포함 시 +560/-50 라인, 1 세션.

### 11.21.9. 작업 흐름 내 위치
- v0.233.5 백포트 잔여 작업의 마지막 항목.
- 완료 후 plan.md §11.7 의 outdated 보류 기록 6건 정리 (체크박스 해제 + 이미 적용 표시) 별도 단계 필요.

---

## 12. 영향 범위 외 (변경 없음)

---

## 12. 영향 범위 외 (변경 없음)
- `README.md` (CLAUDE.md 프로젝트 규칙: 수정 금지)
- `assets/keymaps/default-macos.json`, `default-linux.json` (Windows 전용 정책)
- `crates/repl`, `crates/dev_container/src/{docker,devcontainer_json,devcontainer_manifest}.rs`, `crates/language_models_cloud/` (파일 부재/비대상)
- Dokkaebi 독자 수정: `crates/zed/src/zed/windows_only_instance.rs` 좀비 감지 로직, `crates/zed/src/main.rs` Dev 채널 skip 제거 이력, `crates/cli/Cargo.toml` bin name `dokkaebi-cli`

---

## 13. Phase 13 — Zed v0.233.9 백포트 (PR #54752) — ✅ 완료 (2026-04-24)

### 13.1. 목표·범위
- 상류 v0.233.9 단일 PR #54752 (cherry-pick of #54723) 이식.
- **증상**: AgentV2 sidebar 활성 사용자가 기존 legacy `threads.db` 의 스레드가 사이드바에서 누락되는 문제.
- **원인**: `ThreadStore::new()` 가 `reload()` 를 fire-and-forget 으로 spawn 한 직후 `migrate_thread_metadata` 가 동기 `entries()` 를 읽어 race 발생 → 매 launch migration 이 empty iterator 로 no-op.

### 13.2. Dokkaebi 특수성 (위험도 재평가)
- `AgentV2FeatureFlag::enabled_for_all() -> true` 로 전 사용자 노출 (상류는 staff only).
- Dokkaebi 의 migration 은 `db.is_empty()` 게이트 → race 로 empty read 후 사용자가 한 스레드라도 상호작용하면 DB 가 비지 않게 되어 **legacy 스레드가 영구적으로 사이드바에서 누락** (archive view 도 없음).
- 따라서 상류보다 데이터 손실 위험이 **더 큼**.

### 13.3. 적용 매트릭스
| 상류 patch 항목 | Dokkaebi 적용 여부 |
|---|---|
| `futures::{FutureExt, future::Shared}` import 추가 (thread_store.rs) | **적용** |
| `ThreadStore::reload_task: Shared<Task<()>>` 필드 | **적용** |
| `spawn_reload()` + `reload_task()` public method | **적용** |
| `reload()` 시그너처 `&self` → `&mut self` + `spawn_reload` 사용 | **적용** (호출처 3곳 전부 `&mut Self` 컨텍스트라 호환) |
| `thread_store_ready.await` (migrate_thread_metadata 내부) | **적용** |
| `store.read_with(cx, \|_store, cx\| ThreadStore::global(cx)...)` → `thread_store.read_with(cx, \|store, _cx\| store...)` 로 변수 추출 | **적용** (race 수정과 세트) |
| 상류 `is_first_migration` 제거 / session_id HashSet dedup | **skip** (Dokkaebi 는 `db.is_empty()` 전역 게이트 + per-project 10 건 상한 구조) |
| per-batch top-5 rescue 이동 | **skip** (Dokkaebi 에 `archived` 필드 부재) |
| 신규 regression test `test_migration_awaits_thread_store_reload` | **적용** (Dokkaebi API 로 재작성) |

### 13.4. 작업 단계
- [x] 13.4.1. `crates/agent/src/thread_store.rs` 수정
- [x] 13.4.2. `cargo check -p agent` ✅ (2m15s, 신규 경고 0)
- [x] 13.4.3. `crates/agent_ui/src/thread_metadata_store.rs` 수정 (`thread_store_ready.await` 추가)
- [x] 13.4.4. 동 파일에 regression test 추가 (`test_migration_awaits_thread_store_reload`)
- [x] 13.4.5. `cargo check -p agent_ui --tests` ✅ (19.87s, 신규 경고 0) + `cargo test test_migration_awaits_thread_store_reload` ✅ (1 passed; 0.56s)
- [x] 13.4.6. `cargo check -p Dokkaebi` ✅ (16.68s, 신규 경고·에러 0)
- [x] 13.4.7. `notes.md` + `assets/release_notes.md` 갱신

### 13.5. 승인 필요 사항 — 2026-04-24 사용자 전체 승인 ("모두 진행")
- [x] `ThreadStore::reload` 시그너처 `&self` → `&mut self` (공개 API 시그너처 변경)
- [x] `ThreadStore` 신규 public method `reload_task()` 추가
- [x] regression test 포함

### 13.6. 예상 규모
- 약 +60 / -10 (코드), +60 (신규 regression test) 라인. 2 파일 수정. 0.5 세션 내.

### 13.7. 릴리즈 노트 문구 (잠정)
- 카테고리: `### 버그 수정`
- 문구: "업그레이드 후 사이드바에서 누락되던 에이전트 스레드 복구 — 저장된 스레드가 다시 사이드바에 표시됩니다."
