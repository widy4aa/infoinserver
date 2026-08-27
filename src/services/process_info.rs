use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
}

pub fn get_top_processes(sys: &mut System) -> Vec<ProcessInfo> {
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    
    let mut procs: Vec<ProcessInfo> = sys.processes()
        .iter()
        .map(|(pid, p)| ProcessInfo {
            pid: pid.as_u32(),
            name: p.name().to_string_lossy().to_string(),
            cpu_usage: p.cpu_usage(),
            memory_bytes: p.memory(),
        })
        .collect();
    
    // Urutkan berdasarkan pemakaian CPU terbesar
    procs.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal));
    
    // Ambil top 50 agar payload JSON tidak terlalu raksasa
    procs.into_iter().take(50).collect()
}
