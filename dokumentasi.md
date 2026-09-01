# Dokumentasi Internal — InfoIn Server

Dokumen ini ditujukan bagi **developer** dan **AI Agent** yang akan memelihara, membaca, mengembangkan, atau men-debug proyek **InfoIn Server**. Dokumen ini bersifat teknis, berbahasa Indonesia, dan mencakup arsitektur, struktur file, cara kerja fitur, skema database, referensi API, serta catatan keamanan.

---

## 1. Gambaran Arsitektur

InfoIn Server menggunakan arsitektur **Decoupled Backend-Frontend**:

```
┌────────────────────────────────────────────────────────────────────────┐
│  Browser (Vue 3 SPA)                                                   │
│  - Komunikasi via HTTP REST + 4 channel WebSocket                      │
│  - Token JWT multi-user disimpan di sessionStorage per-server          │
│  - State server (daftar, active user) disimpan di localStorage         │
└──────────────────────────────┬─────────────────────────────────────────┘
                               │ HTTP / WebSocket (?token=...)
                               ▼
┌────────────────────────────────────────────────────────────────────────┐
│  Backend Rust (Axum 0.8 + Tokio)                                       │
│  - REST API + WebSocket handlers (PTY, Metrics, Logs, OS Updates)      │
│  - PAM authentication + Strict Sudo/Wheel Validation                   │
│  - Baca filesystem/kernel: /proc, /sys, /dev, lsblk, find              │
│  - Eksekusi OS command via sudo -S (injeksi password aman via stdin)   │
│  - SQLite via SQLx (async) untuk history, log, dan state lokal         │
│  - Background scheduler (tokio::spawn)                                 │
└────────────────────────────────────────────────────────────────────────┘
```

### State Management Backend

Backend menggunakan 3 tipe state yang diinjeksikan ke router via `.with_state()`:

```rust
// AppState — untuk sebagian besar handler (termasuk Auth, Cloudflare, User, File)
pub struct AppState {
    sys: Arc<Mutex<System>>,       // sysinfo, untuk metrics CPU/RAM
    networks: Arc<Mutex<Networks>>,// sysinfo, untuk statistik network
    db_pool: sqlx::SqlitePool,     // koneksi database SQLite
}

// ContainerState — khusus container/compose handler
pub struct ContainerState {
    pub runtime: Arc<RwLock<Option<RuntimeInfo>>>,
}

// (Implisit) Stateless Routes — untuk endpoint login atau info basic
```

---

## 2. Struktur Direktori Lengkap

```
.
├── Cargo.toml                    # Dependensi Rust
├── .env                          # Konfigurasi environment utama
├── start.sh / stop.sh            # Script utilitas build & run backend ke background
├── data.db                       # Database SQLite (dibuat otomatis)
├── docker-compose.yml            # Deployment frontend via Nginx/Podman
│
├── src/
│   ├── main.rs                   # Entry point: init state, CORS, route merge grouping
│   │
│   ├── auth/
│   │   ├── jwt.rs                # create_token(), verify_token(), struct Claims
│   │   ├── jwt_middleware.rs     # jwt_auth_middleware + whitelist websocket/auth
│   │   └── middleware.rs         # Middleware helper tambahan
│   │
│   ├── background/
│   │   └── scheduler.rs          # Rekam metrics tiap 5 menit, alert anomali
│   │
│   ├── db/
│   │   ├── mod.rs                # init_db(): pool config, eksekusi migrations.sql
│   │   └── migrations.sql        # DDL untuk 5 tabel (termasuk cloudflare_cname_status)
│   │
│   ├── services/                 # Layer logika bisnis & OS command wrapper
│   │   ├── file_manager.rs       # resolve_path_safe(), check_write_permission()
│   │   ├── network_info.rs       # Parse rx/tx + baca `ip route` untuk gateway
│   │   ├── port_scanner.rs       # Parse `ss -tulnp` (extract PID, process, local/public scope)
│   │   ├── proc_reader.rs        # Baca /etc/os-release, uptime, /proc/stat
│   │   ├── speedtest_cli.rs      # Jalankan speedtest-cli via spawn_blocking + FIFO DB
│   │   ├── compose_manager.rs    # Docker/Podman Compose management
│   │   └── ... (nmap_scanner, container_runtime, process_info)
│   │
│   └── routes/                   # Layer HTTP handlers (26 modul)
│       ├── auth.rs               # Login PAM, tolak root, wajib grup sudo/wheel
│       ├── cloudflare.rs         # Status, install, create tunnel, ws logs
│       ├── cloudflare_api.rs     # Config YAML (serde_yaml), DNS CNAME DB, health probe HTTP
│       ├── disk.rs               # Parse `lsblk`, `df`, endpoint mount/umount USB
│       ├── fail2ban.rs           # Status jails, manual ban/unban, config editor, baca log
│       ├── files.rs              # File browse, download, upload, action, text read/write
│       ├── firewall.rs           # Status UFW, toggle, add/delete rules
│       ├── logs.rs               # Activity log dari DB, baca .bash_history
│       ├── network.rs            # Info interface + gateway
│       ├── ports.rs              # Listening ports, trigger nmap scan
│       ├── process_mgmt.rs       # List proses, sudo_exec() helper stdin injection
│       ├── speedtest.rs          # Ambil 5 riwayat terakhir, run test
│       ├── syslogs.rs            # journalctl viewer
│       ├── system_mgmt.rs        # Self-update dashboard, reboot
│       ├── system_updates.rs     # Cek apt/pacman updates, WebSocket live upgrade
│       ├── terminal_ws.rs        # WebSocket PTY terminal interaktif
│       ├── users_mgmt.rs         # CRUD User/Group, chpasswd, manage SSH keys
│       └── ... (cron, services, container, compose)
│
└── frontend-vue/
    └── src/
        ├── composables/
        │   └── useApi.js         # Wrapper fetch: inject JWT, auto-logout jika 401 atau sudo fail
        ├── stores/
        │   ├── serverStore.js    # State multi-user session per server, OS name cache
        │   └── toastStore.js     # Toast notification & confirmation modal
        ├── utils/
        │   └── distro.js         # Helper: mapping OS name ke logo SVG SimpleIcons CDN
        └── views/
            ├── HomeView.vue          # Daftar server dengan ikon distro
            ├── ServerLayout.vue      # Header dengan User Switcher Dropdown, Navigasi Tab, Ping Indicator
            ├── DashboardView.vue     # Live metrics, chart historis, CPU/RAM progress
            ├── ContainerView.vue     # Manajemen Docker/Podman (Containers, Compose, Deploy)
            ├── FilesView.vue         # File Explorer 2.0 (Dual pane, USB sidebar, Read-only badges)
            ├── UsersView.vue         # Manajemen Linux Users, OS Groups, dan SSH Keys (Modals)
            ├── ServicesView.vue      # Systemd Services & Top Processes
            ├── PortsView.vue         # Network Interfaces, Firewall, Fail2Ban Command Center, Scanner
            ├── CloudflareView.vue    # Cloudflare Setup Wizard & 3-Tab Command Center (Routes, Health, Logs)
            ├── SyslogsView.vue       # 3-Tab viewer: System Journal, Dashboard Activity, Bash History
            ├── UpdatesView.vue       # UI OS Upgrade dengan WebSocket terminal
            └── SettingsView.vue      # Pengaturan koneksi server
```

---

## 3. Konfigurasi Environment (`.env`)

| Variabel | Nilai Default | Keterangan |
|----------|--------------|------------|
| `PORT` | `8080` | Port HTTP backend |
| `FILE_ROOT` | `$HOME` | Secara otomatis di-expand ke home direktori user yang sedang aktif pada session dashboard. Jangan gunakan absolute path kecuali ingin mengunci semua user ke folder yang sama. |
| `DB_PATH` | `sqlite:./data.db` | Path file SQLite |
| `JWT_SECRET` | *(fallback)* | Secret untuk sign/verify JWT. **WAJIB diisi** di production. |

---

## 4. Skema Database (SQLite)

Tabel dibuat otomatis via `src/db/migrations.sql`.

1. **`system_metrics_history`**: CPU, RAM, Disk, Net_RX/TX direkam tiap 5 menit oleh scheduler.
2. **`speedtest_history`**: History uji kecepatan. Menggunakan logika FIFO (menghapus data lama otomatis, hanya menyimpan 5 data terbaru).
3. **`port_scan_jobs`**: Job queue nmap scan async.
4. **`activity_log`**: Audit trail. Fungsi `log_activity(pool, level, action, detail)` dipanggil setiap kali terjadi perubahan sistem (User add, FW toggle, Tunnel delete, dll). Ditampilkan di Tab Dashboard Activity.
5. **`cloudflare_cname_status`**: Mencatat CNAME DNS yang didaftarkan manual. Digunakan agar status di UI langsung 'Active' (Hijau) tanpa harus menunggu propagasi query DNS yang sering *pending*.

---

## 5. Konsep Keamanan & Prinsip Kerja

### 5.1 Multi-User Dashboard Sessions
- Dashboard tidak memiliki database pengguna internal. Semua login diverifikasi langsung ke OS via **PAM**.
- **Validasi Ketat**: Backend menolak login jika username adalah `root`. Backend mengeksekusi `groups <username>` untuk memastikan pengguna berada di grup `sudo` atau `wheel`.
- **JWT Storage**: Frontend `serverStore.js` mampu menyimpan banyak token untuk satu server (e.g. user `infratek` dan `webmaster`). Pengguna bisa beralih (switch) user via dropdown di header tanpa perlu memasukkan password berulang kali.
- **Auto-Logout**: Jika `sudo_exec()` mereturn error `"sudo: authentication failed"`, `useApi.js` di frontend akan menangkapnya, menghapus token user tersebut, men-switch ke user lain (jika ada), atau menendang ke layar login.

### 5.2 Zona Izin File Explorer (File Explorer 2.0)
- Navigasi filesystem terbuka dari root `/` (*read-only*).
- Operasi modifikasi (*Write, Upload, Delete, Move, Chmod*) wajib divalidasi oleh `check_write_permission()`.
- Izin Write hanya diberikan jika target berada di dalam `FILE_ROOT` (yang telah di-expand ke `/home/<username>`) **ATAU** berada di dalam *removable mount points* (`/media/` atau `/mnt/` yang terdeteksi via `/proc/mounts`).
- Mode *List, Grid (Image Thumbnails),* dan *Compact* didukung penuh.

### 5.3 Sudo Execution & Blocking Threads
- Fungsi `sudo_exec()` membungkus `Command::new("sudo")` dan menginjeksikan password dari JWT secara aman via saluran *stdin* (piped).
- Karena eksekusi OS (*apt, pacman, cloudflared, speedtest*) memblokir thread, operasi ini wajib dibungkus di dalam `tokio::task::spawn_blocking(...)`.
- Kegagalan membungkus perintah yang memakan waktu lama (seperti speedtest 30 detik) ke dalam `spawn_blocking` akan menyebabkan seluruh *async executor runtime Tokio* hang (termasuk Ping dan WebSocket).

### 5.4 Live Logs & Websocket Terminal
Ada empat (4) endpoint WebSocket yang mendesain PTY atau mem-pipe stdout/stderr:
1. `/api/terminal/ws` — Native Bash PTY
2. `/api/metrics/ws` — Streaming loop sysinfo tiap 3 detik
3. `/api/cloudflare/logs/ws` — Pipe `journalctl -u cloudflared -f`
4. `/api/system/os_updates/ws` — Pipe `apt-get upgrade` (stdout & stderr merged via `tokio::select!`)

*WebSocket membutuhkan `?token=...` query param karena browser API tidak mendukung custom header pada handshake.*

### 5.5 Fail2Ban & UFW (Security)
- **UFW**: Backend melakukan parsing teks statis dari output `ufw status`. Aksi (allow/deny) diinjeksi via CLI.
- **Fail2Ban**:
  - Konfigurasi `jail.local` di-parse secara manual menggunakan INI-style reader sederhana di Rust.
  - Endpoint `register` ban/unban memanipulasi memory daemon menggunakan `fail2ban-client`.
  - UI Frontend memiliki katalog *Quick Add* untuk setup service populer (Nginx, Postfix, dll) dengan parameter default.

---

## 6. Referensi Endpoint API

**(Seluruh endpoint kecuali `/api/ping` dan `/api/auth/*` membutuhkan JWT Bearer Token)**

| Modul | Endpoint | Method | Keterangan |
|-------|----------|--------|------------|
| **Auth** | `/api/auth/login` | POST | PAM auth, return JWT |
| **System** | `/api/metrics/ws` | GET | WS Live metrics |
| | `/api/metrics/history` | GET | DB metrics (24h) |
| | `/api/system/update` | POST | Self-update (git pull) |
| | `/api/system/reboot` | POST | OS reboot |
| | `/api/system/os_updates` | GET | APT/Pacman list upgradable |
| | `/api/system/os_updates/ws`| GET | WS Live OS upgrade |
| **Network** | `/api/ping` | GET | (Public) Fast latency check |
| | `/api/network` | GET | Interfaces, IPs, Gateway, Rx/Tx |
| | `/api/ports` | GET | Listening ports + scope (public/local) |
| | `/api/speedtest/*` | GET/POST | History (limit 5) / Run test |
| **Security** | `/api/firewall/*` | GET/POST | UFW status, toggle, rule |
| | `/api/fail2ban/status` | GET | Active jails + Banned IPs |
| | `/api/fail2ban/ban` | POST | Manual ban IP |
| | `/api/fail2ban/unban` | POST | Manual unban IP |
| | `/api/fail2ban/config` | GET/POST | Editor `jail.local` |
| | `/api/fail2ban/config/{name}`| DELETE | Hapus blok jail dari file |
| | `/api/fail2ban/logs` | GET | Tail `/var/log/fail2ban.log` |
| **Cloudflare**| `/api/cloudflare/status` | GET | Status, service_uptime_secs |
| | `/api/cloudflare/create` | POST | Create tunnel |
| | `/api/cloudflare/routes` | POST/DEL | Add/Remove ingress rules |
| | `/api/cloudflare/routes/dns` | POST | Register CNAME via CLI, save to DB |
| | `/api/cloudflare/health` | GET | E2E HTTP probe (502, 1033, NXDOMAIN) |
| | `/api/cloudflare/logs/ws` | GET | WS Live `journalctl -f` |
| **Files** | `/api/files/config` | GET | Return expanded home_root user aktif |
| | `/api/files/search` | GET | Recursive `find` maxdepth 8 |
| | `/api/files/action` | POST | rename, move, copy, compress, chmod |
| | `/api/files/info` | GET | Owner, modified_at, size, sym-perms |
| **Disk** | `/api/disk/info` | GET | Parse `lsblk -J` + `df` |
| | `/api/disk/mount` | POST | Mount removable to `/media/` |
| **Users** | `/api/users` | GET/POST | List, Create user (`useradd`) |
| | `/api/users/{u}/password` | PUT | `chpasswd` (piped sh -c) |
| | `/api/users/{u}/groups` | PUT | `usermod -G` |
| | `/api/users/{u}/ssh` | GET/POST | Manage `authorized_keys` |
| | `/api/groups` | GET/POST | List, Create, Delete Linux groups |
| **Logs** | `/api/syslogs` | GET | `journalctl` (all/auth/kernel) |
| | `/api/logs/activity` | GET | DB Activity log (Audit trail) |
| | `/api/logs/bash_history` | GET | Baca `.bash_history` user aktif & root |

---
*Dokumentasi ini otomatis merefleksikan status codebase per pembaruan terakhir. Pastikan untuk memperbarui dokumen ini jika ada penambahan rute atau modifikasi tabel DB baru.*