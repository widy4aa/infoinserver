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

-- activity_log (General alert & logs)
CREATE TABLE IF NOT EXISTS activity_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    level TEXT NOT NULL DEFAULT 'INFO', -- INFO, WARNING, CRITICAL
    action TEXT NOT NULL,
    detail TEXT
);

-- system_metrics_history (Tiap 5 menit untuk chart)
CREATE TABLE IF NOT EXISTS system_metrics_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    cpu_usage REAL NOT NULL,
    mem_used_bytes INTEGER NOT NULL,
    mem_total_bytes INTEGER NOT NULL,
    disk_used_bytes INTEGER DEFAULT 0,
    disk_total_bytes INTEGER DEFAULT 0,
    net_rx_bytes INTEGER DEFAULT 0,
    net_tx_bytes INTEGER DEFAULT 0
);

-- cloudflare_cname_status (Status CNAME DNS Cloudflare Tunnel)
CREATE TABLE IF NOT EXISTS cloudflare_cname_status (
    hostname TEXT PRIMARY KEY,
    tunnel_name TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    added_at TEXT NOT NULL
);

-- github_users (Daftar GitHub user yang pernah login + presence tracking)
CREATE TABLE IF NOT EXISTS github_users (
    username   TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    avatar_url TEXT NOT NULL,
    last_seen  INTEGER NOT NULL DEFAULT 0  -- Unix timestamp, update tiap heartbeat
);

-- vm_instances (Podman nested containers: Ubuntu 24.04, Debian 12, Arch)
CREATE TABLE IF NOT EXISTS vm_instances (
    id           TEXT PRIMARY KEY,           -- UUID v4
    name         TEXT UNIQUE NOT NULL,       -- nama VM (slug, juga dipakai sebagai container name suffix)
    distro       TEXT NOT NULL,              -- 'ubuntu-2404' | 'debian-12' | 'arch'
    username     TEXT NOT NULL,              -- user biasa yang dibuat di dalam container
    backend_port INTEGER NOT NULL,           -- port di host yang di-forward ke container:8080
    host_ip      TEXT NOT NULL DEFAULT '',   -- IP host untuk dashboard URL
    container_id TEXT NOT NULL DEFAULT '',   -- Podman container name (infoinserver-vm-<name>)
    image_tag    TEXT NOT NULL DEFAULT '',   -- image yang dipakai (localhost/infoinserver-vm-<distro>:latest)
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);
