//! 워크스페이스 첫 mount 완료를 알리는 글로벌 신호.
//!
//! 배경: `crates/zed/src/main.rs` 의 `restore_or_create_workspace_inner()` 가
//! 비동기 task 로 spawn 되는 동안, IPC 수신 루프(`open_rx.next().await`)도 별도
//! 비동기 task 로 동작한다. 본체 첫 실행 직후 Claude Code 가 보낸 IPC 가 워크스페이스
//! mount 완료 전에 도달하면 `mark_bell_for_notification` 의 `cx.windows()` 가 빈 vec
//! 을 반환해 발신 터미널 매칭이 실패하고, 결과적으로 서브에이전트 뷰 탭이 생성되지
//! 않는다. 본체 종료 후 재실행하면 race window 가 사라져 정상 동작했던 증상의
//! 근본 원인.
//!
//! 동작:
//! - `mark_ready()` — 모든 워크스페이스 mount 완료 후 1회 호출(멱등). main.rs 의
//!   `ReadyGuard` (`Drop` 트레잇) 에서 호출하므로 panic/early-return 시에도 발화 보장.
//! - `wait_for_ready(timeout)` — IPC handler 가 처리 진입 직후 호출. 이미 ready 면
//!   즉시 통과, 아직이면 timeout 까지 대기. timeout 후에도 panic 없이 반환해
//!   호출측이 기존 동작(빈 windows 매칭 → target=None) 으로 진행하도록 한다.
//!
//! 위치 사유: `AppState`(`crates/workspace/src/workspace.rs`) 에 필드를 두려면
//! workspace 크레이트에 `tokio` 또는 `watch` 의존성을 추가해야 한다 — CLAUDE.md
//! "의존성 추가" 승인 사항. zed 크레이트는 이미 `watch` 의존성을 가지므로 의존성
//! 변경 없이 글로벌로 둔다. 본 신호는 본체 lifecycle 전체에 1회만 발화되므로
//! `static` 보관이 안전하고 단순하다.

use std::{
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use futures::FutureExt as _;
use gpui::AsyncApp;
use parking_lot::Mutex;

/// 한 번이라도 ready 가 발화됐는지 atomic 으로 빠르게 검사. 두 번째 IPC 부터는
/// watch 채널 borrow 비용도 들이지 않고 즉시 통과한다.
static READY_FLAG: AtomicBool = AtomicBool::new(false);

/// watch::Sender 는 send 시 `&mut self` 를 요구하므로 Mutex 로 감싼다. lock 은
/// mark_ready() 1회 + 각 wait_for_ready() 진입에서 receiver 생성 시 1회만 발생해
/// 경쟁 비용 무시 가능.
static SENDER: OnceLock<Mutex<watch::Sender<bool>>> = OnceLock::new();

fn sender() -> &'static Mutex<watch::Sender<bool>> {
    SENDER.get_or_init(|| {
        let (tx, _rx) = watch::channel(false);
        Mutex::new(tx)
    })
}

/// 워크스페이스 첫 mount 완료를 알린다. 멱등 — 이미 ready 면 no-op.
/// `restore_or_create_workspace_inner` 의 종단 또는 `ReadyGuard::drop` 에서 호출.
pub fn mark_ready() {
    if READY_FLAG.swap(true, Ordering::AcqRel) {
        return; // 이미 ready
    }
    let _ = sender().lock().send(true);
    log::debug!("[workspace-ready] mark_ready() — IPC handler 가 즉시 통과 가능");
}

/// 워크스페이스 ready 까지 최대 `timeout` 만큼 대기. 이미 ready 면 즉시 반환.
/// timeout 초과 시에도 panic 없이 `WaitResult::Timeout` 반환 — 호출측이 기존
/// 동작(빈 windows 매칭 → target=None) 으로 진행하도록 한다.
pub async fn wait_for_ready(timeout: Duration, cx: &AsyncApp) -> WaitResult {
    if READY_FLAG.load(Ordering::Acquire) {
        return WaitResult::AlreadyReady;
    }
    // receiver 생성과 mark_ready 사이의 race 가드: receiver 생성 후 borrow 로
    // 현재 값을 다시 한 번 검사. (lock 보유 중에는 send 가 차단되지만, lock
    // 해제와 borrow 사이에 mark_ready 가 끼어들 수 있어 명시적으로 확인)
    let mut rx = sender().lock().receiver();
    if *rx.borrow() {
        return WaitResult::AlreadyReady;
    }
    // gpui scheduler 등록 timer 사용 — clippy.toml 이 smol::Timer::after 를 차단.
    let timer = cx.background_executor().timer(timeout).fuse();
    let changed = rx.changed().fuse();
    futures::pin_mut!(timer, changed);
    futures::select_biased! {
        result = changed => match result {
            Ok(()) => WaitResult::Notified,
            // sender 가 drop 됐다 — 본체 종료 단계. 호출측이 그냥 진행하도록.
            Err(_) => WaitResult::Timeout,
        },
        _ = timer => WaitResult::Timeout,
    }
}

/// `wait_for_ready` 결과 — 진단 로그 분기에 사용.
#[derive(Debug, Clone, Copy)]
pub enum WaitResult {
    /// 호출 시점에 이미 ready — 정상 (재실행 또는 두 번째 IPC).
    AlreadyReady,
    /// 대기 중에 ready 신호 도착 — 첫 실행 race 닫음.
    Notified,
    /// timeout 초과 — extreme edge case (restore panic 등).
    Timeout,
}
