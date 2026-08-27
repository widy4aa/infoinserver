use std::process::Command;
use sqlx::SqlitePool;
use chrono::Utc;

// Struct used strictly for JSON Responses in frontend (moved to routes)
// pub struct PortScanJob { ... }

// Menjalankan Nmap sebagai background task dan mengupdate DB
pub async fn run_nmap_scan(db_pool: SqlitePool, job_id: i64, target: String) {
    // Update status to running
    let now = Utc::now().to_rfc3339();
    let _ = sqlx::query(
        "UPDATE port_scan_jobs SET status = 'running', started_at = ? WHERE id = ?"
    )
    .bind(now)
    .bind(job_id)
    .execute(&db_pool)
    .await;

    // Execute nmap
    // Gunakan T4 untuk kecepatan wajar, batasi ke common ports dulu agar tidak terlalu lama di prototype, atau bisa -p- 
    let output = Command::new("nmap")
        .args(["-p", "1-1000", "-T4", &target])
        .output();

    let finished_at = Utc::now().to_rfc3339();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            
            // Sederhananya, simpan raw text sebagai JSON string array, atau parse portnya
            // Di sini kita simpan raw stdout di dalam JSON wrapper untuk simplisitas
            let result_val = serde_json::json!({
                "raw_output": stdout.to_string()
            });

            let result_str = serde_json::to_string(&result_val).unwrap_or_default();

            let _ = sqlx::query(
                "UPDATE port_scan_jobs SET status = 'done', finished_at = ?, result_json = ? WHERE id = ?"
            )
            .bind(finished_at)
            .bind(result_str)
            .bind(job_id)
            .execute(&db_pool)
            .await;
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let _ = sqlx::query(
                "UPDATE port_scan_jobs SET status = 'failed', finished_at = ?, result_json = ? WHERE id = ?"
            )
            .bind(finished_at)
            .bind(stderr.to_string())
            .bind(job_id)
            .execute(&db_pool)
            .await;
        }
        Err(e) => {
            let err_msg = format!("Failed to spawn nmap: {}", e);
            let _ = sqlx::query(
                "UPDATE port_scan_jobs SET status = 'failed', finished_at = ?, result_json = ? WHERE id = ?"
            )
            .bind(finished_at)
            .bind(err_msg)
            .bind(job_id)
            .execute(&db_pool)
            .await;
        }
    }
}
