// 앱 신뢰성 관련 유틸리티 — 로컬 hang 감지만 담당한다.
// 원본 Zed의 Sentry minidump 업로드, 빌드 타이밍 업로드, 원격 크래시 파일 수집 경로는
// 외부 서버로 데이터를 전송했기 때문에 Dokkaebi 포크에서는 모두 제거했다.
// 이 모듈은 main thread가 3초 이상 멈추면 로컬 로그 디렉터리에 스레드 타이밍을
// JSON으로 저장해 개발자가 로컬에서 조사할 수 있게 한다. 네트워크 전송은 일절 없다.

use anyhow::Context as _;
use client::Client;
use futures::StreamExt;
use gpui::{App, AppContext as _, SerializedThreadTaskTimings};
use log::info;
use std::{sync::Arc, thread::ThreadId, time::Duration};
use util::ResultExt;

use crate::STARTUP_TIME;

const MAX_HANG_TRACES: usize = 3;

/// reliability 초기화. Client 인자는 원본 API 호환 위해 유지한다.
pub fn init(_client: Arc<Client>, cx: &mut App) {
    if cfg!(debug_assertions) {
        log::info!("Debug assertions enabled, skipping hang monitoring");
    } else {
        monitor_hangs(cx);
    }
}

/// 메인 스레드 hang 감지기.
/// 1초마다 main thread에 "heartbeat"를 보내고, 응답이 3초 이상 밀리면 hang으로 판단한다.
/// 감지 시점의 전체 스레드 타이밍 스냅샷을 로컬 디렉터리에 저장할 뿐 서버 전송은 없다.
fn monitor_hangs(cx: &App) {
    let main_thread_id = std::thread::current().id();

    let foreground_executor = cx.foreground_executor();
    let background_executor = cx.background_executor();

    // 3초 hang 감지용 채널 (capacity 3).
    let (mut tx, mut rx) = futures::channel::mpsc::channel(3);
    foreground_executor
        .spawn(async move { while (rx.next().await).is_some() {} })
        .detach();

    background_executor
        .spawn({
            let background_executor = background_executor.clone();
            async move {
                cleanup_old_hang_traces();

                let mut hang_time = None;

                let mut hanging = false;
                loop {
                    background_executor.timer(Duration::from_secs(1)).await;
                    match tx.try_send(()) {
                        Ok(_) => {
                            hang_time = None;
                            hanging = false;
                            continue;
                        }
                        Err(e) => {
                            let is_full = e.into_send_error().is_full();
                            if is_full && !hanging {
                                hanging = true;
                                hang_time = Some(chrono::Local::now());
                            }

                            if is_full {
                                save_hang_trace(
                                    main_thread_id,
                                    &background_executor,
                                    hang_time.unwrap(),
                                );
                            }
                        }
                    }
                }
            }
        })
        .detach();
}

/// 오래된 hang trace 파일을 MAX_HANG_TRACES 개만 남기고 삭제한다.
fn cleanup_old_hang_traces() {
    if let Ok(entries) = std::fs::read_dir(paths::hang_traces_dir()) {
        let mut files: Vec<_> = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext == "json" || ext == "miniprof")
            })
            .collect();

        if files.len() > MAX_HANG_TRACES {
            files.sort_by_key(|entry| entry.file_name());
            for entry in files.iter().take(files.len() - MAX_HANG_TRACES) {
                std::fs::remove_file(entry.path()).log_err();
            }
        }
    }
}

/// hang 감지 시점의 전체 스레드 타이밍을 JSON으로 로컬에 저장한다.
fn save_hang_trace(
    main_thread_id: ThreadId,
    background_executor: &gpui::BackgroundExecutor,
    hang_time: chrono::DateTime<chrono::Local>,
) {
    let thread_timings = background_executor.dispatcher().get_all_timings();
    let thread_timings = thread_timings
        .into_iter()
        .map(|mut timings| {
            if timings.thread_id == main_thread_id {
                timings.thread_name = Some("main".to_string());
            }

            SerializedThreadTaskTimings::convert(*STARTUP_TIME.get().unwrap(), timings)
        })
        .collect::<Vec<_>>();

    let trace_path = paths::hang_traces_dir().join(&format!(
        "hang-{}.miniprof.json",
        hang_time.format("%Y-%m-%d_%H-%M-%S")
    ));

    let Some(timings) = serde_json::to_string(&thread_timings)
        .context("hang timings serialization")
        .log_err()
    else {
        return;
    };

    if let Ok(entries) = std::fs::read_dir(paths::hang_traces_dir()) {
        let mut files: Vec<_> = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext == "json" || ext == "miniprof")
            })
            .collect();

        if files.len() >= MAX_HANG_TRACES {
            files.sort_by_key(|entry| entry.file_name());
            for entry in files.iter().take(files.len() - (MAX_HANG_TRACES - 1)) {
                std::fs::remove_file(entry.path()).log_err();
            }
        }
    }

    std::fs::write(&trace_path, timings)
        .context("hang trace file writing")
        .log_err();

    info!(
        "hang detected, trace file saved at: {}",
        trace_path.display()
    );
}
