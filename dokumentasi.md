# Dokumentasi Internal — InfoIn Server

Dokumen ini ditujukan bagi **developer** dan **AI Agent** yang akan memelihara, membaca, mengembangkan, atau men-debug proyek **InfoIn Server**. Dokumen ini bersifat teknis, berbahasa Indonesia, dan mencakup arsitektur, struktur file, cara kerja fitur, skema database, referensi API, serta catatan keamanan.

---

## 1. Gambaran Arsitektur

InfoIn Server menggunakan arsitektur **3-layer**: Browser → Bun Server → Rust Backend → Linux Kernel.

```
┌────────────────────────────────────────────────────────────────────────┐
│  Browser (Vue 3 SPA)                                                   │
│  - Komunikasi via HTTP REST + WebSocket                                │
│  - GitHub session token disimpan di localStorage                       │
│  - Linux PAM token disimpan di sessionStorage per-server              │
│  - Konfigurasi server (list, labels) di-sync ke Bun SQLite            │
└──────────────────────────────┬─────────────────────────────────────────┘
                               │ HTTP / WebSocket
                               ▼
┌────────────────────────────────────────────────────────────────────────┐
│  Bun + Hono Server (port 3000)   [frontend-vue/server.ts]             │
│  - GitHub OAuth handler (secret tidak pernah expose ke browser)       │
│  - RBAC layer: role management (admin/master/slave/custom)            │
│  - Frontend config sync global via bun:sqlite (frontend.db)           │
│  - HTTP proxy → Rust backend                                           │
│  - WebSocket proxy bidirectional → Rust (native Bun.serve websocket)  │
│  - Serve Vue static files (production) / proxy Vite dev (dev mode)    │
└──────────────────────────────┬─────────────────────────────────────────┘
                               │ HTTP / WebSocket
                               ▼
┌────────────────────────────────────────────────────────────────────────┐
│  Backend Rust (Axum 0.8 + Tokio)  [src/]                             │
│  - REST API + WebSocket handlers (PTY, Metrics, Logs, OS Updates)     │
│  - PAM authentication + Strict Sudo/Wheel Validation                  │
│  - Baca filesystem/kernel: /proc, /sys, /dev, lsblk, find             │
│  - Eksekusi OS command via sudo -S (injeksi password aman via stdin)  │
│  - SQLite via SQLx (async) untuk history, log, dan state lokal        │
│  - Background scheduler (tokio::spawn)                                 │
└────────────────────────────────────────────────────────────────────────┘
```

### 1.1 State Management Backend (Rust)

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

### 1.2 Bun Server — Middle Layer

Bun server (`frontend-vue/server.ts`) berjalan sebagai proses terpisah dengan dua tanggung jawab utama:

**GitHub OAuth** — Semua flow OAuth dihandle di Bun:
- `GET /api/auth/github` → redirect ke GitHub
- `GET /api/auth/github/callback` → tukar code → access token → profil → sign JWT session token → redirect ke `/auth/callback`
- `GET /api/auth/github/users` → return presence list dari in-memory store

**RBAC Layer** — Role-based access control dihandle Bun (3 built-in role + custom role):
- `GET /api/roles/me` → return role aktif, `canWrite`, `restrictedLabels`, `allowedLabels`
- `GET /api/roles/users` → list semua GitHub users + role mereka (admin only)
- `PUT /api/roles/:username/role` → assign role ke user (admin only)
- `GET /api/roles/groups` → baca allowed roles per label group (admin only)
- `PUT /api/roles/groups/:labelId` → set roles yang boleh akses suatu label group (admin only)
- `GET /api/roles/settings` → baca default role + default groups (admin only)
- `PUT /api/roles/settings` → update default role/group settings (admin only)
- `GET /api/roles/custom` → list custom roles (admin only)
- `POST /api/roles/custom` → buat custom role (admin only)
- `PUT /api/roles/custom/:name` → edit custom role (admin only)
- `DELETE /api/roles/custom/:name` → hapus custom role (admin only)

**WebSocket Proxy** — Menggunakan `Bun.serve()` native (bukan Hono) dengan `websocket` handler:
- Browser upgrade request ke `/api/*/ws` di-detect dari header `Upgrade: websocket`
- Bun membuka koneksi WebSocket baru ke Rust backend
- Pipe dua arah: browser ↔ Bun ↔ Rust
- Rust tidak tahu bahwa client sebenarnya adalah browser — ia melihat Bun sebagai client WS biasa

```ts
// Deteksi + upgrade
if (url.pathname.startsWith('/api/') && req.headers.get('upgrade') === 'websocket') {
  server.upgrade(req, { data: { backendUrl, backendWs: null } })
}

// Forward frames
websocket: {
  open(ws)    { /* buka WS ke Rust */ },
  message(ws, msg) { ws.data.backendWs?.send(msg) },
  close(ws)   { ws.data.backendWs?.close() },
}
```

**Frontend Config Sync** — Menggunakan `bun:sqlite` (built-in, zero dependency):
- `GET /api/frontend/config` → query `frontend.db` — **semua user membaca config milik `DEFAULT_ADMIN`**, bukan per-user
- `PUT /api/frontend/config` → upsert konfigurasi global — **hanya role `admin` atau `master`** yang diizinkan; `slave` mendapat 403
- Diverifikasi dengan GitHub session JWT (`GITHUB_SESSION_SECRET`, algoritma HS256)
- Memungkinkan server list, labels, dan urutan card tersinkron secara global untuk semua user yang punya akses

---

## 2. Struktur Direktori Lengkap

```
.
├── Cargo.toml                    # Dependensi Rust
├── .env                          # Konfigurasi Rust backend
├── .env.example                  # Template .env Rust
├── start.sh                      # Script start: --backend | --frontend | --dev
├── stop.sh                       # Script stop: --backend | --frontend
├── server.log                    # Log Rust backend (dibuat oleh start.sh)
├── frontend.log                  # Log Bun server (dibuat oleh start.sh)
├── vite.log                      # Log Vite dev server (dibuat oleh start.sh --dev)
├── data.db                       # Database SQLite Rust (dibuat otomatis)
├── docker-compose.yml            # Deployment Bun server via Podman/Docker
│
├── src/                          # Source Rust backend
│   ├── main.rs                   # Entry point: init state, CORS, route merge grouping
│   │
│   ├── auth/
│   │   ├── jwt.rs                # create_token(), verify_token(), struct Claims
│   │   ├── jwt_middleware.rs     # jwt_auth_middleware + whitelist websocket/auth
│   │   └── mod.rs                # Re-export auth module
│   │
│   ├── background/
│   │   └── scheduler.rs          # Rekam metrics tiap 5 menit, alert anomali
│   │
│   ├── db/
│   │   ├── mod.rs                # init_db(): pool config, eksekusi migrations.sql
│   │   └── migrations.sql        # DDL untuk 7 tabel (termasuk github_users, vm_instances)
│   │
│   ├── services/                 # Layer logika bisnis & OS command wrapper
│   │   ├── file_manager.rs       # resolve_path_safe(), check_write_permission()
│   │   ├── network_info.rs       # Parse rx/tx + baca `ip route` untuk gateway
│   │   ├── port_scanner.rs       # Parse `ss -tulnp` (extract PID, process, local/public scope)
│   │   ├── proc_reader.rs        # Baca /etc/os-release, uptime, /proc/stat
│   │   ├── speedtest_cli.rs      # Jalankan speedtest-cli via spawn_blocking + FIFO DB
│   │   ├── compose_manager.rs    # Docker/Podman Compose management
│   │   ├── container_runtime.rs  # Deteksi Docker atau Podman, versi runtime
│   │   ├── nmap_scanner.rs       # Async nmap port scanner
│   │   ├── process_info.rs       # Baca proses aktif (CPU/RAM sort)
│   │   └── system_info.rs        # Informasi sistem (OS, hostname, kernel, uptime)
│   │
│   └── routes/                   # Layer HTTP handlers
│       ├── auth.rs               # Login PAM, tolak root, wajib grup sudo/wheel
│       ├── cloudflare.rs         # Status, install, create/delete tunnel, ws logs
│       ├── cloudflare_api.rs     # Config YAML, DNS CNAME DB, health probe HTTP
│       ├── cron_mgmt.rs          # Baca/tulis /etc/crontab
│       ├── disk.rs               # Parse lsblk, df, endpoint mount/umount USB
│       ├── fail2ban.rs           # Status jails, manual ban/unban, config editor, filters
│       ├── files.rs              # File browse, info, download, upload, fetch URL, action, text read/write
│       ├── firewall.rs           # Status UFW, toggle, add/delete rules
│       ├── logs.rs               # Activity log dari DB, baca .bash_history
│       ├── network.rs            # Info interface + gateway
│       ├── ports.rs              # Listening ports, trigger nmap scan
│       ├── process_mgmt.rs       # List proses, kill PID, sudo_exec() helper stdin injection
│       ├── services_mgmt.rs      # Systemd service management
│       ├── speedtest.rs          # Ambil 5 riwayat terakhir, run test
│       ├── syslogs.rs            # journalctl viewer
│       ├── system_mgmt.rs        # Self-update dashboard, reboot
│       ├── system_updates.rs     # Cek apt/pacman updates, WebSocket live upgrade
│       ├── terminal_ws.rs        # WebSocket PTY terminal interaktif + shellinabox fallback
│       ├── users_mgmt.rs         # CRUD User/Group, chpasswd, manage SSH keys
│       ├── vm.rs                 # VM management via Podman (list, create WS, start, stop, status, delete)
│       └── container.rs          # Docker/Podman container lifecycle + runtime detection
│
└── frontend-vue/
    ├── server.ts                 # Bun + Hono server (GitHub OAuth, RBAC, WS proxy, config sync)
    ├── frontend.db               # SQLite Bun untuk frontend config + RBAC (dibuat otomatis)
    ├── .env                      # Konfigurasi Bun server (GitHub OAuth, JWT, dll)
    ├── .env.example              # Template .env Bun server
    ├── package.json              # Dependencies: hono, sortablejs, vue, vite, dll
    └── src/
        ├── composables/
        │   ├── useApi.js         # Wrapper fetch: inject JWT, auto-logout jika 401
        │   └── useHeartbeat.js   # Heartbeat 30 detik untuk update last_seen presence
        ├── stores/
        │   ├── serverStore.js    # Server list, labels, config sync ke Bun SQLite
        │   ├── toastStore.js     # Toast notification & confirmation modal
        │   ├── themeStore.js     # Dark mode toggle, isDark ref, localStorage
        │   └── authStore.js      # GitHub session, role, canWrite, restrictedLabels
        ├── utils/
        │   └── distro.js         # Mapping OS name ke logo SVG SimpleIcons CDN
        └── views/
            ├── LoginView.vue         # Halaman login (layered bg, LoginPanel, FeatureHighlight)
            ├── AuthCallbackView.vue  # Handle redirect OAuth callback, simpan session
            ├── HomeView.vue          # Server cards dengan ping, edit mode, labels, drag & drop
            ├── ServerLayout.vue      # Sidebar: distro icon, user switcher, nav, ping indicator
            ├── DashboardView.vue     # Live metrics, chart historis, CPU/RAM progress
            ├── ContainerView.vue     # Manajemen Docker/Podman (Containers, Compose, Deploy, VMs)
            ├── CronView.vue          # Cron job manager (/etc/crontab) dengan form terstruktur
            ├── FilesView.vue         # File Explorer (USB sidebar, Read-only badges)
            ├── UsersView.vue         # Manajemen Linux Users, OS Groups, SSH Keys
            ├── ServicesView.vue      # Systemd Services & Top Processes
            ├── PortsView.vue         # Network Interfaces, Firewall, Fail2Ban, Scanner
            ├── CloudflareView.vue    # Cloudflare Setup Wizard & Command Center
            ├── SyslogsView.vue       # Journal, Dashboard Activity, Bash History
            ├── UpdatesView.vue       # UI OS Upgrade dengan WebSocket terminal
            └── SettingsView.vue      # Koneksi server + System Reset (juga tersedia sebagai /settings global)
```

---

## 3. Konfigurasi Environment

### 3.1 Rust Backend (`.env` di root project)

| Variabel | Nilai Default | Keterangan |
|----------|--------------|------------|
| `PORT` | `8080` | Port HTTP backend |
| `FILE_ROOT` | `$HOME` | Root direktori untuk operasi tulis file. Di-expand ke home user aktif. |
| `DB_PATH` | `sqlite:./data.db` | Path file SQLite Rust |
| `JWT_SECRET` | *(fallback tidak aman)* | Secret sign/verify Linux PAM JWT. **WAJIB diisi**. Harus sama dengan `JWT_SECRET` di `frontend-vue/.env`. |
| `CORS_ORIGIN` | `http://localhost:3000` | Origin yang diizinkan. Harus sama dengan URL Bun server. |

### 3.2 Bun Server (`frontend-vue/.env`)

| Variabel | Nilai Default | Keterangan |
|----------|--------------|------------|
| `FRONTEND_PORT` | `3000` | Port Bun server |
| `RUST_BACKEND_URL` | `http://localhost:8080` | URL Rust backend (internal, tidak diakses browser langsung) |
| `FRONTEND_URL` | `http://localhost:3000` | URL publik Bun server (untuk OAuth redirect) |
| `JWT_SECRET` | *(fallback)* | **Harus identik** dengan `JWT_SECRET` di `.env` Rust — digunakan Bun untuk memverifikasi token PAM yang dikirim browser |
| `GITHUB_CLIENT_ID` | — | GitHub OAuth App client ID |
| `GITHUB_CLIENT_SECRET` | — | GitHub OAuth App client secret (disimpan di server, tidak pernah ke browser) |
| `GITHUB_REDIRECT_URI` | — | Harus cocok dengan "Authorization callback URL" di GitHub OAuth App. Format: `{FRONTEND_URL}/api/auth/github/callback` |
| `GITHUB_SESSION_SECRET` | *(fallback)* | Secret untuk sign GitHub session JWT (HS256). Berbeda dengan `JWT_SECRET`. |
| `FRONTEND_DB_PATH` | `./frontend.db` | Path SQLite untuk frontend config sync |
| `NODE_ENV` | — | Set `production` untuk serve static files. Tanpa ini → dev mode (proxy ke Vite) |
| `VITE_DEV_URL` | `http://localhost:5173` | URL Vite dev server, hanya dipakai saat dev mode (dikomentari di `.env.example` by default) |
| `DEFAULT_ADMIN` | `widy4aa` | GitHub username yang menjadi owner config global. Semua user membaca config dari akun ini. Ganti dengan username admin kamu. |

---

## 4. Skema Database (SQLite)

### 4.1 Rust Database (`data.db`)

Tabel dibuat otomatis via `src/db/migrations.sql`:

1. **`system_metrics_history`** — CPU, RAM, Disk, Net_RX/TX direkam tiap 5 menit oleh scheduler.
2. **`speedtest_history`** — History uji kecepatan. FIFO: hanya simpan 5 data terbaru.
3. **`port_scan_jobs`** — Job queue nmap scan async.
4. **`activity_log`** — Audit trail. `log_activity(pool, level, action, detail)` dipanggil setiap kali ada perubahan sistem. Ditampilkan di Tab Dashboard Activity.
5. **`cloudflare_cname_status`** — CNAME DNS yang didaftarkan manual. Agar status UI langsung 'Active' tanpa menunggu propagasi DNS.
6. **`github_users`** — Menyimpan data GitHub user yang pernah login beserta `last_seen` timestamp untuk fitur presence/heartbeat.
7. **`vm_instances`** — Menyimpan state VM yang di-deploy via Podman (nama, image, container_id, image_tag). Schema memiliki incremental migration di `mod.rs`.

### 4.2 Bun Database (`frontend.db`)

Tabel dibuat otomatis saat Bun server pertama kali dijalankan:

```sql
CREATE TABLE IF NOT EXISTS frontend_config (
  github_username  TEXT PRIMARY KEY,   -- GitHub username sebagai key
  servers          TEXT NOT NULL DEFAULT '[]',  -- JSON array server list
  labels           TEXT NOT NULL DEFAULT '[]',  -- JSON array label groups
  active_server_id TEXT NOT NULL DEFAULT '',    -- ID server yang aktif
  updated_at       INTEGER NOT NULL DEFAULT 0   -- Unix timestamp update terakhir
);

-- RBAC: mapping GitHub username ke role
CREATE TABLE IF NOT EXISTS user_roles (
  github_username TEXT PRIMARY KEY,
  role            TEXT NOT NULL DEFAULT 'slave'  -- 'admin' | 'master' | 'slave' | 'custom'
);

-- RBAC: role mana yang boleh akses label group tertentu
CREATE TABLE IF NOT EXISTS group_allowed_roles (
  label_id  TEXT NOT NULL,
  role_name TEXT NOT NULL,
  PRIMARY KEY (label_id, role_name)
);

-- RBAC: konfigurasi global default role dan default groups
CREATE TABLE IF NOT EXISTS group_settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

-- RBAC: definisi custom role yang dibuat admin
CREATE TABLE IF NOT EXISTS custom_roles (
  name      TEXT PRIMARY KEY,
  can_write INTEGER NOT NULL DEFAULT 0
);

-- RBAC: whitelist group yang boleh diakses custom role
CREATE TABLE IF NOT EXISTS custom_role_groups (
  role_name TEXT NOT NULL,
  label_id  TEXT NOT NULL,
  PRIMARY KEY (role_name, label_id)
);
```

- **`frontend_config`** — Config global (bukan per-user). Semua user membaca baris milik `DEFAULT_ADMIN`. Hanya admin/master yang bisa write.
- **`user_roles`** — Tiga built-in role: `admin` (akses penuh), `master` (bisa write config), `slave` (read-only, dibatasi group).
- **`group_allowed_roles`** — Kontrol akses group: role mana saja yang bisa melihat server dalam suatu label.
- **`group_settings`** — Konfigurasi global: `default_role` untuk user baru, `default_groups` yang aktif.
- **`custom_roles`** — Role yang dibuat admin dengan flag `can_write` dan daftar group yang diizinkan.
- **`custom_role_groups`** — Whitelist group per custom role.

---

## 5. Konsep Keamanan & Prinsip Kerja

### 5.1 Dua Layer Autentikasi

**Layer 1 — GitHub OAuth (identitas dashboard):**
- Dihandle sepenuhnya oleh **Bun server** — Rust tidak tahu tentang GitHub OAuth.
- Token di-sign oleh Bun dengan `GITHUB_SESSION_SECRET` (HS256, exp 7 hari).
- Disimpan di `localStorage['github-session']` di browser.
- Digunakan untuk: akses `/api/frontend/config`, identifikasi user di presence store.

**Layer 2 — Linux PAM (autentikasi per-server):**
- Dihandle oleh **Rust backend** via `/api/auth/login`.
- Token di-sign Rust dengan `JWT_SECRET` (HS256, exp 24 jam).
- Disimpan di `sessionStorage['server-tokens']` — **sengaja tidak di-sync** karena menyimpan password Linux.
- Digunakan untuk semua operasi OS: metrics, terminal, file, services, dll.
- Root login diblokir. Wajib anggota grup `sudo` atau `wheel`.

### 5.2 WebSocket Proxy Flow

Browser tidak connect langsung ke Rust untuk WebSocket. Semua WS melalui Bun:

```
Browser → ws://bun:3000/api/metrics/ws?token=<pam_jwt>
  ↓ Bun deteksi Upgrade header, server.upgrade()
  ↓ Bun buka ws://rust:8080/api/metrics/ws?token=<pam_jwt>
  ↓ Pipe bidirectional: browser.send ↔ rust.send
```

Rust mem-validasi token PAM dari `?token=` query param seperti biasa — ia tidak tahu bahwa ada Bun di tengah.

**Catatan penting:** `/api/metrics/ws` dan `/api/cloudflare/logs/ws` masuk **whitelist** JWT middleware Rust (skip header check, validasi hanya dari `?token=`). Endpoint `/api/system/os_updates/ws` **tidak** di-whitelist — ia butuh `AuthUser` extension untuk mendapatkan password dari JWT.

### 5.3 Multi-User Dashboard Sessions

- `serverStore.js` menyimpan banyak token untuk satu server: `{ activeUser, users: { username: token } }`.
- Switch user via dropdown di sidebar tanpa re-login.
- Auto-logout: jika `sudo_exec()` return `"sudo: authentication failed"`, `useApi.js` hapus token user tersebut dan switch ke user lain atau redirect ke login.

### 5.4 Frontend Config Sync

- `App.vue` memanggil `loadConfigFromServer(githubToken)` saat `isLoggedIn = true`.
- `serverStore.js` fetch `GET /api/frontend/config` dengan GitHub session token.
- **Semua user membaca config yang sama** — config global milik `DEFAULT_ADMIN`, bukan config per-user.
- Hanya user dengan role `admin` atau `master` yang dapat mengubah config via `PUT /api/frontend/config`. User `slave` mendapat 403.
- Setiap perubahan (tambah server, edit label, drag card) oleh admin/master → `watch` trigger → `saveConfigToServer()` debounce 1.5 detik → `PUT /api/frontend/config`.
- **`server-tokens`** (password Linux) **tidak** di-sync — tetap di `sessionStorage` saja.

### 5.5 Zona Izin File Explorer

- Navigasi filesystem terbuka dari root `/` (read-only).
- Operasi modifikasi (Write, Upload, Delete, Move, Chmod) divalidasi oleh `check_write_permission()`.
- Izin Write hanya jika target ada di `FILE_ROOT` (`/home/<username>`) atau removable mount (`/media/`, `/mnt/`).

### 5.6 Sudo Execution & Blocking Threads

- `sudo_exec()` inject password dari JWT via `stdin` (piped) — tidak lewat argument.
- Perintah OS yang lama (apt, pacman, speedtest, cloudflared) **wajib** dibungkus `tokio::task::spawn_blocking(...)` agar tidak memblokir async executor Tokio.

### 5.7 Dark Mode

- Class-based Tailwind: `.dark` ditambah/hapus dari `<html>`.
- `themeStore.js` — `isDark` ref + `toggleDark()` + persist ke `localStorage['infoin-theme']`.

### 5.8 System Reset (Settings Page)

| Aksi | Endpoint | Yang Dihapus |
|---|---|---|
| **Reset Cloudflare** | `POST /api/cloudflare/reset` | Stop service → uninstall unit → `rm -rf /etc/cloudflared` → hapus cert + config → hapus CNAME dari DB |
| **Reset UFW** | `POST /api/firewall/reset` | `ufw --force reset` → semua rules dihapus |
| **Reset Fail2Ban** | `POST /api/fail2ban/reset` | `rm -f /etc/fail2ban/jail.local` → `systemctl restart fail2ban` |

Setiap aksi dilindungi confirm dialog di frontend.

---

## 6. Referensi Endpoint API

### 6.1 Bun Server Endpoints (port 3000)

*Tidak membutuhkan Linux PAM token — menggunakan GitHub session token atau publik.*

| Endpoint | Method | Auth | Keterangan |
|---|---|---|---|
| `/api/auth/github` | GET | Public | Redirect ke GitHub OAuth |
| `/api/auth/github/callback` | GET | Public | Tukar code → session token → redirect frontend |
| `/api/auth/github/users` | GET | Public | Presence list (in-memory) |
| `/api/frontend/config` | GET | GitHub JWT | Ambil config global dari SQLite |
| `/api/frontend/config` | PUT | GitHub JWT (admin/master) | Simpan config global ke SQLite |
| `/api/roles/me` | GET | GitHub JWT | Role aktif, canWrite, restrictedLabels, allowedLabels |
| `/api/roles/users` | GET | GitHub JWT (admin) | List semua users + role |
| `/api/roles/:username/role` | PUT | GitHub JWT (admin) | Assign role ke user |
| `/api/roles/groups` | GET | GitHub JWT (admin) | Baca allowed roles per label group |
| `/api/roles/groups/:labelId` | PUT | GitHub JWT (admin) | Set roles untuk suatu label group |
| `/api/roles/settings` | GET | GitHub JWT (admin) | Baca default role + default groups |
| `/api/roles/settings` | PUT | GitHub JWT (admin) | Update default settings |
| `/api/roles/custom` | GET | GitHub JWT (admin) | List custom roles |
| `/api/roles/custom` | POST | GitHub JWT (admin) | Buat custom role |
| `/api/roles/custom/:name` | PUT | GitHub JWT (admin) | Edit custom role |
| `/api/roles/custom/:name` | DELETE | GitHub JWT (admin) | Hapus custom role |
| `/api/*` (lainnya) | * | — | Proxy ke Rust backend |

### 6.2 Rust Backend Endpoints (port 8080, via Bun proxy)

*Seluruh endpoint kecuali `/api/ping` dan `/api/auth/*` membutuhkan Linux PAM JWT.*

| Modul | Endpoint | Method | Keterangan |
|-------|----------|--------|------------|
| **Auth** | `/api/auth/login` | POST | PAM auth, return JWT |
| | `/api/auth/refresh` | POST | Refresh JWT yang masih valid |
| | `/api/ping` | GET | (Public) Fast latency check |
| **System** | `/api/metrics/ws` | WS | Live metrics stream tiap 3 detik |
| | `/api/metrics/history` | GET | DB metrics (24h) |
| | `/api/system/update` | POST | Self-update (git pull) |
| | `/api/system/reboot` | POST | OS reboot |
| | `/api/system/os_updates` | GET | APT/Pacman list upgradable |
| | `/api/system/os_updates/ws` | WS | Live OS upgrade stream |
| **Process** | `/api/process/list` | GET | Top proses by CPU/RAM |
| | `/api/process/kill/:pid` | POST | Force kill process |
| **Network** | `/api/network` | GET | Interfaces, IPs, Gateway, Rx/Tx |
| | `/api/ports` | GET | Listening ports + scope |
| | `/api/speedtest/history` | GET | Riwayat speed test (limit 5) |
| | `/api/speedtest/run` | POST | Jalankan speed test |
| **Security** | `/api/firewall/*` | GET/POST | UFW status, toggle, rule, reset |
| | `/api/fail2ban/status` | GET | Status jails + banned IPs |
| | `/api/fail2ban/ban` | POST | Manual ban IP |
| | `/api/fail2ban/unban` | POST | Manual unban IP |
| | `/api/fail2ban/config` | GET/POST | Baca/tulis jail.local |
| | `/api/fail2ban/config/:name` | DELETE | Hapus jail config |
| | `/api/fail2ban/logs` | GET | Tail fail2ban.log |
| | `/api/fail2ban/filters` | GET | List available filters |
| | `/api/fail2ban/reset` | POST | Reset ke default |
| | `/api/fail2ban/install` | POST | Install via apt/pacman |
| **Cloudflare** | `/api/cloudflare/status` | GET | Status tunnel + service |
| | `/api/cloudflare/install` | POST | Install cloudflared |
| | `/api/cloudflare/login` | POST | Authorize cloudflared |
| | `/api/cloudflare/login/status` | GET | Polling status login |
| | `/api/cloudflare/tunnel` | POST | Create tunnel |
| | `/api/cloudflare/tunnel` | DELETE | Delete tunnel |
| | `/api/cloudflare/routes` | GET/POST | Manage DNS routes |
| | `/api/cloudflare/routes/dns` | POST | Register CNAME DNS |
| | `/api/cloudflare/health` | GET | HTTP probe per domain |
| | `/api/cloudflare/logs/ws` | WS | Live journalctl stream |
| | `/api/cloudflare/reset` | POST | Uninstall + reset |
| **Files** | `/api/files/list` | GET | Browse filesystem |
| | `/api/files/info` | GET | Metadata detail satu file |
| | `/api/files/download` | GET | Download file |
| | `/api/files/upload` | POST | Upload file (multipart) |
| | `/api/files/fetch` | POST | Download file dari URL (wget-style) |
| | `/api/files/action` | POST | rename/move/copy/delete/chmod/compress |
| | `/api/files/text` | POST | Baca/tulis file teks |
| **Disk** | `/api/disk/*` | GET/POST | Info, mount, umount |
| **Users** | `/api/users/*` | GET/POST/PUT/DELETE | CRUD user, password, groups, SSH keys |
| | `/api/groups/*` | GET/POST/DELETE | CRUD Linux groups |
| **Cron** | `/api/cron` | GET | Baca /etc/crontab |
| | `/api/cron` | POST | Tulis /etc/crontab |
| **Logs** | `/api/syslogs` | GET | journalctl viewer |
| | `/api/logs/activity` | GET | Audit trail dari DB |
| | `/api/logs/bash_history` | GET | .bash_history user aktif & root |
| **Terminal** | `/api/terminal/ws` | WS | Native PTY shell |
| | `/api/terminal/start` | POST | Start shellinabox (fallback terminal) |
| **Container** | `/api/container/list` | GET | List containers |
| | `/api/container/runtime` | GET | Deteksi runtime (Docker/Podman) |
| | `/api/container/runtime/refresh` | POST | Re-deteksi runtime |
| | `/api/container/create` | POST | Buat container baru |
| | `/api/container/inspect/:id` | GET | Inspect detail container |
| | `/api/container/:id/start` | POST | Start container |
| | `/api/container/:id/stop` | POST | Stop container |
| | `/api/container/:id/remove` | POST | Remove container |
| | `/api/container/:id/logs` | GET | Logs container |
| **Compose** | `/api/compose/list` | GET | List compose projects |
| | `/api/compose/:name/up` | POST | Deploy compose project |
| | `/api/compose/:name/down` | POST | Stop compose project |
| | `/api/compose/:name/rebuild` | POST | Rebuild + restart project |
| | `/api/compose/:name/scale` | POST | Scale service |
| | `/api/compose/:name/yaml` | GET/PUT | Baca/edit compose YAML |
| **VM** | `/api/vm/list` | GET | List VM instances dari DB |
| | `/api/vm/create/ws` | WS | Deploy VM baru via WebSocket wizard |
| | `/api/vm/:name/start` | POST | Start VM |
| | `/api/vm/:name/stop` | POST | Stop VM |
| | `/api/vm/:name/status` | GET | Status satu VM |
| | `/api/vm/:name` | DELETE | Hapus VM (container + volume + DB row) |

---

*Dokumentasi ini merefleksikan status codebase per pembaruan terakhir. Perbarui dokumen ini jika ada penambahan rute, modifikasi tabel DB, atau perubahan arsitektur.*

---

## 7. Catatan Bug Fix & Technical Notes

Bagian ini mendokumentasikan bug yang pernah ditemukan dan cara fix-nya.

### 7.1 WebSocket OS Upgrade — stdout EOF Bug (`system_updates.rs`)

**Bug:** `tokio::select!` dengan dua branch (stdout + stderr) langsung `break` saat stdout return `Ok(None)` (EOF). `apt-get` menulis ke stderr lebih dulu, stdout belum ada output → `Ok(None)` → loop break → WS closed sebelum upgrade dimulai.

**Fix:** Ganti `tokio::select!` dengan dua `tokio::spawn` terpisah yang stream ke `mpsc::channel`. Main task forward dari channel ke WS. Tambah `stdin.flush().await` + `drop(stdin)` sebelum streaming.

**File:** `src/routes/system_updates.rs` — `handle_upgrade_ws()`

---

### 7.2 WebSocket OS Upgrade — JWT Whitelist Bug (`jwt_middleware.rs`)

**Bug:** `/api/system/os_updates/ws` di-whitelist (skip JWT), tapi handler butuh `Extension<AuthUser>` yang hanya ada jika middleware inject. Karena di-skip → `AuthUser` tidak ada → panic → WS dropped.

**Fix:** Hapus dari whitelist. Middleware ambil token dari `?token=` query param.

**File:** `src/auth/jwt_middleware.rs`

---

### 7.3 Cloudflare Create Tunnel — False Positive Guard (`cloudflare.rs`)

**Bug:** `sudo_exec(...).is_ok()` selalu `true` (cek apakah proses berhasil di-spawn, bukan apakah file ada). Create tunnel selalu diblock.

**Fix:** Hapus `|| sudo_exec(...)` — cukup `Path::new(config_path).exists()`.

**File:** `src/routes/cloudflare.rs` — `create_tunnel()`

---

### 7.4 Health Check — Semua Connection Error jadi NXDOMAIN (`cloudflare_api.rs`)

**Bug:** `e.is_connect()` menangkap semua error termasuk connection refused → semua jadi NXDOMAIN. Domain yang DNS-nya ada tapi service mati seharusnya ERR_502.

**Fix:** Cek keyword DNS spesifik (`"dns error"`, `"no such host"`, dll) di error message. Jika tidak cocok tapi `e.is_connect()` → ERR_502.

**File:** `src/routes/cloudflare_api.rs` — `check_health_status()`

---

### 7.5 Hono `verify()` — Missing Algorithm Parameter (`server.ts`)

**Bug:** `verify(token, secret)` tanpa parameter ketiga throw `JwtAlgorithmRequired` → catch block return `null` → semua request ke `/api/frontend/config` return 401, meskipun token dan secret sudah benar.

**Fix:** Tambahkan parameter ketiga eksplisit: `verify(token, secret, 'HS256')`.

**File:** `frontend-vue/server.ts` — `verifySession()`

---

### 7.6 WebSocket Proxy via `fetch()` Tidak Bekerja (`server.ts`)

**Bug:** `app.all('/api/*')` menggunakan `fetch()` untuk proxy semua request ke Rust. `fetch()` tidak mendukung WebSocket upgrade (`101 Switching Protocols`) — browser menerima frame header yang tidak valid → `Invalid frame header` error.

**Fix:** Gunakan `Bun.serve()` native dengan `websocket` handler yang pipe frames bidirectional. Deteksi WebSocket upgrade dari header `Upgrade: websocket` di fungsi `fetch()`, kemudian `server.upgrade(req, { data })`. Handler `open/message/close` membuka koneksi ke Rust dan pipe data dua arah.

**File:** `frontend-vue/server.ts` — `Bun.serve()` + `websocket` handler

---

### 7.7 `overflow-hidden` + `backdrop-filter` Menyebabkan Text Bug (`HomeView.vue`, `ServerLayout.vue`)

**Bug:** Card container dengan `overflow-hidden` + `backdrop-filter: blur()` + CSS animation (`transform: rotate()`) menyebabkan text rendering corrupt — teks tumpang tindih atau ter-clip di Chrome.

**Fix:** Hapus `overflow-hidden` dari card container di HomeView. Untuk ServerLayout nav, pertahankan `overflow-hidden` (diperlukan untuk rendering) tapi tambah `flex-direction: row` eksplisit di `.sidebar-nav-item` CSS karena tanpa `overflow-hidden` ada edge case dimana flex direction tidak direspect.

**File:** `frontend-vue/src/views/HomeView.vue`, `frontend-vue/src/views/ServerLayout.vue`
