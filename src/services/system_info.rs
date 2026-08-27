use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Serialize, Deserialize, Debug)]
pub struct SystemInfo {
    pub hostname: Option<String>,
    pub os_name: Option<String>,
    pub kernel_version: Option<String>,
    pub uptime: u64,
    pub cpu_cores: usize,
    pub global_cpu_usage: f32,
    pub total_memory: u64,
    pub used_memory: u64,
}

pub fn get_system_info(sys: &mut System) -> SystemInfo {
    sys.refresh_all();
    
    SystemInfo {
        hostname: System::host_name(),
        os_name: System::name(),
        kernel_version: System::kernel_version(),
        uptime: System::uptime(),
        cpu_cores: sys.cpus().len(),
        global_cpu_usage: sys.global_cpu_usage(),
        total_memory: sys.total_memory(),
        used_memory: sys.used_memory(),
    }
}