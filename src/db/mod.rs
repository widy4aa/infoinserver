use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    SqlitePool,
};
use std::str::FromStr;

pub async fn init_db(db_url: &str) -> Result<SqlitePool, sqlx::Error> {
    // Gunakan SqliteConnectOptions untuk kontrol penuh atas koneksi:
    // - journal_mode(Wal)    → fix SQLITE_READONLY_DBMOVED (code 1032), WAL tidak
    //                          melakukan inode-check saat write, dan concurrent
    //                          readers tidak diblokir oleh writer
    // - create_if_missing    → buat file DB otomatis jika belum ada (menggantikan
    //                          manual fs::File::create di atas)
    // - busy_timeout(5s)     → retry otomatis sampai 5 detik jika ada lock contention
    //                          sebelum return error, bukan langsung gagal
    let conn_opts = SqliteConnectOptions::from_str(db_url)
        .map_err(|e| sqlx::Error::Configuration(e.to_string().into()))?
        .journal_mode(SqliteJournalMode::Wal)
        .create_if_missing(true)
        .busy_timeout(std::time::Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(conn_opts)
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

    // ── vm_instances incremental migrations ──────────────────────────────────
    // Migrasi dari schema lama (socat_pid) ke schema baru (container_id, image_tag)

    // Tambah container_id jika belum ada
    let vm_cid: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('vm_instances') WHERE name = 'container_id'"
    ).fetch_all(&pool).await.unwrap_or_default();
    if vm_cid.is_empty() {
        let _ = sqlx::query(
            "ALTER TABLE vm_instances ADD COLUMN container_id TEXT NOT NULL DEFAULT ''"
        ).execute(&pool).await;
    }

    // Tambah image_tag jika belum ada
    let vm_img: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('vm_instances') WHERE name = 'image_tag'"
    ).fetch_all(&pool).await.unwrap_or_default();
    if vm_img.is_empty() {
        let _ = sqlx::query(
            "ALTER TABLE vm_instances ADD COLUMN image_tag TEXT NOT NULL DEFAULT ''"
        ).execute(&pool).await;
    }

    Ok(pool)
}
