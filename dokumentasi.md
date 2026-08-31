# Dokumentasi Internal — InfoIn Server

Dokumen ini ditujukan bagi **developer** dan **AI Agent** yang akan memelihara, membaca, mengembangkan, atau men-debug proyek **InfoIn Server**. Dokumen ini bersifat teknis, berbahasa Indonesia, dan mencakup arsitektur, struktur file, cara kerja fitur, skema database, referensi API, serta catatan keamanan.

---

## 1. Gambaran Arsitektur

InfoIn Server menggunakan arsitektur **Decoupled Backend-Frontend**:

```
┌─────────────────────────────────────────────────────────────────────┐
│  Browser (Vue 3 SPA)                                                │
│  - Komunikasi via HTTP REST + 4 channel WebSocket                   │
│  - Token JWT disimpan di sessionStorage per-server                  │
│  - State server (daftar, token) disimpan di localStorage            │
└──────────────────────────────┬──────────────────────────────────────┘
                               │ HTTP / WebSocket
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│  Backend Rust (Axum 0.8 + Tokio)                                    │
│  - REST API + WebSocket handlers                                     │
│  - PAM authentication + JWT middleware                               │
│  - Baca kernel: /proc, /sys, /dev                                   │
│  - Eksekusi OS command via sudo                                      │
│  - SQLite via SQLx (async)                                           │
│  - Background scheduler (tokio::spawn)                              │
└─────────────────────────────────────────────────────────────────────┘
```

### State Management Backend

Backend menggunakan 2 tipe state yang diinjeksikan ke handler via Axum `State`:

```rust
// AppState — untuk sebagian besar handler
pub struct AppState {
    sys: Arc<Mutex<System>>,       // sysinfo, untuk metrics CPU/RAM
    networks: Arc<Mutex<Networks>>,// sysinfo, untuk statistik network
    db_pool: sqlx::SqlitePool,     // koneksi database SQLite
}

// ContainerState — khusus container/compose handler
pub struct ContainerState {
    pub runtime: Arc<RwLock<Option<RuntimeInfo>>>,
}
```

### Background Tasks

Dijalankan sekali saat startup via `background::scheduler::start_background_tasks()`:
- **Rekam metrics** ke `system_metrics_history` setiap **5 menit**
- **Deteksi anomali** CPU > 90%, RAM > 90%, Disk sisa < 10% — tulis ke `activity_log`

---

## 2. Struktur Direktori Lengkap

```
.
├── Cargo.toml                    # Dependensi Rust
├── Cargo.lock                    # Lock file versi dependensi
├── .env                          # Konfigurasi environment
├── start.sh                      # Build release + jalankan daemon background
├── stop.sh                       # Hentikan daemon yang sedang berjalan
├── server.log                    # Output log daemon backend
├── data.db                       # Database SQLite (dibuat otomatis)
├── docker-compose.yml            # Deployment frontend via Nginx/Podman
├── Dockerfile.frontend           # Multi-stage build: Vite → Nginx Alpine
├── static/                       # Output build Vue (dilayani backend sebagai fallback)
│
├── src/
│   ├── main.rs                   # Entry point: init state, CORS, route registration, server
│   │
│   ├── auth/
│   │   ├── mod.rs
│   │   ├── jwt.rs                # create_token(), verify_token(), struct Claims
│   │   ├── jwt_middleware.rs     # jwt_auth_middleware + struct AuthUser(Claims)
│   │   └── middleware.rs
│   │
│   ├── background/
│   │   ├── mod.rs
│   │   └── scheduler.rs          # Background: rekam metrics tiap 5 menit, alert anomali
│   │
│   ├── db/
│   │   ├── mod.rs                # init_db(): buat koneksi pool, jalankan migrations
│   │   ├── migrations.sql        # DDL 5 tabel utama
│   │   └── migrations_cf.sql     # DDL tambahan Cloudflare (legacy/kosong)
│   │
│   ├── services/                 # Layer logika bisnis & OS wrapper
│   │   ├── mod.rs
│   │   ├── proc_reader.rs        # Baca /proc/stat, /proc/meminfo, /proc/uptime, OS info
│   │   ├── network_info.rs       # Parse statistik network interface (rx/tx) + gateway
│   │   ├── system_info.rs        # Info sistem umum (hostname, os_name, kernel, user)
│   │   ├── process_info.rs       # Top process list via sysinfo
│   │   ├── port_scanner.rs       # Parse /proc/net/tcp untuk listening ports
│   │   ├── nmap_scanner.rs       # Jalankan nmap, simpan job ke DB, poll hasil
│   │   ├── file_manager.rs       # resolve_path_safe(), check_write_permission(), list_directory()
│   │   ├── compose_manager.rs    # Logic Docker/Podman Compose management
│   │   ├── container_runtime.rs  # Deteksi runtime, list/create/action container
│   │   ├── speedtest_cli.rs      # Jalankan speedtest-cli, parse hasil, simpan ke DB
│   │   └── podman_cli.rs         # (Legacy) CLI wrapper Podman lama
│   │
│   └── routes/                   # Layer HTTP handlers (24 modul)
│       ├── mod.rs                # Ekspor semua modul
│       ├── auth.rs               # Login via PAM, terbitkan JWT
│       ├── network.rs            # Info network interface
│       ├── container.rs          # Docker/Podman container management
│       ├── compose.rs            # Docker/Podman Compose management
│       ├── files.rs              # File manager: list, search, download, upload, actions
│       ├── users_mgmt.rs         # User, group, SSH key management
│       ├── services_mgmt.rs      # Systemd service management
│       ├── syslogs.rs            # journalctl viewer
│       ├── cron_mgmt.rs          # Crontab management root
│       ├── process_mgmt.rs       # Top proses + kill + sudo_exec() helper
│       ├── ports.rs              # Listening ports + nmap scan jobs
│       ├── disk.rs               # Block device info (lsblk), mount/umount
│       ├── speedtest.rs          # Speedtest runner + history dari DB
│       ├── firewall.rs           # UFW firewall management
│       ├── fail2ban.rs           # Fail2ban management (install, ban, unban, config, logs)
│       ├── cloudflare.rs         # Cloudflared binary, tunnel, login, service, WS logs
│       ├── cloudflare_api.rs     # Config YAML, routes, DNS CNAME, health check
│       ├── logs.rs               # Activity log DB + bash history + log_activity() utility
│       ├── metrics_ws.rs         # WebSocket live metrics (CPU, RAM, disk, net)
│       ├── metrics_history.rs    # Historical metrics dari DB
│       ├── system_mgmt.rs        # git pull + rebuild, reboot
│       ├── system_updates.rs     # apt/pacman update check + upgrade WebSocket
│       ├── terminal.rs           # Shellinabox handler
│       ├── terminal_ws.rs        # WebSocket PTY terminal (portable-pty + bash)
│       └── podman*.rs            # (Legacy) Podman endpoints lama
│
└── frontend-vue/
    ├── package.json              # Dependensi NPM
    ├── vite.config.js            # Vite: output ke ../static/, proxy ke :8080
    └── src/
        ├── main.js               # Entry point Vue, mount app
        ├── App.vue               # Root: global toast, dark mode provider
        ├── composables/
        │   └── useApi.js         # apiFetch(): inject Authorization header otomatis, auto-logout on 401/sudo-fail
        ├── stores/
        │   ├── serverStore.js    # Pinia: daftar server, JWT token per-server, active server
        │   ├── themeStore.js     # Pinia: dark/light mode toggle
        │   └── toastStore.js     # Pinia: toast notification + confirm dialog
        ├── utils/
        │   └── distro.js         # getDistroIcon(), getDistroColorClass() — mapping OS ke CDN icon
        ├── components/
        │   ├── NativeTerminal.vue    # xterm.js terminal over WebSocket
        │   ├── LoginModal.vue        # Modal login server
        │   ├── ToastAlert.vue        # Toast notification renderer
        │   └── CloudflarePanel.vue   # Sub-komponen Cloudflare (legacy)
        ├── router/
        │   └── index.js          # Vue Router SPA routes
        └── views/
            ├── HomeView.vue          # Daftar server dengan distro icon
            ├── ServerLayout.vue      # Layout: navigasi + ping indicator + terminal button
            ├── DashboardView.vue     # Live metrics + historical charts
            ├── ContainerView.vue     # Container & Compose management (3 tab)
            ├── FilesView.vue         # File manager (browse, search, grid/list/compact)
            ├── UsersView.vue         # User & group & SSH key management (2 tab)
            ├── ServicesView.vue      # Systemd services + process manager (2 tab)
            ├── PortsView.vue         # Network, ports, speedtest, firewall, fail2ban
            ├── CloudflareView.vue    # Cloudflare Tunnel Command Center (3 tab)
            ├── SyslogsView.vue       # System logs (journal, activity, bash history)
            ├── LogsView.vue          # Activity log dari database
            ├── CronView.vue          # Cron job manager
            ├── UpdatesView.vue       # OS update check + live upgrade terminal
            ├── SettingsView.vue      # Pengaturan server & app
            └── PodmanView.vue        # (Legacy) Podman view lama
```

---

## 3. Konfigurasi Environment (`.env`)

| Variabel | Nilai Default | Keterangan |
|----------|--------------|------------|
| `PORT` | `8080` | Port HTTP backend |
| `FILE_ROOT` | `$HOME` | Root direktori file manager. `$HOME` di-*expand* ke home OS user saat runtime |
| `DB_PATH` | `sqlite:./data.db` | Path file SQLite |
| `JWT_SECRET` | *(tidak ada default aman)* | Secret untuk sign/verify JWT. **WAJIB diisi** di production. Generate: `openssl rand -hex 32` |

**Variabel yang dibaca langsung dari environment OS (bukan `.env`):**
- `HOME` — Home directory OS user, dipakai untuk path cert cloudflared dan `$HOME` expand
- `USER` — Username OS aktif, dipakai untuk label di bash history viewer

---

## 4. Skema Database (SQLite)

Database dibuat otomatis via `src/db/migrations.sql` saat pertama kali backend dijalankan.

### Tabel `system_metrics_history`
Merekam data metrik sistem setiap 5 menit oleh background scheduler.
```sql
CREATE TABLE IF NOT EXISTS system_metrics_history (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp        TEXT NOT NULL,          -- ISO8601
    cpu_usage        REAL NOT NULL,          -- Persentase 0-100
    mem_used_bytes   INTEGER NOT NULL,
    mem_total_bytes  INTEGER NOT NULL,
    disk_used_bytes  INTEGER DEFAULT 0,
    disk_total_bytes INTEGER DEFAULT 0,
    net_rx_bytes     INTEGER DEFAULT 0,      -- Kumulatif bytes diterima
    net_tx_bytes     INTEGER DEFAULT 0       -- Kumulatif bytes dikirim
);
```

### Tabel `speedtest_history`
Menyimpan hasil internet speed test.
```sql
CREATE TABLE IF NOT EXISTS speedtest_history (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    tested_at     TEXT NOT NULL,    -- ISO8601
    download_mbps REAL,
    upload_mbps   REAL,
    ping_ms       REAL,
    server_name   TEXT
);
```

### Tabel `port_scan_jobs`
Job queue untuk nmap scan async.
```sql
CREATE TABLE IF NOT EXISTS port_scan_jobs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    target      TEXT NOT NULL,
    status      TEXT NOT NULL,   -- pending | running | done | failed
    started_at  TEXT,
    finished_at TEXT,
    result_json TEXT             -- JSON string hasil scan
);
```

### Tabel `activity_log`
Audit trail semua aksi admin via dashboard + alert anomali dari scheduler.
```sql
CREATE TABLE IF NOT EXISTS activity_log (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    level     TEXT NOT NULL DEFAULT 'INFO',  -- INFO | WARNING | CRITICAL
    action    TEXT NOT NULL,                 -- e.g. "Firewall Toggle", "Fail2Ban Ban"
    detail    TEXT                           -- Deskripsi detail
);
```

### Tabel `cloudflare_cname_status`
Status lokal registrasi DNS CNAME. Dipakai agar status CNAME langsung hijau tanpa menunggu propagasi DNS.
```sql
CREATE TABLE IF NOT EXISTS cloudflare_cname_status (
    hostname     TEXT PRIMARY KEY,
    tunnel_name  TEXT NOT NULL,
    is_active    BOOLEAN NOT NULL DEFAULT 1,
    added_at     TEXT NOT NULL    -- ISO8601
);
```

### Migrasi Incremental (`src/db/mod.rs`)
Beberapa kolom ditambahkan secara incremental (karena SQLite tidak mendukung `IF NOT EXISTS` pada `ALTER TABLE`):
- `activity_log.level` — ditambahkan via `pragma_table_info` check
- `system_metrics_history.disk_used_bytes`, `disk_total_bytes`, `net_rx_bytes`, `net_tx_bytes` — sama

---

## 5. Keamanan & Prinsip Penting

### Aturan yang Harus Selalu Dipatuhi

1. **Jangan pernah gunakan `sh -c` dengan string input dari user secara langsung.**
   Gunakan selalu `std::process::Command::new("...").args([...])` dengan argumen terpisah. Pengecualian hanya untuk string yang sudah di-sanitize ketat dan tidak mengandung input dinamis.

2. **Path traversal protection di File Manager.**
   Fungsi `resolve_path_safe()` di `src/services/file_manager.rs` memvalidasi path via `canonicalize()`. Jangan pernah bypass ini.

3. **Write permission check di File Manager.**
   Fungsi `check_write_permission()` memastikan write hanya diizinkan di `FILE_ROOT` (home) dan removable mount points. Endpoint write wajib memanggil ini sebelum eksekusi.

4. **JWT Secret di production.**
   `JWT_SECRET` harus di-set ke nilai acak yang kuat. Default fallback `"changeme-jwt-secret"` **tidak aman** untuk production.

5. **Jangan expose port backend ke internet tanpa TLS.**
   Backend tidak mengimplementasikan TLS sendiri. Gunakan reverse proxy (Nginx/Caddy) dengan HTTPS di production.

6. **CORS saat ini menggunakan `Any`** untuk kemudahan pengembangan. Pertimbangkan membatasi origin di production.

### Mekanisme Auto-Logout

Di `frontend-vue/src/composables/useApi.js`, fungsi `apiFetch()` menginspeksi respons:
- Jika status `401` → hapus token, redirect ke `/`
- Jika status `500` dan body mengandung string `"sudo: authentication failed"` → sama dengan 401

Ini mencegah pengguna yang token-nya kadaluarsa terus mendapat error misterius.

### Sudo Injection

Helper `sudo_exec()` di `src/routes/process_mgmt.rs`:
```rust
pub fn sudo_exec(password: &str, args: &[&str]) -> std::io::Result<Output> {
    let mut child = Command::new("sudo")
        .arg("-S")    // Baca password dari stdin
        .arg("-p").arg("")  // Kosongkan prompt
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(format!("{}\n", password).as_bytes());
    }
    child.wait_with_output()
}
```
Password diinjeksikan via stdin, **tidak pernah disimpan ke disk**, tidak pernah dikembalikan ke frontend, dan tidak pernah dilog.

---

## 6. Cara Kerja Fitur Utama

### 6.1 Autentikasi & Sesi

1. User kirim `POST /api/auth/login` dengan `{ username, password }`
2. Backend verifikasi via PAM (`libpam`) — ini adalah autentikasi PAM Linux sungguhan
3. Jika berhasil, backend buat JWT token yang berisi `sub` (username) dan `pwd` (password) — password dienkripsi di dalam JWT payload
4. Frontend simpan token di `sessionStorage` per server ID
5. Setiap request selanjutnya, `apiFetch()` menyertakan `Authorization: Bearer <token>`
6. `jwt_auth_middleware` memverifikasi token dan mengekstrak `Claims` yang berisi password
7. Handler yang butuh sudo mengambil `auth.0.pwd` dari `Claims` untuk `sudo_exec()`

> **Catatan penting:** Password user disimpan di dalam JWT token agar handler bisa melakukan `sudo`. Ini adalah trade-off desain yang disengaja — memungkinkan autentikasi sekali (login) tapi bisa menjalankan `sudo` di setiap request berikutnya. Akibatnya: **JWT Secret harus kuat**, dan **HTTPS wajib di production** agar token tidak bisa dicegat.

### 6.2 Streaming Metrics via WebSocket

Handler `metrics_ws_handler`:
1. Upgrade koneksi HTTP ke WebSocket
2. Spawn loop async yang berjalan setiap 3 detik
3. Setiap iterasi: lock `AppState.sys`, refresh sysinfo, baca `/proc/stat`, `/proc/meminfo`, `/proc/net/dev`
4. Serialize ke JSON, kirim via `socket.send(Message::Text(...))`
5. Jika `socket.recv()` menerima `Close` atau `None` → break loop

### 6.3 File Manager — Sistem Permission

Dua fungsi kunci di `src/services/file_manager.rs`:

**`resolve_path_safe(requested_path)`** — Validasi dan resolve path:
- Tolak path yang mengandung null byte
- Gunakan `Path::canonicalize()` untuk mendapat absolute path
- Tidak ada batasan ke FILE_ROOT — seluruh filesystem dapat diakses untuk *read*

**`check_write_permission(path, home_root, removable_mounts)`** — Cek izin write:
- Return `true` jika path berada di dalam `home_root` (FILE_ROOT)
- Return `true` jika path berada di dalam salah satu removable mount point (`/media/`, `/mnt/`)
- Return `false` untuk semua path lainnya

Semua endpoint *write* (`upload`, `file_action` dengan modifikasi, `text_file` mode write, `fetch_url`) **wajib** memanggil `check_write_permission()` sebelum eksekusi.

### 6.4 Cloudflare Tunnel — Alur Setup

```
Install cloudflared binary
        ↓
Run `cloudflared tunnel login`
Backend: spawn process, capture stderr, cari URL "https://dash.cloudflare.com"
Frontend: tampilkan URL untuk dibuka user di browser, polling /api/cloudflare/login/status
        ↓
User authorize di browser → cert.pem tersimpan di ~/.cloudflared/cert.pem
        ↓
Create tunnel: `cloudflared tunnel create <name>`
Backend: cp credentials .json ke /etc/cloudflared/, tulis config.yml awal,
         jalankan `cloudflared service install`, `systemctl enable`, `systemctl start`
        ↓
Tunnel aktif. User tambah route via UI.
        ↓
Add Route: update config.yml + restart service
        ↓
Register DNS CNAME: `cloudflared tunnel route dns <tunnel> <hostname>`
Backend: simpan status ke tabel `cloudflare_cname_status` jika berhasil
        ↓
Health Check: HTTP probe ke setiap hostname, klasifikasikan: HEALTHY / ERR_502 / ERR_1033 / NXDOMAIN
```

### 6.5 Activity Log (Audit Trail)

Fungsi utility `log_activity()` di `src/routes/logs.rs`:
```rust
pub async fn log_activity(pool: &SqlitePool, level: &str, action: &str, detail: &str) {
    let now = chrono::Utc::now().to_rfc3339();
    let _ = sqlx::query("INSERT INTO activity_log ...")
        .bind(now).bind(level).bind(action).bind(detail)
        .execute(pool).await;
}
```
Dipanggil dari berbagai route setelah operasi berhasil. Contoh:
- Firewall toggle → level `WARNING`
- Create user → level `INFO`
- Delete tunnel → level `WARNING`
- System reboot → level `CRITICAL`
- OS upgrade → level `WARNING`

### 6.6 Distro Icon System

File `frontend-vue/src/utils/distro.js`:
- Fungsi `getDistroIcon(osName)` memetakan string `os_name` (dari `/etc/os-release PRETTY_NAME`) ke URL ikon di `cdn.simpleicons.org`
- Fungsi `getDistroColorClass(osName)` mengembalikan kelas Tailwind untuk background
- `os_name` disimpan ke `localStorage` via `serverStore.setServerOsName()` saat pertama kali WebSocket metrics menerima data
- Distro yang didukung: Ubuntu, Debian, CachyOS, Manjaro, Arch Linux, Fedora, CentOS, RHEL, openSUSE, Alpine, Linux Mint, Kali, Raspberry Pi, NixOS, Rocky, AlmaLinux

---

## 7. Referensi API Lengkap

### Endpoint Public
| Method | Path | Deskripsi |
|--------|------|-----------|
| GET | `/api/ping` | Health check, return "pong" |
| POST | `/api/auth/login` | Login PAM, return JWT |

### Metrics & System
| Method | Path | Deskripsi |
|--------|------|-----------|
| GET WS | `/api/metrics/ws` | Live metrics tiap 3 detik |
| GET | `/api/metrics/history` | Riwayat 24 jam dari DB |
| GET | `/api/syslogs?filter=all\|auth\|kernel` | Journal logs |
| GET | `/api/process/list` | Top proses |
| POST | `/api/process/kill/{pid}` | Kill proses |
| GET | `/api/services` | List systemd services |
| POST | `/api/services/action` | Aksi service |
| GET | `/api/cron` | Read crontab root |
| POST | `/api/cron` | Update crontab root |
| POST | `/api/system/update` | git pull + rebuild |
| POST | `/api/system/reboot` | Reboot server |
| GET | `/api/system/os_updates` | Cek OS updates |
| GET WS | `/api/system/os_updates/ws` | Live upgrade stream |
| GET | `/api/logs/activity` | 200 activity log terakhir |
| GET | `/api/logs/bash_history` | Bash history root + user |

### Network & Security
| Method | Path | Deskripsi |
|--------|------|-----------|
| GET | `/api/network` | Info interface + gateway |
| GET | `/api/ports` | Listening ports |
| POST | `/api/ports/scan` | Trigger nmap scan |
| GET | `/api/ports/scan/{job_id}` | Status nmap scan |
| GET | `/api/firewall/status` | Status UFW |
| POST | `/api/firewall/toggle` | Toggle UFW |
| POST | `/api/firewall/rule` | Tambah/hapus UFW rule |
| GET | `/api/speedtest/history` | Riwayat speedtest |
| POST | `/api/speedtest/run` | Jalankan speedtest |

### Fail2Ban
| Method | Path | Deskripsi |
|--------|------|-----------|
| GET | `/api/fail2ban/status` | Status + banned IPs |
| POST | `/api/fail2ban/install` | Install fail2ban |
| POST | `/api/fail2ban/ban` | Ban IP manual |
| POST | `/api/fail2ban/unban` | Unban IP |
| GET | `/api/fail2ban/logs` | Log fail2ban |
| GET | `/api/fail2ban/config` | Baca jail.local |
| POST | `/api/fail2ban/config` | Simpan jail config |
| DELETE | `/api/fail2ban/config/{name}` | Hapus jail |
| GET | `/api/fail2ban/filters` | List filter.d |

### File Manager
| Method | Path | Deskripsi |
|--------|------|-----------|
| GET | `/api/files/config` | home_root + system_root |
| GET | `/api/files/list?path=` | List direktori |
| GET | `/api/files/search?path=&query=` | Cari file (max 500 hasil, depth 8) |
| GET | `/api/files/download?path=` | Download file |
| POST | `/api/files/upload?path=` | Upload file (multipart) |
| POST | `/api/files/fetch` | Download URL via wget |
| POST | `/api/files/action` | rename/move/copy/delete/compress/extract/chmod |
| POST | `/api/files/text` | Read/write teks |
| GET | `/api/files/info?path=` | Metadata file |
| GET | `/api/disk/info` | Block device (lsblk + df) |
| POST | `/api/disk/mount` | Mount device |
| POST | `/api/disk/umount` | Unmount device |

### Users & Groups & SSH
| Method | Path | Deskripsi |
|--------|------|-----------|
| GET | `/api/users` | List user Linux |
| POST | `/api/users` | Buat user |
| PUT | `/api/users/{u}/password` | Ganti password |
| PUT | `/api/users/{u}/groups` | Update groups |
| DELETE | `/api/users/{u}?remove_home=` | Hapus user |
| GET | `/api/users/{u}/ssh` | List SSH keys |
| POST | `/api/users/{u}/ssh` | Tambah SSH key |
| DELETE | `/api/users/{u}/ssh` | Hapus SSH key |
| GET | `/api/groups` | List grup |
| POST | `/api/groups` | Buat grup |
| DELETE | `/api/groups/{name}` | Hapus grup |

### Cloudflare Tunnel
| Method | Path | Deskripsi |
|--------|------|-----------|
| GET | `/api/cloudflare/status` | Status lengkap tunnel |
| POST | `/api/cloudflare/install` | Install cloudflared |
| POST | `/api/cloudflare/login` | Start login, capture URL |
| GET | `/api/cloudflare/login/status` | Poll cert.pem |
| POST | `/api/cloudflare/create` | Buat tunnel |
| DELETE | `/api/cloudflare/tunnel` | Hapus tunnel |
| POST | `/api/cloudflare/start` | Start service |
| POST | `/api/cloudflare/stop` | Stop service |
| POST | `/api/cloudflare/restart` | Restart service |
| GET | `/api/cloudflare/config` | Baca config.yml + CNAME status |
| POST | `/api/cloudflare/routes` | Tambah ingress route |
| DELETE | `/api/cloudflare/routes` | Hapus ingress route |
| POST | `/api/cloudflare/routes/dns` | Register DNS CNAME |
| GET | `/api/cloudflare/health` | HTTP probe semua hostname |
| GET | `/api/cloudflare/logs` | Log journalctl 100 baris |
| GET WS | `/api/cloudflare/logs/ws` | Live log stream |

### Container & Compose
| Method | Path | Deskripsi |
|--------|------|-----------|
| GET | `/api/container/runtime` | Runtime terdeteksi |
| POST | `/api/container/runtime/refresh` | Re-detect runtime |
| GET | `/api/container/list` | List container |
| POST | `/api/container/create` | Buat container |
| POST | `/api/container/{action}/{id}` | Aksi container |
| GET | `/api/container/inspect/{id}` | Inspect container |
| GET | `/api/container/logs/{id}?tail=` | Log container |
| GET | `/api/compose/projects` | List compose project |
| POST | `/api/compose/deploy` | Deploy project |
| POST | `/api/compose/{name}/stop` | Stop project |
| POST | `/api/compose/{name}/restart` | Restart project |
| POST | `/api/compose/{name}/rebuild` | Rebuild project |
| GET | `/api/compose/{name}/ps` | Status per-service |
| GET | `/api/compose/{name}/logs?service=&tail=` | Log project |
| POST | `/api/compose/{name}/scale` | Scale service |
| GET | `/api/compose/{name}/yaml` | Ambil YAML |
| PUT | `/api/compose/{name}/yaml` | Update YAML + redeploy |
| DELETE | `/api/compose/{name}` | Hapus project |

### Terminal
| Method | Path | Deskripsi |
|--------|------|-----------|
| GET WS | `/api/terminal/ws` | Interactive PTY terminal |
| POST | `/api/terminal/start` | Start shellinabox di port 4200 |

---

## 8. Dependensi Rust (Cargo.toml)

| Crate | Versi | Kegunaan |
|-------|-------|---------|
| `axum` | 0.8.9 | Web framework (HTTP + WebSocket + Multipart) |
| `tokio` | 1.53.1 | Async runtime (full features: fs, io-util, dll) |
| `sqlx` | 0.9.0 | SQLite async (features: sqlite, runtime-tokio, chrono) |
| `serde` | 1.0.229 | Serialisasi/deserialisasi JSON & YAML |
| `serde_json` | 1.0.151 | JSON parsing dan generation |
| `serde_yaml` | 0.9 | YAML parsing (untuk cloudflare config.yml) |
| `pam` | 0.8 | Autentikasi Linux PAM |
| `jsonwebtoken` | 9 | JWT sign & verify |
| `argon2` | 0.5.3 | Hashing password |
| `chrono` | 0.4.45 | Date/time (features: serde) |
| `dotenvy` | 0.15.7 | Load file `.env` |
| `sysinfo` | 0.39.6 | Info CPU, RAM, disk, proses, network |
| `portable-pty` | 0.9.0 | PTY untuk terminal WebSocket |
| `reqwest` | 0.13.4 | HTTP client async (health probe, wget via server) |
| `rustls` | 0.23 | TLS pure-Rust (features: ring, std) |
| `ring` | 0.17 | Kriptografi untuk rustls |
| `futures-util` | 0.3.34 | Utilities async/futures |
| `tokio-stream` | 0.1.19 | Stream utilities (features: sync) |
| `tokio-util` | 0.7.19 | IO adapter (features: io) — ReaderStream |
| `tower` | 0.5.3 | Middleware layer untuk Axum |
| `tower-http` | 0.7.0 | Static file serving, CORS, trace |
| `tracing` | 0.1.44 | Structured logging |
| `tracing-subscriber` | 0.3.23 | Output tracing ke stdout |
| `uuid` | 1.26.0 | Generate UUID v4 |
| `rust-embed` | 8.12.0 | Embed file statis ke binary |
| `libc` | 0.2 | Binding ke C library Linux |
| `axum-auth` | 0.8.1 | Auth helper untuk Axum |

---

## 9. Dependensi Frontend (package.json)

### Runtime Dependencies
| Package | Versi | Kegunaan |
|---------|-------|---------|
| `vue` | ^3.5.40 | Framework Vue 3 (Composition API) |
| `vue-router` | ^5.2.0 | SPA routing |
| `@vueuse/core` | ^14.4.0 | Composable utilities (useStorage, dll) |
| `tailwindcss` | ^4.3.3 | Utility-first CSS framework |
| `@tailwindcss/vite` | ^4.3.3 | Integrasi Tailwind v4 dengan Vite |
| `lucide-vue-next` | ^1.0.0 | Icon components (700+ ikon) |
| `@lucide/vue` | ^1.34.0 | Icon library alias |
| `chart.js` | ^4.5.1 | Charting library |
| `vue-chartjs` | ^5.3.4 | Wrapper Vue untuk Chart.js |
| `@xterm/xterm` | ^6.0.0 | Terminal emulator (WebSocket PTY) |
| `@xterm/addon-fit` | ^0.11.0 | Auto-resize terminal ke container |

### Dev Dependencies
| Package | Versi | Kegunaan |
|---------|-------|---------|
| `vite` | ^8.1.5 | Build tool & dev server |
| `@vitejs/plugin-vue` | ^6.0.8 | Plugin Vite untuk `.vue` SFC |
| `typescript` | ~6.0.0 | TypeScript compiler |
| `vue-tsc` | ^3.3.7 | Type checking Vue SFC |
| `npm-run-all2` | ^9.0.2 | Jalankan beberapa npm script |

**Node.js version requirement:** `^22.18.0 || >=24.12.0`

---

## 10. Build & Deployment Flow

### Backend Build
```bash
# Development
cargo run

# Production (dipanggil otomatis oleh start.sh)
cargo build --release
./target/release/infoinserver
```

### Frontend Build
```bash
cd frontend-vue
npm install

# Development (hot-reload, proxy ke :8080)
npm run dev

# Production build (output ke ../static/)
npm run build
```

### `start.sh` — Apa yang Terjadi
1. Jalankan `cargo build --release`
2. Stop instance lama (jika ada) via PID file atau `pkill`
3. Spawn binary baru sebagai daemon background
4. Tulis output ke `server.log`
5. Aplikasi startup: load `.env`, inisialisasi DB (jalankan migrations), start scheduler, bind ke `PORT`

### Deployment Database
Migrasi SQL dijalankan **otomatis** setiap startup via `src/db/mod.rs`:
```rust
let migration_sql = include_str!("migrations.sql");
sqlx::query(migration_sql).execute(&pool).await?;
```
File `migrations.sql` menggunakan `CREATE TABLE IF NOT EXISTS` sehingga aman dijalankan berulang kali.

---

## 11. Catatan Pengembangan

### Menambah Route Baru
1. Buat handler di file `src/routes/` yang relevan (atau buat file baru)
2. Tambahkan ke `src/routes/mod.rs` sebagai `pub mod nama_modul;`
3. Daftarkan route di `src/main.rs` ke blok router yang sesuai:
   - Tanpa state: ke `stateless_routes`
   - Dengan `AppState`: ke `app_routes`
   - Dengan `ContainerState`: ke `container_routes`
   - Dengan `Networks`: ke `network_routes`

### Menambah Tabel Database Baru
1. Tambahkan DDL `CREATE TABLE IF NOT EXISTS ...` ke `src/db/migrations.sql`
2. Jika menambahkan kolom ke tabel existing, tambahkan logic incremental di `src/db/mod.rs` karena SQLite tidak mendukung `ALTER TABLE IF NOT EXISTS`

### Menambah Halaman Frontend Baru
1. Buat file `frontend-vue/src/views/NamaView.vue`
2. Import dan daftarkan di `frontend-vue/src/router/index.js`
3. Tambahkan link navigasi di `frontend-vue/src/views/ServerLayout.vue`
4. Import ikon yang dibutuhkan dari `lucide-vue-next`

### Konvensi Error Handling Backend
- Handler yang return `Result<Json<T>, (StatusCode, String)>` — paling umum
- Handler yang return `Result<Json<T>, (StatusCode, Json<Value>)>` — untuk handler lama (users_mgmt, dll)
- Selalu gunakan `map_err` untuk propagate error dengan status code yang tepat
- Gunakan `StatusCode::FORBIDDEN` untuk akses ditolak (path traversal, write ke read-only)
- Gunakan `StatusCode::BAD_REQUEST` untuk input tidak valid
- Gunakan `StatusCode::INTERNAL_SERVER_ERROR` untuk kegagalan OS command
