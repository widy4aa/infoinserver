use tokio::time::{interval, Duration};
use sqlx::SqlitePool;
use chrono::Utc;
use crate::services::speedtest_cli::run_speedtest;
use crate::services::proc_reader::{get_system_metrics, read_network_interfaces};

pub async fn start_background_tasks(db_pool: SqlitePool) {
    let speedtest_pool = db_pool.clone();
    let metrics_pool = db_pool.clone();
    
    // 1. Task Speedtest (tiap 1 jam)
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(3600));
        loop {
            interval.tick().await;
            println!("Scheduler: Running scheduled speedtest...");
            match run_speedtest(&speedtest_pool).await {
                Ok(res) => println!("Scheduler: Speedtest success - D: {:.2} Mbps, U: {:.2} Mbps", res.download_mbps, res.upload_mbps),
                Err(e) => eprintln!("Scheduler: Speedtest failed - {}", e),
            }
        }
    });

    // 2. Task Metrik Riwayat & Alerting (tiap 5 menit)
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(300)); // 5 menit
        
        loop {
            interval.tick().await;
            
            // Baca metrik saat ini
            let metrics = get_system_metrics();
            let now = Utc::now().to_rfc3339();

            // Hitung agragat disk (total & used dari seluruh disk mount point)
            let mut total_disk_bytes: u64 = 0;
            let mut used_disk_bytes: u64 = 0;
            for disk in &metrics.disks {
                if disk.total_space > 0 {
                    total_disk_bytes += disk.total_space;
                    used_disk_bytes += disk.total_space.saturating_sub(disk.available_space);
                }
            }

            // Hitung agregat network RX dan TX (jumlahkan semua interface)
            let mut total_rx_bytes: u64 = 0;
            let mut total_tx_bytes: u64 = 0;
            let net_ifaces = read_network_interfaces();
            for iface in net_ifaces {
                if iface.name != "lo" { // Abaikan loopback
                    total_rx_bytes += iface.rx_bytes;
                    total_tx_bytes += iface.tx_bytes;
                }
            }

            // Insert ke history
            let insert_res = sqlx::query(
                "INSERT INTO system_metrics_history (timestamp, cpu_usage, mem_used_bytes, mem_total_bytes, disk_used_bytes, disk_total_bytes, net_rx_bytes, net_tx_bytes)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&now)
            .bind(metrics.global_cpu_usage)
            .bind(metrics.used_memory as i64)
            .bind(metrics.total_memory as i64)
            .bind(used_disk_bytes as i64)
            .bind(total_disk_bytes as i64)
            .bind(total_rx_bytes as i64)
            .bind(total_tx_bytes as i64)
            .execute(&metrics_pool)
            .await;

            if let Err(e) = insert_res {
                eprintln!("Scheduler: Failed to insert metrics history - {}", e);
            }

            // Bersihkan data lama (lebih dari 24 jam = 288 baris jika 5 menit)
            let _ = sqlx::query(
                "DELETE FROM system_metrics_history WHERE timestamp <= datetime('now', '-1 day')"
            ).execute(&metrics_pool).await;

            // ── Cek Threshold & Log Alerts ──
            
            // a) CPU Alert (> 90%)
            if metrics.global_cpu_usage > 90.0 {
                let _ = sqlx::query(
                    "INSERT INTO activity_log (timestamp, level, action, detail) VALUES (?, ?, ?, ?)"
                )
                .bind(&now)
                .bind("WARNING")
                .bind("High CPU Usage")
                .bind(format!("CPU Usage is at {:.1}%", metrics.global_cpu_usage))
                .execute(&metrics_pool).await;
            }

            // b) RAM Alert (> 90%)
            if metrics.total_memory > 0 {
                let mem_pct = (metrics.used_memory as f64 / metrics.total_memory as f64) * 100.0;
                if mem_pct > 90.0 {
                    let _ = sqlx::query(
                        "INSERT INTO activity_log (timestamp, level, action, detail) VALUES (?, ?, ?, ?)"
                    )
                    .bind(&now)
                    .bind("WARNING")
                    .bind("High Memory Usage")
                    .bind(format!("Memory Usage is at {:.1}% ({}/{} MB)", mem_pct, metrics.used_memory/1024/1024, metrics.total_memory/1024/1024))
                    .execute(&metrics_pool).await;
                }
            }

            // c) Disk Alert (< 10% free)
            for disk in metrics.disks {
                if disk.total_space > 0 {
                    let free_pct = (disk.available_space as f64 / disk.total_space as f64) * 100.0;
                    if free_pct < 10.0 {
                        let _ = sqlx::query(
                            "INSERT INTO activity_log (timestamp, level, action, detail) VALUES (?, ?, ?, ?)"
                        )
                        .bind(&now)
                        .bind("CRITICAL")
                        .bind("Low Disk Space")
                        .bind(format!("Mount point {} has only {:.1}% free space left", disk.mount_point, free_pct))
                        .execute(&metrics_pool).await;
                    }
                }
            }
        }
    });
}
