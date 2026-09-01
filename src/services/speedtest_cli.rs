use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use chrono::Utc;

#[derive(Serialize, Deserialize, Debug)]
pub struct SpeedtestResult {
    pub download_mbps: f64,
    pub upload_mbps: f64,
    pub ping_ms: f64,
    pub server_name: String,
}

pub async fn run_speedtest(db_pool: &SqlitePool) -> Result<SpeedtestResult, String> {
    // Jalankan speedtest-cli di thread pool terpisah (spawn_blocking) agar
    // tidak memblokir Tokio's async runtime. Tanpa ini, seluruh dashboard akan
    // freeze selama speedtest berjalan (30-120 detik) karena blocking
    // std::process::Command menyita executor thread dan mencegah request lain
    // (ping, WebSocket metrics, dll.) diproses.
    let output = tokio::task::spawn_blocking(|| {
        std::process::Command::new("speedtest-cli")
            .args(["--json"])
            .output()
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e| format!("Failed to execute speedtest-cli: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Speedtest error: {}", err));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse speedtest output: {}", e))?;

    let download_bps = json["download"].as_f64().unwrap_or(0.0);
    let upload_bps = json["upload"].as_f64().unwrap_or(0.0);
    let ping_ms = json["ping"].as_f64().unwrap_or(0.0);
    let server_name = json["server"]["name"].as_str().unwrap_or("Unknown").to_string();

    let result = SpeedtestResult {
        download_mbps: download_bps / 1_000_000.0,
        upload_mbps: upload_bps / 1_000_000.0,
        ping_ms,
        server_name,
    };

    // Simpan ke database
    let now = Utc::now().to_rfc3339();
    let _ = sqlx::query(
        "INSERT INTO speedtest_history (tested_at, download_mbps, upload_mbps, ping_ms, server_name) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(now)
    .bind(result.download_mbps)
    .bind(result.upload_mbps)
    .bind(result.ping_ms)
    .bind(result.server_name.clone())
    .execute(db_pool)
    .await
    .map_err(|e| format!("DB Insert error: {}", e))?;

    // Pertahankan hanya 5 data terakhir (FIFO — hapus yang lebih lama)
    let _ = sqlx::query(
        "DELETE FROM speedtest_history WHERE id NOT IN (SELECT id FROM speedtest_history ORDER BY id DESC LIMIT 5)"
    )
    .execute(db_pool)
    .await;

    Ok(result)
}
