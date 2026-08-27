use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::fs;

pub async fn init_db(db_url: &str) -> Result<SqlitePool, sqlx::Error> {
    // Pastikan file SQLite ada atau buat yang baru
    let db_path = db_url.replace("sqlite:", "");
    if !std::path::Path::new(&db_path).exists() {
        fs::File::create(&db_path).unwrap_or_else(|_| panic!("Failed to create db file at {}", db_path));
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await?;

    // Jalankan migrasi
    let migration_sql = include_str!("migrations.sql");
    sqlx::query(migration_sql).execute(&pool).await?;

    // Tambahan tabel cloudflare
    let cf_sql = include_str!("migrations_cf.sql");
    sqlx::query(cf_sql).execute(&pool).await?;

    Ok(pool)
}
