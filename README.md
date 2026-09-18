<div align="center">
  <img src="frontend-vue/public/server-icon-blue.svg" width="80" alt="InfoIn Server" />
  <h1>InfoIn Server</h1>
  <p><em>Your Linux server. Fully visible. Fully in control.</em></p>
</div>

A self-hosted, open-source server monitoring and administration dashboard for Linux. Reads metrics directly from the kernel, streams data in real-time, and gives you a modern web interface to manage every aspect of your server — no SSH required.

No agents. No telemetry. No cloud dependency. Just your server, talking to you.

---

## Features

### Real-time Monitoring
- **Live System Metrics** — CPU, RAM, Disk, and Network streamed from `/proc` every 3 seconds
- **Historical Charts** — Data recorded every 5 minutes to SQLite, displayed as interactive charts with time range filters
- **System Info** — Hostname, OS name with distro icon, kernel version, uptime, logged-in user

### Home Dashboard
- **Server Ping Indicators** — Each server card shows live latency (online/offline) with sonar pulse animation
- **Edit Mode** — Pencil button toggles edit mode with fluid jiggle animation; delete servers with X button
- **Server Labels / Groups** — Organize servers into labeled groups, drag & drop cards across labels
- **Add Server Modal** — Add a new server directly from the home screen without navigating away
    - **Cross-browser Config Sync** — Server list, labels, and order stored in shared global config (Bun SQLite). Admin and Master roles can write; Slave roles are read-only

### System Administration
- **Systemd Services** — List, start, stop, restart, enable, and disable background daemons
- **Process Manager** — Top process list (sorted by CPU/RAM), search by name or PID, force-kill
- **OS Package Updates** — Detect upgradable packages via `apt` or `pacman`, apply upgrades with a live streaming terminal
- **Cron Job Manager** — Read, add, edit, and delete `/etc/crontab` jobs via a structured form UI
- **System Reset** — One-click reset for Cloudflare, UFW Firewall, and Fail2Ban to default state from the Settings page
- **Reboot & Update** — Reboot the server and self-update the dashboard directly from the UI

### User & Group Management
- **Multi-user Sessions** — Switch between multiple sudo users without logging out
- **Linux User Management** — Create users, change passwords, assign groups, delete users
- **SSH Key Manager** — Manage `~/.ssh/authorized_keys` per user (add, delete, format validation)

### File Explorer
- **Full Filesystem Access** — Browse the entire Linux filesystem from `/`. Write operations restricted to `$HOME` and USB drives
- **USB/SD Card Sidebar** — Auto-detect removable media, view usage, mount/eject from the UI
- **3 View Modes** — List, Grid (with image thumbnails), and Compact
- **File Actions** — Rename, move, copy, delete, compress to ZIP, extract, visual chmod
- **Text Editor & Upload** — Built-in browser text editor, drag-and-drop upload, download via URL

### Network & Security
- **Network Interfaces** — Active interfaces, MAC, IP, gateway, RX/TX bars
- **Listening Ports** — All locally listening ports with Scope badges (🌍 Public / 🔒 Local)
- **Port Scanner** — On-demand async scan via `nmap` with dangerous port highlighting
- **UFW Firewall** — View status, toggle on/off, manage allow/deny rules
- **Internet Speedtest** — On-demand speed test with 5-result history
- **Ping Indicator** — Live latency to backend server, shown in the sidebar

### Intrusion Prevention (Fail2Ban)
- **Status Dashboard** — Active jails, banned IP count per jail
- **Manual Ban & Unban** — Ban or unban any IP from any jail
- **Jail Configuration** — Visual editor for `jail.local` with quick-add templates (SSH, Nginx, Postfix, etc.)
- **Live Activity Log** — Tail `/var/log/fail2ban.log` in real-time
- **One-click Install** — Install Fail2Ban via `apt` or `pacman` directly from the dashboard

### Container Management
- **Multi-runtime Support** — Automatically detects Docker or Podman
- **Container Management** — List, start, stop, remove containers; view logs and inspect details
- **Compose** — Deploy new projects via YAML editor, manage per-service, edit YAML inline
- **VM Management** — Deploy lightweight virtual machines (Ubuntu 24.04, Arch) as Podman containers with systemd as PID 1; managed via a WebSocket deployment wizard

### Cloudflare Tunnel
- **Setup Wizard** — Step-by-step: Install → Authorize → Create Tunnel
- **Command Center** — Tunnel status, start/stop/restart service, manage routes
- **Health Diagnostics** — HTTP probe per domain: HEALTHY, ERR_502, ERR_1033, NXDOMAIN
- **Live Logs** — Stream `journalctl -f` for cloudflared via WebSocket

### Logs & Audit
- **System Journal** — `journalctl` viewer with filters: All, Auth (SSH), Kernel
- **Dashboard Audit Log** — Every admin action logged to SQLite (INFO / WARNING / CRITICAL)
- **Bash History Viewer** — Read `.bash_history` for the active user and root

### Authentication & Access Control
- **GitHub OAuth** — Login via GitHub account, handled by Bun server (not Rust)
- **Linux PAM** — Per-server authentication using real OS credentials (sudo/wheel group required)
- **Multi-user Sessions** — Hold tokens for multiple OS users per server, switch without re-entering passwords
- **Role-Based Access Control** — Three built-in roles (`admin`, `master`, `slave`) plus custom roles with configurable group access. Admin/Master can modify the server list; Slave is read-only

### Developer Tools
- **Multi-session Terminal** — Native PTY shell via WebSocket (`xterm.js`), each browser tab gets its own shell
- **Dark Mode** — Moon/Sun toggle, preference saved automatically to localStorage

---

## Architecture

```
┌─────────────────────────────────────┐
│  Browser (Vue 3 SPA)                │
│  http://host:3000                   │
│  HTTP REST + WebSocket              │
└────────────────┬────────────────────┘
                 │ HTTP / WebSocket
                 ▼
┌─────────────────────────────────────┐
│  Bun + Hono (port 3000)             │
│  - GitHub OAuth handler             │
│  - Frontend config sync (SQLite)    │
│  - HTTP proxy → Rust                │
│  - WebSocket proxy → Rust (native)  │
│  - Serve Vue static files           │
└────────────────┬────────────────────┘
                 │ HTTP / WebSocket
                 ▼
┌─────────────────────────────────────┐
│  Rust — Axum 0.8 + Tokio (port 8080)│
│  PAM Auth · sudo -S injection       │
│  SQLite (SQLx async)                │
│  Background scheduler               │
└────────────────┬────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────┐
│  Linux Kernel                       │
│  /proc · /sys · /dev · lsblk        │
│  systemctl · apt · journalctl       │
└─────────────────────────────────────┘
```

**Bun + Hono server** (`frontend-vue/server.ts`) — The middle layer between browser and Rust. Handles GitHub OAuth (secret never exposed to browser), enforces role-based access control (RBAC) for the global config, proxies all `/api/*` HTTP requests to Rust, and proxies WebSocket connections bidirectionally using Bun's native `websocket` handler. Also persists frontend configuration (server list, labels, order) to a local SQLite database (`frontend.db`) as a **single shared global config** — readable by all authenticated users, writable only by `admin` and `master` roles.

**Rust backend** (`src/`) — Axum 0.8 + Tokio. Reads metrics directly from the kernel, executes OS commands via `sudo`, streams data over WebSocket. Does **not** handle GitHub OAuth (moved to Bun). Does **not** serve the frontend.

**Frontend** — Vue 3 + Vite + Tailwind CSS v4. Built to `static/` directory, served by Bun in production. In development, Bun proxies non-API requests to Vite dev server (port 5173).

**Authentication** — Two-layer:
1. **GitHub OAuth** — Identity verification, handled entirely by Bun server. RBAC roles (`admin`/`master`/`slave`/custom) assigned per GitHub user.
2. **Linux PAM** — Per-server credential verification via Rust. Only users in `sudo` or `wheel` group are allowed. Root login is blocked.

---

## Getting Started

### Prerequisites

```bash
# Rust backend dependencies
# Debian/Ubuntu
sudo apt install build-essential pkg-config libclang-dev libpam0g-dev

# Arch Linux
sudo pacman -S base-devel clang pam

# Bun (for frontend server)
curl -fsSL https://bun.sh/install | bash
```

### 1. Configure Rust Backend

```bash
cp .env.example .env
```

Edit `.env`:

```env
PORT=8080
JWT_SECRET=your-strong-random-secret
CORS_ORIGIN=http://YOUR_IP:3000
```

### 2. Configure Bun Server

```bash
cp frontend-vue/.env.example frontend-vue/.env
```

Edit `frontend-vue/.env`:

```env
FRONTEND_PORT=3000
RUST_BACKEND_URL=http://YOUR_IP:8080
FRONTEND_URL=http://YOUR_IP:3000

# GitHub OAuth App (create at https://github.com/settings/developers)
# Authorization callback URL must be: http://YOUR_IP:3000/api/auth/github/callback
GITHUB_CLIENT_ID=your-client-id
GITHUB_CLIENT_SECRET=your-client-secret
GITHUB_REDIRECT_URI=http://YOUR_IP:3000/api/auth/github/callback
GITHUB_SESSION_SECRET=your-random-session-secret

# Must be identical to JWT_SECRET in root .env
JWT_SECRET=your-strong-random-secret
```

### 3. Start Backend

```bash
chmod +x start.sh && ./start.sh --backend
# Rust backend running at http://YOUR_IP:8080
```

### 4. Start Frontend

```bash
# Production (serves built static files)
./start.sh --frontend

# Development (Vite HMR + Bun proxy — auto-reload on file changes)
./start.sh --frontend --dev

# Start both backend + frontend at once
./start.sh
```

### 5. Open the Dashboard

1. Go to `http://YOUR_IP:3000`
2. Click **Sign in with GitHub**
3. After OAuth, you'll see the home screen
4. Click **Add Server** → enter backend URL → login with a Linux sudo user

---

## Script Usage

```bash
# Start
./start.sh                   # backend + frontend (production)
./start.sh --backend         # Rust only
./start.sh --frontend        # Bun server only (production)
./start.sh --frontend --dev  # Bun + Vite dev mode (HMR)
./start.sh --dev             # backend + frontend dev mode

# Stop
./stop.sh                    # stop all
./stop.sh --backend          # stop Rust only
./stop.sh --frontend         # stop Bun + Vite (if running)
```

**Log files:**

| Process | Log file |
|---|---|
| Rust backend | `server.log` |
| Bun server | `frontend.log` |
| Vite dev server | `vite.log` |

---

## Environment Variables

Ada **2 file `.env`** yang perlu dikonfigurasi — satu untuk Rust backend, satu untuk Bun server. Keduanya **tidak saling berbagi** file, kecuali `JWT_SECRET` yang **harus identik** di keduanya.

```
infoinserver/
├── .env                  ← Rust backend
└── frontend-vue/
    └── .env              ← Bun + Hono server (GitHub OAuth, proxy, config)
```

> **Penting:** `JWT_SECRET` di `.env` dan `frontend-vue/.env` harus **sama persis**. Bun menandatangani JWT, Rust memverifikasinya dengan secret yang sama.

---

### Rust Backend (`.env`)

Template: `cp .env.example .env`

| Variable | Default | Required | Description |
|---|---|---|---|
| `PORT` | `8080` | No | Port HTTP server Rust |
| `DB_PATH` | `sqlite:./data.db` | No | Path file SQLite metrics. Dibuat otomatis jika belum ada |
| `JWT_SECRET` | *(fallback tidak aman)* | **Yes** | Secret untuk sign & verify JWT Linux user session. Harus sama dengan `JWT_SECRET` di `frontend-vue/.env` |
| `FILE_ROOT` | `$HOME` | No | Root direktori untuk operasi tulis di File Explorer (upload, delete, rename, chmod). `$HOME` di-expand ke home dir user aktif |
| `CORS_ORIGIN` | `http://localhost:3000` | **Yes** | Origin yang diizinkan CORS — harus sama dengan URL Bun server (`FRONTEND_URL`) |

**Variabel OS-level** yang juga dibaca Rust dari environment shell (tidak perlu di-set di `.env`, sudah otomatis tersedia):

| Variable | Dibaca di | Keterangan |
|---|---|---|
| `HOME` | `src/routes/files.rs`, `src/routes/cloudflare.rs`, `src/routes/logs.rs` | Home directory user yang menjalankan Rust |
| `USER` | `src/routes/logs.rs`, `src/services/proc_reader.rs` | Username aktif. Jika tidak ada, fallback ke `LOGNAME` |
| `LOGNAME` | `src/services/proc_reader.rs` | Fallback username jika `USER` tidak ter-set |

---

### Bun Server (`frontend-vue/.env`)

Template: `cp frontend-vue/.env.example frontend-vue/.env`

| Variable | Default | Required | Description |
|---|---|---|---|
| `FRONTEND_PORT` | `3000` | No | Port listen Bun server |
| `RUST_BACKEND_URL` | `http://localhost:8080` | **Yes** | URL internal ke Rust backend. Semua `/api/*` request di-proxy ke sini |
| `FRONTEND_URL` | `http://localhost:3000` | **Yes** | URL publik Bun server. Dipakai sebagai base redirect setelah GitHub OAuth |
| `JWT_SECRET` | *(fallback tidak aman)* | **Yes** | Harus **sama persis** dengan `JWT_SECRET` di root `.env` (Rust backend) |
| `GITHUB_CLIENT_ID` | — | **Yes** | Client ID dari GitHub OAuth App |
| `GITHUB_CLIENT_SECRET` | — | **Yes** | Client Secret dari GitHub OAuth App |
| `GITHUB_REDIRECT_URI` | `{FRONTEND_URL}/api/auth/github/callback` | **Yes** | Harus sama persis dengan "Authorization callback URL" di GitHub OAuth App settings |
| `GITHUB_SESSION_SECRET` | *(fallback tidak aman)* | **Yes** | Secret untuk sign session token GitHub OAuth. Gunakan random string kuat |
| `FRONTEND_DB_PATH` | `./frontend.db` | No | Path SQLite untuk menyimpan frontend config (server list, labels, urutan) |
| `DEFAULT_ADMIN` | `widy4aa` | **Yes** | GitHub username yang dijadikan sumber global config bersama. Ganti dengan GitHub username kamu sendiri |
| `NODE_ENV` | *(unset = dev mode)* | No | Set ke `production` untuk serve static files hasil build Vite. Omit untuk dev mode |
| `VITE_DEV_URL` | `http://localhost:5173` | No | URL Vite dev server. Hanya aktif saat `NODE_ENV != production`. Uncomment di `.env.example` jika perlu |

---

### Podman Compose (`docker-compose.yml`)

Saat menggunakan `podman compose up`, beberapa variabel **wajib diset di shell host** (atau via file `.env` di root project) karena di-interpolasi langsung oleh compose:

```bash
# Set di shell sebelum menjalankan podman compose:
export FRONTEND_URL=http://YOUR_IP:3000
export JWT_SECRET=your-strong-random-secret
export GITHUB_CLIENT_ID=your-client-id
export GITHUB_CLIENT_SECRET=your-client-secret
export GITHUB_REDIRECT_URI=http://YOUR_IP:3000/api/auth/github/callback
export GITHUB_SESSION_SECRET=your-session-secret

podman compose up -d --build
```

| Variable di Compose | Behavior | Nilai Hardcoded di Compose |
|---|---|---|
| `NODE_ENV` | Hardcoded `production` | Ya |
| `FRONTEND_PORT` | Hardcoded `3000` | Ya |
| `RUST_BACKEND_URL` | Hardcoded `http://host.gateway.internal:8080` | Ya (route ke host) |
| `FRONTEND_URL` | `${FRONTEND_URL:-http://127.0.0.1:3000}` | Fallback ke `127.0.0.1:3000` |
| `JWT_SECRET` | `${JWT_SECRET}` — **wajib**, tanpa fallback | Tidak |
| `GITHUB_CLIENT_ID` | `${GITHUB_CLIENT_ID}` — **wajib**, tanpa fallback | Tidak |
| `GITHUB_CLIENT_SECRET` | `${GITHUB_CLIENT_SECRET}` — **wajib**, tanpa fallback | Tidak |
| `GITHUB_REDIRECT_URI` | `${GITHUB_REDIRECT_URI}` — **wajib**, tanpa fallback | Tidak |
| `GITHUB_SESSION_SECRET` | `${GITHUB_SESSION_SECRET}` — **wajib**, tanpa fallback | Tidak |

> `DEFAULT_ADMIN` dan `FRONTEND_DB_PATH` tidak ada di `docker-compose.yml` — jika ingin menggantinya saat pakai compose, tambahkan manual ke blok `environment:` di file tersebut.

> Rust backend **tidak dijalankan** lewat compose — hanya Bun server yang dikontainerisasi. Rust tetap harus dijalankan terpisah di host.

---

## Production (Podman Compose)

```bash
podman compose up -d --build
# Bun server berjalan di http://YOUR_IP:3000
# Rust backend harus sudah berjalan terpisah di host (port 8080)
```

---

## License

MIT License — see `LICENSE` for details.

---

*InfoIn Server is an independent open-source project. Not affiliated with any commercial monitoring product.*
