use tokio::time::{interval, Duration};
use sqlx::SqlitePool;
use crate::services::speedtest_cli::run_speedtest;

pub async fn start_background_tasks(db_pool: SqlitePool) {
    let pool_clone = db_pool.clone();
    
    // Spawn task speedtest (tiap 1 jam)
    tokio::spawn(async move {
        // Interval: 1 jam
        let mut interval = interval(Duration::from_secs(3600));
        
        loop {
            interval.tick().await;
            println!("Scheduler: Running scheduled speedtest...");
            match run_speedtest(&pool_clone).await {
                Ok(res) => println!("Scheduler: Speedtest success - D: {:.2} Mbps, U: {:.2} Mbps", res.download_mbps, res.upload_mbps),
                Err(e) => eprintln!("Scheduler: Speedtest failed - {}", e),
            }
        }
    });
}
