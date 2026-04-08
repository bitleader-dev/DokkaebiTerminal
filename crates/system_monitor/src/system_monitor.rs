use std::time::Duration;

use gpui::{Context, Render, Task, Window};
use settings::Settings;
use ui::prelude::*;
use workspace::{StatusItemView, WorkspaceSettings, item::ItemHandle};

use crate::metrics_collector::{MetricsCollector, SystemMetrics};

/// 상태표시줄 중앙에 CPU, 메모리, GPU 사용률을 실시간 표시하는 컴포넌트
pub struct SystemMonitor {
    metrics: SystemMetrics,
    _poll_task: Task<()>,
}

/// 메트릭 수집 주기 (2초)
const POLL_INTERVAL: Duration = Duration::from_secs(2);

impl SystemMonitor {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let poll_task = cx.spawn(async move |this, cx| {
            let mut collector = cx
                .background_executor()
                .spawn(async { MetricsCollector::new() })
                .await;

            loop {
                cx.background_executor().timer(POLL_INTERVAL).await;

                // 설정 비활성 시 수집 스킵
                let enabled = this
                    .update(cx, |_, cx| {
                        WorkspaceSettings::get(None, cx).system_monitoring
                    })
                    .unwrap_or(false);
                if !enabled {
                    continue;
                }

                let (metrics, coll) = cx
                    .background_executor()
                    .spawn(async move {
                        let m = collector.collect();
                        (m, collector)
                    })
                    .await;
                collector = coll;

                // 표시 값이 변경된 경우에만 리렌더링
                let result = this.update(cx, |this, cx| {
                    if display_changed(&this.metrics, &metrics) {
                        this.metrics = metrics;
                        cx.notify();
                    }
                });
                if result.is_err() {
                    break;
                }
            }
        });

        Self {
            metrics: SystemMetrics::default(),
            _poll_task: poll_task,
        }
    }
}

/// 표시되는 정수 값 기준으로 변경 여부 판정
fn display_changed(old: &SystemMetrics, new: &SystemMetrics) -> bool {
    old.cpu_usage as u32 != new.cpu_usage as u32
        || old.memory_usage as u32 != new.memory_usage as u32
        || old.gpu_usage.map(|g| g as u32) != new.gpu_usage.map(|g| g as u32)
}

/// 라벨+수치 쌍을 렌더링하는 헬퍼
fn metric_chip(label: &'static str, value: u32) -> impl IntoElement {
    h_flex()
        .gap_0p5()
        .child(Label::new(label).size(LabelSize::Small).color(Color::Muted))
        .child(Label::new(format!("{value}%")).size(LabelSize::Small))
}

impl Render for SystemMonitor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let enabled = WorkspaceSettings::get(None, cx).system_monitoring;
        if !enabled {
            return div().into_any_element();
        }

        h_flex()
            .gap_2()
            .child(metric_chip("CPU", self.metrics.cpu_usage as u32))
            .child(metric_chip("MEM", self.metrics.memory_usage as u32))
            .when_some(self.metrics.gpu_usage, |el, gpu| {
                el.child(metric_chip("GPU", gpu as u32))
            })
            .into_any_element()
    }
}

impl StatusItemView for SystemMonitor {
    fn set_active_pane_item(
        &mut self,
        _active_pane_item: Option<&dyn ItemHandle>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        // 시스템 메트릭은 패널과 무관하므로 no-op
    }
}
