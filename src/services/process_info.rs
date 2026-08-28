use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
}

pub fn get_top_processes(sys: &mut System) -> Vec<ProcessInfo> {
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    
    let procs: Vec<ProcessInfo> = sys.processes()
        .iter()
        .map(|(pid, p)| ProcessInfo {
            pid: pid.as_u32(),
            name: p.name().to_string_lossy().to_string(),
            cpu_usage: p.cpu_usage(),
            memory_bytes: p.memory(),
        })
        .collect();
    
    // Sort & ambil 50 tertinggi berdasarkan CPU
    let mut top_cpu = procs.clone();
    top_cpu.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal));
    let mut result: Vec<ProcessInfo> = top_cpu.into_iter().take(50).collect();

    // Sort & ambil 50 tertinggi berdasarkan RAM
    let mut top_ram = procs;
    top_ram.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
    
    // Gabungkan, pastikan tidak ada duplikasi PID
    for p in top_ram.into_iter().take(50) {
        if !result.iter().any(|r| r.pid == p.pid) {
            result.push(p);
        }
    }
    
    // Hasil akhirnya adalah gabungan proses yang menonjol di CPU ATAU RAM (max 100 proses)
    result
}
