use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

/// 시스템 메트릭 데이터
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SystemMetrics {
    /// CPU 사용률 (0.0 ~ 100.0)
    pub cpu_usage: f32,
    /// 메모리 사용률 (0.0 ~ 100.0)
    pub memory_usage: f32,
    /// GPU 사용률 (None이면 GPU 미지원)
    pub gpu_usage: Option<f32>,
}

/// RAM만 갱신하는 메모리 리프레시 설정을 반환한다 (swap 제외).
fn memory_refresh() -> MemoryRefreshKind {
    MemoryRefreshKind::nothing().with_ram()
}

/// 백그라운드 스레드에서 시스템 메트릭을 수집하는 구조체
pub struct MetricsCollector {
    system: System,
    #[cfg(target_os = "windows")]
    gpu_query: Option<super::gpu_windows::GpuPdhQuery>,
}

impl MetricsCollector {
    /// 새 수집기를 생성한다.
    pub fn new() -> Self {
        let system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                .with_memory(memory_refresh()),
        );

        #[cfg(target_os = "windows")]
        let gpu_query = super::gpu_windows::GpuPdhQuery::new();

        Self {
            system,
            #[cfg(target_os = "windows")]
            gpu_query,
        }
    }

    /// 메트릭을 수집하여 반환한다.
    pub fn collect(&mut self) -> SystemMetrics {
        self.system.refresh_cpu_usage();
        let cpu_usage = self.system.global_cpu_usage();

        self.system.refresh_memory_specifics(memory_refresh());
        let total = self.system.total_memory();
        let used = self.system.used_memory();
        let memory_usage = if total > 0 {
            (used as f64 / total as f64 * 100.0) as f32
        } else {
            0.0
        };

        #[cfg(target_os = "windows")]
        let gpu_usage = self.gpu_query.as_mut().and_then(|q| q.collect());
        #[cfg(not(target_os = "windows"))]
        let gpu_usage = None;

        SystemMetrics {
            cpu_usage,
            memory_usage,
            gpu_usage,
        }
    }
}
