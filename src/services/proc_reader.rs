// src/services/proc_reader.rs
// Membaca metrics langsung dari /proc dan /sys filesystem Linux
// Zero external dependency untuk system metrics — mirip cara kerja Node Exporter
//
// Data yang dibaca:
//   /proc/stat         → CPU counters
//   /proc/meminfo      → Memory info
//   /proc/uptime       → Uptime
//   /proc/net/dev      → Network RX/TX
//   /proc/diskstats    → Disk I/O stats
//   /proc/mounts       → Mount points aktif
//   /proc/sys/kernel/hostname → Hostname
//   /etc/os-release    → OS name
//   /proc/version      → Kernel version

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self};
use std::collections::HashMap;
use std::sync::Mutex;

// ── CPU delta state ──────────────────────────────────────────
// Harus simpan snapshot sebelumnya untuk hitung usage %
#[derive(Clone, Debug, Default)]
pub struct CpuSnapshot {
    pub idle: u64,
    pub total: u64,
}

// Global state untuk CPU delta — protected by Mutex
static LAST_CPU: Mutex<Option<CpuSnapshot>> = Mutex::new(None);

// ── Structs output ────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub total_space: u64,
    pub available_space: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub ip_networks: Vec<String>,
    pub mac_address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemMetrics {
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
    pub current_user: String,
}

// ── Readers ───────────────────────────────────────────────────

/// Baca hostname dari /proc/sys/kernel/hostname
pub fn read_hostname() -> Option<String> {
    fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Baca OS name dari /etc/os-release
pub fn read_os_name() -> Option<String> {
    let content = fs::read_to_string("/etc/os-release").ok()?;
    for line in content.lines() {
        if line.starts_with("PRETTY_NAME=") {
            return Some(
                line.trim_start_matches("PRETTY_NAME=")
                    .trim_matches('"')
                    .to_string(),
            );
        }
    }
    None
}

/// Baca kernel version dari /proc/version
pub fn read_kernel_version() -> Option<String> {
    let content = fs::read_to_string("/proc/version").ok()?;
    // Format: "Linux version 6.x.x-arch ..."
    let version = content.split_whitespace().nth(2)?;
    Some(version.to_string())
}

/// Baca uptime dari /proc/uptime (format: "uptime_seconds idle_seconds")
pub fn read_uptime() -> u64 {
    fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| {
            s.split_whitespace()
                .next()
                .and_then(|v| v.parse::<f64>().ok())
        })
        .unwrap_or(0.0) as u64
}

/// Baca CPU usage dengan metode delta (seperti Node Exporter)
/// /proc/stat format: cpu  user nice system idle iowait irq softirq steal guest guest_nice
pub fn read_cpu_usage() -> (f32, usize) {
    let content = match fs::read_to_string("/proc/stat") {
        Ok(c) => c,
        Err(_) => return (0.0, 1),
    };

    let mut cores = 0usize;
    let mut agg_idle = 0u64;
    let mut agg_total = 0u64;

    for line in content.lines() {
        if line.starts_with("cpu ") {
            // Aggregate CPU line
            let parts: Vec<u64> = line
                .split_whitespace()
                .skip(1)
                .filter_map(|v| v.parse().ok())
                .collect();
            if parts.len() >= 4 {
                let total: u64 = parts.iter().sum();
                let idle = parts[3] + parts.get(4).copied().unwrap_or(0); // idle + iowait
                agg_idle = idle;
                agg_total = total;
            }
        } else if line.starts_with("cpu") && !line.starts_with("cpu ") {
            cores += 1;
        }
    }

    if cores == 0 { cores = 1; }

    // Hitung delta dari snapshot sebelumnya
    let usage = {
        let mut last = LAST_CPU.lock().unwrap();
        let prev = last.clone().unwrap_or_default();
        let delta_total = agg_total.saturating_sub(prev.total);
        let delta_idle = agg_idle.saturating_sub(prev.idle);
        *last = Some(CpuSnapshot { idle: agg_idle, total: agg_total });

        if delta_total == 0 {
            0.0f32
        } else {
            let busy = delta_total - delta_idle;
            (busy as f32 / delta_total as f32) * 100.0
        }
    };

    (usage, cores)
}

/// Baca CPU model dari /proc/cpuinfo
pub fn read_cpu_model() -> String {
    fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|l| l.starts_with("model name"))
                .and_then(|l| l.split(':').nth(1))
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|| "Unknown CPU".to_string())
}

/// Baca memory dari /proc/meminfo — return (total_bytes, used_bytes)
pub fn read_memory() -> (u64, u64) {
    let content = match fs::read_to_string("/proc/meminfo") {
        Ok(c) => c,
        Err(_) => return (0, 0),
    };

    let mut map: HashMap<&str, u64> = HashMap::new();
    for line in content.lines() {
        let mut parts = line.splitn(2, ':');
        if let (Some(key), Some(val)) = (parts.next(), parts.next()) {
            let kb: u64 = val
                .trim()
                .split_whitespace()
                .next()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            map.insert(key, kb);
        }
    }

    let total_kb = map.get("MemTotal").copied().unwrap_or(0);
    let avail_kb = map
        .get("MemAvailable")
        .copied()
        .unwrap_or_else(|| {
            let free = map.get("MemFree").copied().unwrap_or(0);
            let buffers = map.get("Buffers").copied().unwrap_or(0);
            let cached = map.get("Cached").copied().unwrap_or(0);
            free + buffers + cached
        });

    let total = total_kb * 1024;
    let used = total.saturating_sub(avail_kb * 1024);
    (total, used)
}

/// Baca disk info via statvfs syscall (libc) untuk setiap mount point
pub fn read_disks() -> Vec<DiskInfo> {
    let mounts = match fs::read_to_string("/proc/mounts") {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut disks = Vec::new();
    let mut seen_devices: std::collections::HashSet<String> = std::collections::HashSet::new();

    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 { continue; }
        let device = parts[0];
        let mount_point = parts[1];

        // Skip pseudo/virtual filesystems
        if !device.starts_with('/') { continue; }
        // Skip duplicate devices
        if !seen_devices.insert(device.to_string()) { continue; }

        let mount_cstr = match std::ffi::CString::new(mount_point) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
        let ret = unsafe { libc::statvfs(mount_cstr.as_ptr(), &mut stat) };
        if ret != 0 { continue; }

        let block_size = stat.f_frsize as u64;
        let total = stat.f_blocks as u64 * block_size;
        let available = stat.f_bavail as u64 * block_size;

        if total == 0 { continue; }

        // Ambil nama device (e.g. /dev/sda1 → sda1)
        let name = device.rsplit('/').next().unwrap_or(device).to_string();

        disks.push(DiskInfo {
            name,
            mount_point: mount_point.to_string(),
            total_space: total,
            available_space: available,
        });
    }

    disks
}

/// Baca network interfaces dari /proc/net/dev
pub fn read_network_interfaces() -> Vec<NetworkInterface> {
    let content = match fs::read_to_string("/proc/net/dev") {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut ifaces = Vec::new();

    for line in content.lines().skip(2) {
        let line = line.trim();
        let colon_pos = match line.find(':') {
            Some(p) => p,
            None => continue,
        };
        let name = line[..colon_pos].trim().to_string();
        let values: Vec<u64> = line[colon_pos + 1..]
            .split_whitespace()
            .filter_map(|v| v.parse().ok())
            .collect();

        if values.len() < 9 { continue; }

        let rx_bytes = values[0];
        let tx_bytes = values[8];

        // Baca IP dari /proc/net/if_inet6 + /proc/net/fib_trie (simplified)
        // Untuk simplicity, baca via /sys/class/net/<iface>/address untuk MAC
        let mac = fs::read_to_string(format!("/sys/class/net/{}/address", name))
            .unwrap_or_default()
            .trim()
            .to_string();

        ifaces.push(NetworkInterface {
            name,
            rx_bytes,
            tx_bytes,
            ip_networks: vec![], // IP akan di-resolve di route handler jika dibutuhkan
            mac_address: mac,
        });
    }

    ifaces
}

/// Entry point utama — kumpulkan semua metrics
pub fn get_system_metrics() -> SystemMetrics {
    let (cpu_usage, cpu_cores) = read_cpu_usage();
    let (total_memory, used_memory) = read_memory();
    let current_user = std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "unknown".to_string());

    SystemMetrics {
        hostname: read_hostname(),
        os_name: read_os_name(),
        kernel_version: read_kernel_version(),
        uptime: read_uptime(),
        cpu_cores,
        cpu_model: read_cpu_model(),
        global_cpu_usage: cpu_usage,
        total_memory,
        used_memory,
        disks: read_disks(),
        current_user,
    }
}
