# 윈도우 표시 보장 + 표시 실패 cleanup — 계획 (v2)

> **작성일**: 2026-04-26 (v1) → **개정**: 2026-04-26 (v2, 리뷰 반영)
> **대상**:
> - `crates/gpui_windows/src/window.rs` (윈도우 표시 트리거 보강)
> - `crates/workspace/src/workspace.rs` (`Workspace::new_local` cleanup)
> - `crates/zed/src/main.rs` (`restore_or_create_workspace` 진입/종단 가드)
>
> **배경**: 사용자 보고 — 실행 시 프로세스만 살고 화면이 안 뜨는 증상 발생, 재실행해도 같은 증상 무한 반복. 좀비 본체 사이클로 식별.
>
> 1. **표시 트리거 누락**: `WindowOptions { show: false }` (zed.rs:349) 로 hidden 윈도우 생성 → 후속 `activate_window()` 만이 유일한 표시 트리거. `set_window_placement` 의 Windowed 분기는 `SetWindowPlacement` 만 호출하고 `ShowWindowAsync` 가 없으며, `placement.showCmd` 가 명시되지 않아 GetWindowPlacement 의 초기값(SW_HIDE/SW_SHOWMINIMIZED 가능)이 그대로 사용됨.
> 2. **부분 실패 시 hidden 윈도우 잔존**: `Workspace::new_local` (workspace.rs:1789-2022) 에서 `cx.open_window?` 성공 후 후속 `?` 가 fail 하면 OS 윈도우 + GPUI 핸들이 만들어진 채로 cleanup 없이 dangling. 호출자 `restore_or_create_workspace` 의 모든 분기가 fail 해도 spawn 결과가 `detach_and_log_err` 로 swallow.
>
> **목적**:
> - **(2)** 윈도우가 만들어지면 어떤 분기로 가도 표시 트리거가 보장되도록 한다.
> - **(3)** 부분 실패로 hidden 윈도우가 잔존하지 않도록 cleanup. `restore_or_create_workspace` 종단에서 성공 카운트 0 이면 fail_to_open_window 진입.
>
> **비목적**:
> - 좀비 본체에 args 보낸 두 번째 인스턴스 hang 문제 ((5) 타임아웃) — 범위 외.
> - **`cx.open_window` 의 `build_root_view` 콜백에서 panic 하는 경우 (gpui app.rs:1081)** GPUI windows[id]=None 잔존 — gpui core 문제. 본 변경으로 해결 안 됨. 발생 시 좀비 사이클 재발 가능 — 별도 PR 필요.
> - 좀비 본체 자가 종료 ((1)) — 범위 외.
> - macOS/Linux 키맵·플랫폼 분기 — Windows 경로만 반영.

---

## 1. v1 → v2 변경 요약 (리뷰 반영)

| 영역 | v1 | v2 |
|---|---|---|
| (2) showCmd 설정 위치 | `retrieve_window_placement` 일괄 SW_SHOWNORMAL | `set_window_placement` 분기별 (Windowed=SW_SHOWNORMAL, Maximized=SW_SHOWMAXIMIZED) — Maximized 깜빡임 방지 |
| (2) ShowWindow 상수 | SW_SHOWNORMAL | **SW_NORMAL** (코드베이스 컨벤션 — events.rs:999 일관) |
| (3-B) 검증 기준 | `cx.windows().is_empty()` | **success_count 카운터** (hidden window가 컬렉션에 포함되는 약점 회피) |
| (3-B) quit_on_empty 경쟁 | 미고려 | **`set_quit_mode(QuitMode::Explicit)` 진입/종단 토글** (cleanup 시 자동 cx.quit() 차단, bail 도달 보장) |
| 검증 | `cargo check -p ...` | **`--tests` 추가** (구조 변경 검증 규칙) |
| 구현 패턴 | 의사코드만 | helper closure로 cleanup 통합 명시 |

---

## 2. 변경 상세

### (2-A) `set_window_placement` 분기별 showCmd 명시 + Windowed ShowWindowAsync 추가

**파일**: `crates/gpui_windows/src/window.rs` 라인 317-340

**현재 코드 골격**:
```rust
fn set_window_placement(self: &Rc<Self>) -> Result<()> {
    let Some(open_status) = self.state.initial_placement.take() else {
        return Ok(());
    };
    match open_status.state {
        WindowOpenState::Maximized => unsafe {
            SetWindowPlacement(self.hwnd, &open_status.placement)?;
            ShowWindowAsync(self.hwnd, SW_MAXIMIZE).ok()?;
        },
        WindowOpenState::Fullscreen => { ... toggle_fullscreen() ... }
        WindowOpenState::Windowed => unsafe {
            SetWindowPlacement(self.hwnd, &open_status.placement)?;
        },
    }
    Ok(())
}
```

**변경 후**:
```rust
fn set_window_placement(self: &Rc<Self>) -> Result<()> {
    let Some(mut open_status) = self.state.initial_placement.take() else {
        return Ok(());
    };
    // GetWindowPlacement 결과 showCmd 의 초기값(SW_HIDE 등)에 의존하지 않도록
    // 분기별로 명시 설정 — hidden 윈도우(WS_VISIBLE 미포함)도 안정적으로 표시.
    open_status.placement.showCmd = match open_status.state {
        WindowOpenState::Maximized => SW_SHOWMAXIMIZED.0 as u32,
        WindowOpenState::Fullscreen | WindowOpenState::Windowed => SW_SHOWNORMAL.0 as u32,
    };
    match open_status.state {
        WindowOpenState::Maximized => unsafe {
            SetWindowPlacement(self.hwnd, &open_status.placement)
                .context("failed to set window placement")?;
            ShowWindowAsync(self.hwnd, SW_MAXIMIZE).ok()?;
        },
        WindowOpenState::Fullscreen => {
            unsafe {
                SetWindowPlacement(self.hwnd, &open_status.placement)
                    .context("failed to set window placement")?
            };
            self.toggle_fullscreen();
        }
        WindowOpenState::Windowed => unsafe {
            SetWindowPlacement(self.hwnd, &open_status.placement)
                .context("failed to set window placement")?;
            // SetWindowPlacement 만으로는 일부 케이스에서 표시가 누락되는 사례가
            // 보고됨. ShowWindowAsync 로 이중 안전망 — events.rs:999 와 동일 SW_NORMAL.
            ShowWindowAsync(self.hwnd, SW_NORMAL).ok()?;
        },
    }
    Ok(())
}
```

**주의**: `SW_SHOWNORMAL == SW_NORMAL == 1` (Win32). showCmd 필드에 setting 시에는 `SW_SHOWNORMAL` 명칭이 의미상 정명칭, ShowWindowAsync 인자에는 코드베이스 기존 컨벤션(`SW_NORMAL`) 사용. import 추가: `SW_SHOWNORMAL`.

### (2-B) `retrieve_window_placement` — 변경 없음

**판단**: v1에서 여기에 showCmd를 일괄 설정하면 Maximized 분기에서 SetWindowPlacement(NORMAL) → ShowWindowAsync(MAXIMIZE) 순으로 깜빡임 발생. (2-A)에서 분기별로 덮어쓰는 게 깔끔하므로 retrieve 단계는 변경 없음.

### (3-A) `Workspace::new_local` 새 윈도우 경로 cleanup

**파일**: `crates/workspace/src/workspace.rs` 라인 1933-1962, 1974-1981

**위험 지점 두 곳**:
1. 라인 1933-1956: `cx.open_window(options, |window, cx| { ... })?` — 성공 시 윈도우 등록. 그 직후 1957-1960의 `window.update(...)?` 가 fail 하면 dangling.
2. 라인 1974-1981: `window.update(cx, |_, window, cx| { ... open_items(...) })?` 의 outer `?` 또는 inner setup fail 시 dangling.

**변경 패턴** (helper closure):
```rust
// 윈도우 생성 직후부터 활성화 직전까지의 fail-cleanup 영역.
// 새로 만든 윈도우(requesting_window 가 None 인 경로) 한정.
let cleanup_on_fail = |window: WindowHandle<MultiWorkspace>, cx: &mut AsyncApp| {
    let _ = window.update(cx, |_, window, _| window.remove_window());
};

// (1957-1960) workspace clone 단계
let workspace = match window.update(cx, |multi_workspace: &mut MultiWorkspace, _, _cx| {
    multi_workspace.workspace().clone()
}) {
    Ok(ws) => ws,
    Err(e) => {
        cleanup_on_fail(window, cx);
        return Err(e.into());
    }
};
(window, workspace)
```

같은 패턴으로 1974-1979의 outer `?` 도 매치 분기로 풀어 cleanup 후 Err 전파.

**`requesting_window: Some(...)` 경로 (1872-1904)**: 새 윈도우 생성 안 함 → cleanup 대상 아님. 1903의 `?` 실패 시 윈도우는 호출자 소유로 유지(기존 동작).

### (3-B) `restore_or_create_workspace` — success_count + QuitMode 토글

**파일**: `crates/zed/src/main.rs` 라인 1413-1555

**변경**:
```rust
pub(crate) async fn restore_or_create_workspace(
    app_state: Arc<AppState>,
    cx: &mut AsyncApp,
) -> Result<()> {
    // cleanup 으로 마지막 윈도우가 닫혀 quit_on_empty 가 자동 cx.quit() 을
    // 호출하지 않도록 복원 진행 동안 Explicit 로 강제. 종단에서 Default 복원.
    let prev_quit_mode = cx.update(|cx| {
        let mode = cx.quit_mode();
        cx.set_quit_mode(QuitMode::Explicit);
        mode
    })?;
    let restore_result = restore_or_create_workspace_inner(app_state, cx).await;
    cx.update(|cx| cx.set_quit_mode(prev_quit_mode)).ok();
    restore_result
}

async fn restore_or_create_workspace_inner(
    app_state: Arc<AppState>,
    cx: &mut AsyncApp,
) -> Result<()> {
    let kvp = cx.update(|cx| KeyValueStore::global(cx))?;
    let mut success_count: usize = 0;

    if let Some((multi_workspaces, remote_workspaces)) =
        restorable_workspaces(cx, &app_state).await
    {
        // 기존 흐름 유지하되, restore_multiworkspace Ok 시 success_count += 1.
        // remote tasks 의 Ok 결과도 동일.
        for multi_workspace in multi_workspaces {
            match restore_multiworkspace(multi_workspace, app_state.clone(), cx).await {
                Ok(result) => {
                    if result.errors.is_empty() {
                        success_count += 1;
                    }
                    for error in result.errors {
                        log::error!("Failed to restore workspace in group: {error:#}");
                    }
                }
                Err(e) => {
                    log::error!("Failed to restore workspace: {e:#}");
                }
            }
        }
        // remote 처리 동일 패턴
        // ...
        // fallback open_new 진입 조건은 success_count == 0 으로 단순화.
        if success_count == 0 {
            log::error!("All workspace restorations failed. Opening fallback empty workspace.");
            cx.update(|cx| {
                workspace::open_new(Default::default(), app_state.clone(), cx, |workspace, _window, cx| {
                    workspace.show_toast(
                        Toast::new(NotificationId::unique::<()>(), "Failed to restore workspaces..."),
                        cx,
                    );
                })
            })
            .await?;
            success_count += 1; // open_new 가 성공한 케이스
        }
    } else if matches!(kvp.read_kvp(FIRST_OPEN), Ok(None)) {
        cx.update(|cx| show_onboarding_view(app_state, cx)).await?;
        success_count += 1;
    } else {
        cx.update(|cx| {
            workspace::open_new(Default::default(), app_state, cx, |workspace, window, cx| {
                terminal_view::TerminalView::deploy(
                    workspace, &workspace::NewCenterTerminal { local: false }, window, cx,
                );
            })
        })
        .await?;
        success_count += 1;
    }

    if success_count == 0 {
        anyhow::bail!("모든 워크스페이스 복원·신규 생성 시도 실패 — 표시할 윈도우 없음");
    }
    Ok(())
}
```

**핵심**:
- `success_count` 는 호출자 명시 카운트 — hidden window가 컬렉션에 포함되는 GPUI 동작과 무관.
- QuitMode::Explicit 토글로 (3-A) cleanup → quit_on_empty 자동 발동 차단. bail 도달 보장.
- bail → main.rs:998 의 `fail_to_open_window_async` → `process::exit(1)` → 다음 실행은 mutex 회수 후 first 진입.

**부수**: `cx.quit_mode()` getter API 가 GPUI에 노출되어 있는지 확인 필요. 없으면 추가 또는 `QuitMode::Default` 로 단순 복원 (Dokkaebi는 Application 빌드 시 명시적으로 변경하지 않음 — main.rs:106의 `with_quit_mode(QuitMode::Explicit)` 는 fail_to_open_window 분기 한정).

---

## 3. 데이터/동작 시나리오 검증 (확장)

### A. 정상 실행 흐름
- 모든 `?` 통과 → `restore_multiworkspace` 끝의 `window.activate_window()` (workspace.rs:9381) → `set_window_placement(Windowed)` → SetWindowPlacement(showCmd=SHOWNORMAL) + ShowWindowAsync(SW_NORMAL) → 표시 보장. success_count > 0 → bail 안 함.
- Maximized 워크스페이스: showCmd=SHOWMAXIMIZED → SetWindowPlacement → ShowWindowAsync(MAXIMIZE) — 깜빡임 없음.

### B. open_items 실패 시
- (3-A) cleanup → window.removed=true → 다음 effect cycle 에서 OS 윈도우 destroy. cx.windows에서 제거.
- restore_multiworkspace 에 Err 전파 → restore_or_create_workspace 의 results 처리 → success_count 증가 안 함.
- 모든 multi_workspace 실패 → fallback open_new 시도. 성공 시 success_count=1. 실패 시 `?` 로 함수 자체 Err 반환.
- bail 또는 함수 Err → fail_to_open_window_async → process::exit(1).

### C. requesting_window 경로 — 변경 없음, 안전

### D. cx.open_window 콜백 panic
- gpui app.rs:1097-1100: Window::new Err 면 cx.windows.remove(id). 그러나 build_root_view panic 케이스는 plan 비목적. 본 변경 효과 범위 외.

### E. activate_window noop (foreground_executor 큐 막힘)
- 본 변경으로 해결 안 됨 — 별도 GPUI 이슈.

### F. WS_VISIBLE 미채택 — v1 판단 유지
- CW_USEDEFAULT 위치에 잠깐 표시되는 시각 결함 야기.

### G. quit_on_empty 경쟁 (v2 추가)
- **시나리오**: 단일 multi_workspace 복원 → Workspace::new_local 첫 윈도우 생성 → open_items fail → cleanup `remove_window` → 다음 effect cycle에서 cx.windows 빔 → quit_on_empty=true 면 자동 cx.quit() → PostQuitMessage(0) → 메시지 펌프 종료.
- **위험**: fallback open_new가 PostQuitMessage 이후 큐에 남으면 영원히 실행 안 됨 → success_count=0 검증 도달 안 함 → 좀비 잔존 가능 (또는 그냥 종료되지만 사용자에게는 검은 화면 후 즉시 종료처럼 보임).
- **해결**: `restore_or_create_workspace` 진입 시 `set_quit_mode(QuitMode::Explicit)`. 이 동안 cleanup이 일어나도 자동 quit 안 함. 종단에서 이전 모드 복원.
- **검증**: Explicit 동안 cx.quit() 호출은 명시적으로만 가능. cleanup-cascade 차단 확인.

### H. macOS/Linux 영향
- Windows 전용 코드(window.rs)는 영향 없음.
- workspace.rs cleanup: macOS는 quit_on_empty=false 라 cleanup해도 자동 quit 안 됨 — 영향 없음. Linux는 Windows와 동일.
- main.rs QuitMode 토글: 모든 플랫폼에서 동일 효과. macOS는 어차피 Default=Explicit 라 no-op.

### I. requesting_window 경로 cleanup 재검토
- 라인 1903의 `?` fail 시 cleanup 안 함이라 v1 결정. 호출자 (`restore_multiworkspace` 라인 9347) 가 errors.push 만 하고 윈도우는 유지. 새 hidden 좀비는 안 생김 — 결정 유지.

### J. Maximized + showCmd 변경의 부수 효과
- 기존: Maximized 분기에서 SetWindowPlacement(showCmd=GetWindowPlacement 값) → ShowWindowAsync(MAXIMIZE). 만약 직전값이 SW_HIDE면 SetWindowPlacement가 hidden으로 만든 후 ShowWindowAsync로 maximize → 잠깐 hidden 표시.
- 변경: showCmd=SHOWMAXIMIZED 명시 → SetWindowPlacement가 즉시 maximize → ShowWindowAsync(MAXIMIZE)는 사실상 idempotent. 깜빡임 감소.

---

## 4. 변경 파일 목록

| 파일 | 수정 내용 | 예상 라인 변경 |
|---|---|---|
| `crates/gpui_windows/src/window.rs` | (2-A) `set_window_placement` 분기별 `placement.showCmd` 명시 + Windowed 분기에 `ShowWindowAsync(SW_NORMAL)` 추가. SW_SHOWNORMAL/SW_SHOWMAXIMIZED import. | +6 |
| `crates/workspace/src/workspace.rs` | (3-A) `Workspace::new_local` 새 윈도우 경로 두 `?` 지점에 helper closure cleanup. | +12~15 |
| `crates/zed/src/main.rs` | (3-B) `restore_or_create_workspace` 진입/종단 QuitMode::Explicit 토글 + success_count 카운터 + 종단 bail. inner 함수로 분리. | +25~30 |
| `crates/gpui/src/app.rs` (필요 시) | `pub fn quit_mode(&self) -> QuitMode` getter 추가 (현재 setter 만 있음). 이전 모드 복원에 필요. | +3 |
| `notes.md` | 2026-04-26 항목 추가 | +5 |
| `assets/release_notes.md` | v0.4.0 `### 버그 수정` 카테고리에 항목 추가 | +1~2 |

**`quit_mode()` getter 추가 가능성**: GPUI 자체에 setter 만 있다면 getter 추가가 필요 (구조 변경). 또는 단순화: 진입 전 `let was_explicit = false;` 가정하지 말고, **종단에서 무조건 `QuitMode::Default` 로 복원**(Dokkaebi 의 평소 모드는 Default). 이게 더 단순.

**채택**: 종단 복원은 `QuitMode::Default` 하드코딩. getter 추가 불필요.

---

## 5. 작업 단계

### Phase A — 코드 변경
- [x] **1. 범위 확인 — 승인 완료 (2026-04-26 v2 plan)**
- [x] 2. (2-A) `gpui_windows/src/window.rs` `set_window_placement` 수정 (분기별 showCmd 명시 + Windowed ShowWindowAsync). SW_* import 는 이미 와일드카드 import 로 가용.
- [x] 3. (3-A) `workspace/src/workspace.rs` `Workspace::new_local` `is_new_window` 도입 + 두 위험 지점에 cleanup
- [x] 4. (3-B) `zed/src/main.rs` `restore_or_create_workspace` wrapper + `_inner` 분리 + QuitMode::Explicit/Default 토글 + success_count + bail. QuitMode import 이미 존재.

### Phase B — 검증
- [ ] 5. `cargo check -p gpui_windows` (사용자 직접 — Dev Drive 신뢰 탑재 에러로 Dokkaebi 인스턴스 cargo spawn 차단)
- [ ] 6. `cargo check -p workspace` (사용자 직접)
- [ ] 7. `cargo check -p workspace --tests` (사용자 직접)
- [ ] 8. `cargo check -p Dokkaebi` (사용자 직접)
- [ ] 9. `cargo check -p Dokkaebi --tests` (사용자 직접)
- [x] 10. 코드 리뷰 — 시나리오 A~J 트레이스 재확인 (plan §3, 모든 분기 코드와 매칭 확인)
- [ ] 11. 런타임 검증 (사용자 수동)

### Phase C — 문서
- [x] 12. `notes.md` 항목 추가 (2026-04-26)
- [x] 13. `assets/release_notes.md` — `crates/zed/Cargo.toml` 버전 v0.4.1 기준이라 파일 맨 위에 새 v0.4.1 (2026-04-26) 섹션 신설 + `### 버그 수정` 1 항목 + 구분자 `---`. (초안에서 v0.4.0 섹션에 잘못 추가 + 날짜 변경한 부분 원복 완료.)
- [x] 14. 완료 보고 (이 메시지)

> 검증 단계의 `[x]` 는 빌드/체크 통과 후에만 표시한다.

---

## 6. 검증 방법

### 빌드 검증
- `cargo check -p gpui_windows` — 신규 경고 0
- `cargo check -p workspace --tests` — 테스트 fixture까지 신규 경고 0
- `cargo check -p Dokkaebi --tests` — 신규 경고·에러 0 (메모리 규칙)

### 런타임 검증 (사용자 수동)
1. **정상 실행**: dokkaebi.exe 실행 → 윈도우 즉시 표시. 깜빡임/지연 증가 없는지 확인.
2. **Maximized 복원**: 종료 시 최대화 → 재실행 → 즉시 최대화로 표시 (NORMAL→MAXIMIZE 깜빡임 없음).
3. **Fullscreen 복원**: 종료 시 fullscreen → 재실행 → 정상 복원.
4. **다중 윈도우**: 워크스페이스 여러 개 열린 상태에서 종료 → 재실행 → 모두 정상 표시.
5. **표시 실패 → process::exit 검증** (재현 가능 시): 손상된 세션 DB로 실행 → 콘솔 로그에 "표시할 윈도우 없음" → 프로세스 종료 → 작업 관리자에 dokkaebi.exe 잔존 안 함. 재실행 시 정상 first 진입 (mutex 회수).
6. **마지막 윈도우 닫기**: 평소 사용 중 마지막 워크스페이스 탭 닫기 → 정상 cx.quit() 동작 (QuitMode 복원 확인).

---

## 7. 승인 필요 항목 (v2)

CLAUDE.md "절대 금지 — 승인 필요":
1. **동작 변경 (윈도우 표시 흐름)**: `set_window_placement` 분기별 showCmd 명시 + Windowed 분기 ShowWindowAsync 추가. 깜빡임 영향 없음 또는 감소(시나리오 J).
2. **흐름 변경 (실패 시 cleanup + bail + QuitMode 토글)**: cleanup으로 hidden 좀비 차단, bail로 fail_to_open_window 보장, QuitMode::Explicit 토글로 quit_on_empty 경쟁 차단. **복구 동작이 "재실행"으로 변경됨** (현재는 좀비 잔존). 일시적 fail에도 process::exit 됨 — 안전성 우선.
3. **inner 함수 분리**: `restore_or_create_workspace_inner` 내부 함수 추가는 구조 변경 (CLAUDE.md "구조 변경" 해당). 외부 시그너처는 변경 없음.
4. **알트 옵션**:
   - (2-A) 의 ShowWindowAsync 추가 생략 — showCmd 명시만으로 충분할 가능성. 이중 안전망 vs 단순함.
   - QuitMode 토글 대신 cleanup 시점을 success_count 검증 후로 지연 — 흐름 어색.
   - GPUI에 `quit_mode()` getter 추가 — 더 정확한 복원, 그러나 GPUI core 변경.

승인되면 Phase A 부터 구현 진행하겠습니다.
