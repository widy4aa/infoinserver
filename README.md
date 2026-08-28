# Server Monitoring Dashboard

A lightweight, self-hosted server monitoring dashboard with a **Rust backend** (Axum + Tokio) and a **Vue.js frontend**. Built for Linux bare-metal servers — reads metrics directly from the kernel (`/proc`, `/sys`) like Node Exporter, streams real-time data via WebSocket, and secured with PAM-based OS authentication + JWT session tokens.

---

## Features

### System Monitoring
- **Real-time Metrics via WebSocket** — CPU usage, RAM, Uptime, OS, and Kernel info streamed live (no polling).
- **Historical Performance Charts** — CPU, RAM, and Disk usage recorded every 5 minutes to SQLite and displayed as interactive line charts (24h, 12h, 1h, 30m, 10m, 5m range selector).
- **Storage (Disks)** — Per-mount-point disk usage with progress bars.
- **Top Processes** — Live process list sortable by CPU or RAM, with keyword/PID search and force-kill support.

### Network & Security
- **Network Interfaces** — Active interfaces, MAC address, RX/TX traffic in MB.
- **Listening Ports** — All locally listening ports and their associated processes via `ss`, with kill-process action.
- **Deep Port Scan** — On-demand async port scanning via `nmap` with background job queue.
- **UFW Firewall** — View status, toggle enable/disable, add/deny rules.

### Container Management
- **Podman Integration** — List containers, view status, Start / Stop / Restart / Delete, and deploy new containers from the UI.

### File Management
- **File Explorer** — Sandboxed web file browser (scoped to `FILE_ROOT`). View, download, upload files, and fetch remote URLs via `wget`. Protected against Path Traversal attacks.

### Developer Tools
- **Multi-Tab Root Terminal** — Native PTY shell via WebSocket (`xterm.js`). Multiple tabs, each with its own independent shell session.
- **Cloudflare Integration** — Manage Cloudflare tunnels (Quick Tunnel & Managed Tunnel with token) and Zero Trust ingress routes via API.
- **Speedtest** — Scheduled (hourly) and on-demand network speed testing via `speedtest-cli`. Results stored and displayed in history table.

### Activity Logs & Alerts
- **Automatic Alerts** — Background scheduler detects anomalies and writes to `activity_log` table:
  - CPU usage > 90%
  - RAM usage > 90%
  - Disk free space < 10%
- **Logs & Alerts tab** — View all recorded alerts with severity levels (INFO / WARNING / CRITICAL).

### Security
- **PAM Authentication** — Login uses the server's actual Linux OS credentials (username + password verified via `libpam`). No separate user management needed.
- **JWT Session Tokens** — After login, a signed JWT is issued (valid for 24h). All API endpoints are protected.
- **Per-Server Sessions** — Each server in the dashboard has its own login session, stored in `sessionStorage` (cleared when browser tab closes or user navigates back to Home).
- **Sudo Injection** — Commands requiring elevated privileges (`kill`, `reboot`, UFW) use the user's authenticated password injected via `sudo -S` stdin — never stored on disk.

---

## Tech Stack

| Layer | Technology |
|---|---|
| **Backend** | Rust, Axum 0.8, Tokio, SQLx (SQLite), portable-pty |
| **Metrics Source** | `/proc/stat`, `/proc/meminfo`, `/proc/uptime`, `/proc/mounts`, `statvfs()` (no sysinfo for metrics) |
| **Auth** | PAM (`libpam`), JWT (`jsonwebtoken`) |
| **Frontend** | Vue 3, Vite, Tailwind CSS v4, Chart.js, xterm.js |
| **Deployment** | Backend: bare-metal binary · Frontend: Nginx via Docker/Podman Compose |

---

## OS Support

Designed specifically for **Linux** environments due to direct `/proc` and `/sys` filesystem access.

| Distro | Status |
|---|---|
| Arch Linux / CachyOS / Manjaro | ✅ Fully Supported & Tested |
| Ubuntu / Debian | ✅ Fully Supported & Tested |
| Windows (Native) | ❌ Unsupported |
| macOS | ❌ Unsupported |

---

## Prerequisites

### Backend Host (bare-metal)

```bash
# Arch Linux / CachyOS / Manjaro
sudo pacman -S rustup libpam
rustup default stable
sudo pacman -S iproute2 podman nmap wget ufw speedtest-cli

# Ubuntu / Debian
sudo apt install curl build-essential libpam0g-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
sudo apt install iproute2 podman nmap wget ufw speedtest-cli
```

> **Note:** `libpam-dev` / `libpam0g-dev` is required to compile the PAM authentication module.

### Frontend Host

```bash
# Docker Compose or Podman Compose
docker --version   # or: podman --version
```

---

## Deployment Architecture

```
┌─────────────────────────────────────┐    ┌────────────────────────────────┐
│        Frontend (Vue.js SPA)        │    │     Backend Agent (Rust)        │
│  Hosted anywhere via Nginx/Podman   │◄──►│  Must run on bare-metal Linux   │
│  http://your-dashboard-host:3000    │    │  http://<SERVER_IP>:8080        │
└─────────────────────────────────────┘    └────────────────────────────────┘
                                                        │
                                                Reads from kernel:
                                                /proc/stat
                                                /proc/meminfo
                                                /proc/uptime
                                                /proc/mounts
                                                /proc/net/dev
                                                /sys/class/net/
```

The frontend is a standard SPA that can be hosted anywhere (even a different server or country). You can add **multiple Backend Agent IPs** into a single Frontend dashboard.

---

## Backend Setup

### 1. Configure `.env`

```env
PORT=8080
FILE_ROOT=/home/user/Documents
DB_PATH=sqlite:./data.db
JWT_SECRET=your-random-secret-string-here
```

> **Important:** Set a strong `JWT_SECRET`. This is used to sign all session tokens. Without it, a default insecure value is used.

### 2. Build & Run

```bash
chmod +x start.sh
./start.sh
```

This script runs `cargo build --release` and spawns the backend daemon in the background. Logs are written to `server.log`.

---

## Frontend Setup

### Via Podman Compose (Recommended)

```bash
# Build and run
podman compose up -d --build

# Or using Docker
docker compose up -d --build
```

Open `http://localhost:3000` in your browser.

### Via npm (Development)

```bash
cd frontend-vue
npm install
npm run dev
```

---

## Adding a Server

1. Open the dashboard and click **"Add Server"** (top right of Home page).
2. Fill in:
   - **Server Alias** — A friendly name (e.g. "VPS Singapore")
   - **Backend IP / URL** — Just enter the IP:Port (e.g. `100.127.55.109:8080`). `http://` is added automatically.
   - **Username** — Your Linux OS username on that server.
   - **Password** — Your Linux OS password.
3. Click **"Add & Login"** — The dashboard will authenticate against the server's PAM system immediately. If successful, you're redirected to the server dashboard.

---

## Database Schema

The backend uses SQLite (`data.db`) with the following tables:

| Table | Purpose |
|---|---|
| `system_metrics_history` | CPU, RAM, Disk usage recorded every 5 minutes for historical charts |
| `activity_log` | Automatic alerts (High CPU/RAM, Low Disk) with severity levels |
| `speedtest_history` | Network speed test results (scheduled hourly + on-demand) |
| `port_scan_jobs` | Async nmap scan jobs with status tracking |

The database is **auto-created and migrated** on first run. Incremental migrations (ALTER TABLE) are applied automatically if columns are missing.

---

## Security Notes

- All `/api/*` endpoints require a valid JWT token in the `Authorization: Bearer` header, **except** `/api/auth/login`.
- WebSocket endpoints (`/api/terminal/ws`, `/api/metrics/ws`) accept the token via query parameter `?token=...` since browsers cannot set custom headers on WebSocket connections.
- The backend **never logs passwords**. The user's password is held in RAM only for the duration of a `sudo` call, never written to disk.
- **Do NOT expose this dashboard directly to the public internet without a reverse proxy.** Always run behind Nginx/Caddy with SSL/TLS.

---

## License

MIT License
