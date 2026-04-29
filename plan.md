# Shell Integration (OSC 133) + 터미널 탭 메타 표시 plan v1 (종료)

> **작성일**: 2026-04-29
> **종료일**: 2026-04-29
> **상태**: ✅ 종료 — Phase 1~5 모두 완료, 사용자 수동 검증 대기
> **버전 기준**: `crates/zed/Cargo.toml` **v0.4.2** (bump 완료)
> **출처 검토**: Warp `warp-master` 분석 결과 1순위 후보 (라이선스 검토 별도 통과)
> **사용자 결정 (2026-04-29)**:
> - Phase 4 주입 정책: **Auto** (표준 셸 감지 시 자동 ON, 비표준 환경 OFF) ✅ 적용
> - Phase 3 선택 항목: **변경** — `title()` 에 실행 중 명령 라인 32자 노출 (foreground 자식 프로세스명 대체) ✅ 적용
> - 버전: **v0.4.2 신규 섹션** ✅ 적용

## 목표

PTY 기반 일반 셸(bash / zsh / PowerShell / cmd) 사용 시에도 명령어 단위 메타데이터(시작·종료 시각, 종료 코드, 명령 라인, cwd)를 인지해 다음 사용자 가치를 제공:

1. **탭 아이콘에 마지막 명령 종료 코드 표시** — 현재는 `task` 터미널만 표시됨. 일반 셸 터미널도 동일 패턴 확장.
2. **탭 제목/툴팁에 실행 중인 명령 라인 노출** — 현재는 foreground 자식 프로세스명(`claude.exe`)까지. 인자 포함 라인을 보유하도록 확장.
3. **(스코프 외 — 후속 plan 후보) 블록 단위 navigation / 검색**

## 비목표 (이번 plan에서 안 함)

- 블록 모델 도입(alacritty 그리드 교체) — 별도 plan
- 자연어→명령어 변환 — 별도 plan
- VSCode 전용 OSC 633 / iTerm2 전용 OSC 1337 — 표준성 이유로 제외
- 명령어 보정 제안(thefuck-like) — 별도 plan

## 라이선스 게이트 (엄수)

| 항목 | 정책 |
|---|---|
| `warp-master/**/src/**` 본문 | **열람 금지**. 메타(`*.md`, `Cargo.toml`)만 참고 가능 |
| 참조 가능 출처 | FinalTerm 공식 사양, VSCode terminal docs, iTerm2 docs, WezTerm `assets/shell-integration/` (**MIT**) |
| 참조 금지 출처 | Kitty(GPL-3.0), VTE(LGPL-2.1), Warp(AGPL) — 라이선스 강도 차이로 안전한 MIT만 사용 |
| 외부 의존성 추가 | 본 plan 에서는 0건 예상. 추가 시 MIT/Apache-2.0/BSD/MPL-2.0 만 |
| 코드 스니펫 인용 | 금지. 클린룸 사후 diff 금지 |

## 기술 배경

- `alacritty_terminal` v0.25.1(zed-industries fork rev `9d9640d4`)이 사용하는 `vte` 0.15.0의 `osc_dispatch`는 OSC 133 미처리 → `[unhandled osc_dispatch]` 로그만 발생.
- 현재 PTY 데이터 파이프: `pty_adapter.rs::spawn_pty_reader` → `vte::ansi::Processor::advance(&mut Term, bytes)`.
- → **alacritty 내부 패치 없이** `pty_adapter.rs`에서 바이트 스트림을 pre-scan 하는 경량 상태 기계로 OSC 133 검출 후 별도 이벤트 emit. alacritty 파서는 그대로 통과(미처리 OSC는 무해).

## OSC 133 표준 시퀀스 (FinalTerm)

```
ESC ] 133 ; A [;params] ST   — Prompt Start (PS1 시작)
ESC ] 133 ; B [;params] ST   — Command Start (사용자 입력 영역 시작)
ESC ] 133 ; C [;params] ST   — Command Executed (커맨드 실행 시작 = 출력 영역 시작)
ESC ] 133 ; D [;exit_code] ST — Command Finished (종료 코드 포함)
```
- `ST` = `BEL` (0x07) 또는 `ESC \` (0x1B 0x5C)
- 사양 자체는 공개 표준 → 저작권 보호 대상 아님

## 작업 단계

### Phase 1 — OSC 133 바이트 스캐너 + 이벤트 정의 (순수 추가) [x]
- 채널 패턴 결정 (spike 결과): 별도 `UnboundedSender<ShellIntegrationEvent>` 채널 신설. `Terminal::events_rx` 옆에 `shell_events_rx` 추가하고 `select_biased!` 로 동시 폴링. alac `AlacTermEvent` 는 손대지 않음 (외부 크레이트 보호).
- [x] `crates/terminal/src/shell_integration.rs` **신규 파일**: `Osc133Scanner` 상태 기계 + `ShellIntegrationEvent` enum + 단위 테스트 20건.
- [x] `crates/terminal/src/pty_adapter.rs::spawn_pty_reader` 시그너처 확장: `shell_events_tx: Option<UnboundedSender<ShellIntegrationEvent>>` 파라미터 추가. 읽은 바이트 청크에 대해 스캐너 `feed` 호출 후 매칭 이벤트 송신.
- [x] `crates/terminal/src/terminal.rs` 빌더(display-only + PTY)에 채널 pair 생성 코드 추가, `Terminal::events_rx` 옆에 `shell_events_rx` 추가. `subscribe()` 의 event loop 에서 `select_biased!` 로 동시 폴링.

### Phase 2 — Terminal 엔티티에 명령 메타 보유 (구조 변경 — 승인 완료) [x]
- [x] `Terminal` 구조체에 `shell_command_status: ShellCommandStatus` + `last_command_line: Option<String>` 필드 추가. 신규 enum `ShellCommandStatus { Idle, Running, Succeeded, Failed { exit_code } }`.
- [x] `process_shell_event` 메서드 추가. task 미설정 PTY 터미널에 한해 OSC 133 신호로 상태 갱신. 상태 변경 시 `Event::TitleChanged` emit 으로 탭 UI 갱신 트리거.
- [x] `pub fn shell_command_status(&self)` + `pub fn last_command_line(&self) -> Option<&str>` 게터 추가 (Phase 3 UI 에서 사용).
- 검증: `cargo check -p terminal` 통과, `cargo check --tests -p terminal` 통과, `cargo test -p terminal shell_integration::` 20/20 통과, `cargo check -p Dokkaebi` 통과 (신규 warning 0).

### Phase 2 — Terminal 엔티티에 명령 메타 보유 (구조 변경 — 승인 필요)
- [ ] `Terminal` 구조체에 다음 필드 추가:
  - `last_command_line: Option<String>` — 마지막 `C` 시점의 입력 라인(있으면)
  - `last_exit_code: Option<i32>`
  - `command_status: ShellCommandStatus { Idle, Running, Succeeded, Failed }` (신규 enum)
- [ ] Phase 1 이벤트 수신 시 위 필드 갱신.
- [ ] **승인 필요**: 구조 변경. 다만 신규 필드만 추가하고 기존 필드/메서드는 변경하지 않음.

### Phase 3 — 탭 UI 확장 (UI/UX 변경) [x]
- [x] `terminal_view.rs::tab_content` 에 task 가 아닌 일반 셸 터미널의 `shell_command_status` 분기 추가 — Running=PlayFilled/Disabled, Succeeded=Check/Success, Failed=XCircle/Error, Idle=Terminal/Muted(기존 유지).
- [x] `tab_tooltip_content` 에 `shell_status_line` (Running / last_succeeded / last_failed (exit_code)) 한 줄 PID 아래에 추가. i18n 키 3개 신규.
- [x] **사용자 결정 "변경" 적용** — `terminal.rs::title()` 의 PTY 분기에서 `shell_command_status::Running` 이면 foreground argv 합쳐 32자 truncate 노출. 그 외(Idle/Succeeded/Failed) 는 기존 25자 process_name 유지.

### Phase 4 — Shell integration 스크립트 자동 주입 (외부 호출 — 승인 완료) [x]
- 단순화 결정 (착수 직전): zsh 는 macOS/Linux 정책상 제외, cmd 는 hook 한정으로 가치 낮아 제외. **bash + pwsh/powershell** 만 지원. 설정 노출은 후속 plan 으로 미루고 escape hatch `DOKKAEBI_SHELL_INTEGRATION=off` 환경변수만 제공.
- [x] `crates/terminal/assets/shell_integration/dokkaebi.bash` 신규 — PROMPT_COMMAND(D+A) + DEBUG trap(C) + 사용자 .bashrc 명시 source. `__dokkaebi_running` 플래그로 자체 함수 재진입 방지.
- [x] `crates/terminal/assets/shell_integration/dokkaebi.ps1` 신규 — prompt 함수 wrapping(D+A+orig+B) + PSReadLine Enter hook(C) + 사용자 $PROFILE source.
- [x] `shell_integration.rs::inject_shell_integration(shell_kind, &mut args, &mut env) -> InjectOutcome` 헬퍼 — Auto 정책. 스크립트는 `include_str!` 임베드 후 `paths::temp_dir()/shell_integration/` 에 1회 작성(내용 동일 시 skip). bash → `--rcfile <path> -i`, pwsh → `-NoExit -Command "& '<path>'"`.
- [x] `terminal.rs` PTY spawn 직전 호출. task=None 일 때만 적용. 사용자 args 비어있을 때만 적용.
- [x] `crates/terminal/Cargo.toml` 에 `paths.workspace = true` 추가 (신규 워크스페이스 의존성 0건).

### Phase 5 — 검증 + 문서 [x]
- [x] `cargo check -p terminal -p terminal_view` 통과
- [x] `cargo check --tests -p terminal` 통과 (memory: `feedback_tests_check_on_api_removal.md` 적용)
- [x] `cargo check -p Dokkaebi` 통과 (신규 warning 0)
- [x] `cargo test -p terminal shell_integration::` **20 passed; 0 failed**
- [ ] 사용자 환경 수동 검증(필수): bash/pwsh 각각에서 성공/실패 명령 후 탭 아이콘·툴팁 변화, dotfiles/profile 비변경 확인, `DOKKAEBI_SHELL_INTEGRATION=off` escape 동작
- [x] `notes.md` Phase 1~5 종합 항목 추가
- [x] `release_notes.md` v0.4.2 신규 섹션 — 새로운 기능 1 + UI/UX 3
- [x] `crates/zed/Cargo.toml` v0.4.1 → v0.4.2 bump

## 승인 필요 사항 요약

| 항목 | 사유 |
|---|---|
| **Phase 1** Terminal `Event` enum 신규 variant 4개 | 공개 API 변경 |
| **Phase 2** Terminal 구조체 필드 3개 추가 | 구조 변경 |
| **Phase 3 (선택)** title() 에 명령 요약 32자 표시 | 기존 사용자 동작 변경 가능 — 사용자 의견 필요 |
| **Phase 4** Shell rc/env 자동 주입 | 외부 호출 변경, 사용자 환경 영향 가능 — 기본 동작·opt-out 정책 사용자 결정 필요 |
| **버전 bump** | v0.4.1 누적 또는 v0.4.2 신규 — 사용자 결정 |

## 리스크 및 대응

| 리스크 | 대응 |
|---|---|
| OSC 133 미지원 셸/CLI(예: 일부 REPL, ssh 원격) 에서 무동작 | 기능이 자동 활성됐을 때 단순 미작동(이전 동작 유지)이라 안전. UI 는 task 미적용 셸의 기본 아이콘 유지 |
| Phase 1 alac `Event` enum 우회 채널 설계 복잡도 | Phase 1 착수 직전 30 분 spike 로 채널 패턴 결정. 결과 plan v2 로 보강 |
| Shell 스크립트 주입이 사용자 dotfiles 와 충돌 | rc 파일 수정 절대 금지. 임시 rcfile 또는 환경변수 hook 만 사용. opt-out 즉시 가능 |
| Windows cmd.exe 한계 | cmd 는 prompt only 부분 지원으로 명시 |
| portable-pty 안정성 회귀 | Phase 1~3 까지는 PTY 인자 변경 없음. Phase 4 만 인자/env 추가 → 분리 적용 |

## 작업 외 자동 제외 (참고)

- macOS/Linux 키맵·플랫폼 분기 (CLAUDE.md 정책)
- collab/세션 공유 (Warp `shared_session.rs` 등 — 정책)
- VSCode OSC 633 / iTerm2 OSC 1337 (표준성 이유)
- Warp block 모델 / warp_completer / command-corrections (라이선스 게이트)

## 후속 plan 후보 (본 plan 종료 후)

| 후보 | 트리거 |
|---|---|
| 블록 단위 navigation (Ctrl+↑/↓ 로 이전 명령 점프) | OSC 133 안정 동작 확인 후 |
| Kitty Keyboard Protocol 확장 | 사용자 modern TUI 필요 사례 발생 시 |
| 자연어 → 명령어 변환 | Dokkaebi 어시스턴트 통합 설계 별도 |

---

**다음 액션**: 본 plan 의 ① Phase 4 shell rc/env 주입 정책(opt-in/opt-out) ② title() 에 명령 요약 노출 여부 ③ 버전 bump 정책 — 3가지 결정 후 Phase 1 착수.
