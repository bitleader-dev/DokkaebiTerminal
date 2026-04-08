mod metrics_collector;
mod system_monitor;

#[cfg(target_os = "windows")]
mod gpu_windows;

pub use system_monitor::SystemMonitor;
