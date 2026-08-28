// system_info.rs — thin wrapper, re-export proc_reader agar existing code tidak perlu ubah import
pub use crate::services::proc_reader::{SystemMetrics, DiskInfo, get_system_metrics};

pub fn get_system_info() -> SystemMetrics {
    get_system_metrics()
}
