-- speedtest_history
CREATE TABLE IF NOT EXISTS speedtest_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tested_at TEXT NOT NULL,   -- ISO8601
    download_mbps REAL,
    upload_mbps REAL,
    ping_ms REAL,
    server_name TEXT
);

-- port_scan_jobs
CREATE TABLE IF NOT EXISTS port_scan_jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    target TEXT NOT NULL,
    status TEXT NOT NULL,      -- pending | running | done | failed
    started_at TEXT,
    finished_at TEXT,
    result_json TEXT           -- daftar port terbuka hasil scan
);

-- activity_log
CREATE TABLE IF NOT EXISTS activity_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    action TEXT NOT NULL,
    detail TEXT
);
