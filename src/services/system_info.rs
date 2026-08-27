use serde::{Deserialize, Serialize};
use sysinfo::{System, Disks};

#[derive(Serialize, Deserialize, Debug)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub total_space: u64,
    pub available_space: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SystemInfo {
    pub hostname: Option<String>,
    pub os_name: Option<String>,
    pub kernel_version: Option<String>,
    pub uptime: u64,
    pub cpu_cores: usize,
    pub cpu_model: String,
    pub global_cpu_usage: f32,
    pub total_memory: u64,
    pub used_memory: u64,
    pub disks: Vec<DiskInfo>,
}

pub fn get_system_info(sys: &mut System) -> SystemInfo {
    sys.refresh_all();
    
    let disks = Disks::new_with_refreshed_list();
    let mut disks_info = Vec::new();
    
    for disk in disks.list() {
        disks_info.push(DiskInfo {
            name: disk.name().to_string_lossy().to_string(),
            mount_point: disk.mount_point().to_string_lossy().to_string(),
            total_space: disk.total_space(),
            available_space: disk.available_space(),
        });
    }
    
    let cpu_model = if let Some(cpu) = sys.cpus().first() {
        cpu.brand().to_string()
    } else {
        "Unknown CPU".to_string()
    };
    
    SystemInfo {
        hostname: System::host_name(),
        os_name: System::name(),
        kernel_version: System::kernel_version(),
        uptime: System::uptime(),
        cpu_cores: sys.cpus().len(),
        cpu_model,
        global_cpu_usage: sys.global_cpu_usage(),
        total_memory: sys.total_memory(),
        used_memory: sys.used_memory(),
        disks: disks_info,
    }
}