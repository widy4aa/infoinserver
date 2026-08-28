use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::fs;

pub async fn init_db(db_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let db_path = db_url.replace("sqlite:", "");
    if !std::path::Path::new(&db_path).exists() {
        fs::File::create(&db_path).unwrap_or_else(|_| panic!("Failed to create db file at {}", db_path));
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await?;

    // Jalankan migrasi utama
    let migration_sql = include_str!("migrations.sql");
    sqlx::query(migration_sql).execute(&pool).await?;

    // ── Incremental ALTER TABLE migrations ──
    // SQLite tidak support IF NOT EXISTS pada ALTER TABLE,
    // jadi kita cek dulu apakah kolom sudah ada sebelum menambahkan.

    // Tambahkan kolom `level` ke activity_log jika belum ada
    let cols: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('activity_log') WHERE name = 'level'"
    ).fetch_all(&pool).await.unwrap_or_default();
    if cols.is_empty() {
        let _ = sqlx::query(
            "ALTER TABLE activity_log ADD COLUMN level TEXT NOT NULL DEFAULT 'INFO'"
        ).execute(&pool).await;
    }

    // Tambahkan kolom disk ke system_metrics_history jika belum ada
    let disk_cols: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('system_metrics_history') WHERE name = 'disk_used_bytes'"
    ).fetch_all(&pool).await.unwrap_or_default();
    if disk_cols.is_empty() {
        let _ = sqlx::query(
            "ALTER TABLE system_metrics_history ADD COLUMN disk_used_bytes INTEGER DEFAULT 0"
        ).execute(&pool).await;
        let _ = sqlx::query(
            "ALTER TABLE system_metrics_history ADD COLUMN disk_total_bytes INTEGER DEFAULT 0"
        ).execute(&pool).await;
    }

    // Tambahkan kolom network ke system_metrics_history jika belum ada
    let net_cols: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('system_metrics_history') WHERE name = 'net_rx_bytes'"
    ).fetch_all(&pool).await.unwrap_or_default();
    if net_cols.is_empty() {
        let _ = sqlx::query(
            "ALTER TABLE system_metrics_history ADD COLUMN net_rx_bytes INTEGER DEFAULT 0"
        ).execute(&pool).await;
        let _ = sqlx::query(
            "ALTER TABLE system_metrics_history ADD COLUMN net_tx_bytes INTEGER DEFAULT 0"
        ).execute(&pool).await;
    }

    Ok(pool)
}
