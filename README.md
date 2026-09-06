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
- **Ping Indicator** — Live latency to backend server, shown in the navbar

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

### Cloudflare Tunnel
- **Setup Wizard** — Step-by-step: Install → Authorize → Create Tunnel
- **Command Center** — Tunnel status, start/stop/restart service, manage routes
- **Health Diagnostics** — HTTP probe per domain: HEALTHY, ERR_502, ERR_1033, NXDOMAIN
- **Live Logs** — Stream `journalctl -f` for cloudflared via WebSocket

### Logs & Audit
- **System Journal** — `journalctl` viewer with filters: All, Auth (SSH), Kernel
- **Dashboard Audit Log** — Every admin action logged to SQLite (INFO / WARNING / CRITICAL)
- **Bash History Viewer** — Read `.bash_history` for the active user and root

### Authentication
- **GitHub OAuth** — Login via GitHub account (primary authentication)
- **Linux PAM** — Per-server authentication using real OS credentials (sudo/wheel group required)
- **Multi-user Sessions** — Hold tokens for multiple OS users per server, switch without re-entering passwords

### Developer Tools
- **Multi-session Terminal** — Native PTY shell via WebSocket (`xterm.js`), each browser tab gets its own shell
- **Dark Mode** — Moon/Sun toggle, preference saved automatically to localStorage

---

## Architecture

```
┌─────────────────────────────────┐
│   Browser (Vue 3 SPA)           │
│   http://host:3000              │
│   HTTP REST + 4 WebSocket       │
│   JWT token per server          │
└────────────────┬────────────────┘
                 │ REST API / WebSocket
                 ▼
┌─────────────────────────────────┐
│   Backend — Rust (Axum + Tokio) │
│   http://host:8080              │
│   PAM Auth · GitHub OAuth       │
│   sudo -S injection             │
│   SQLite (SQLx async)           │
│   Background scheduler          │
└────────────────┬────────────────┘
                 │
                 ▼
┌─────────────────────────────────┐
│   Linux Kernel                  │
│   /proc · /sys · /dev · lsblk   │
│   systemctl · apt · journalctl  │
└─────────────────────────────────┘
```

**Backend** — Rust (Axum 0.8, Tokio). Fully decoupled from frontend. Reads metrics directly from the kernel, executes OS commands via `sudo`, and streams data to the browser over WebSocket. Does **not** serve the frontend.

**Frontend** — Vue 3 + Vite + Tailwind CSS v4. Runs independently on port 3000. Communicates with the backend via REST API and 4 WebSocket channels. In development, Vite proxies `/api/*` requests to the backend.

**Authentication** — Two-layer:
1. **GitHub OAuth** — Identity verification for dashboard access
2. **Linux PAM** — Per-server credential verification. Only users in `sudo` or `wheel` group are allowed. Root login is blocked.

---

## Getting Started

### Prerequisites

```bash
# Debian/Ubuntu
sudo apt install build-essential pkg-config libclang-dev libpam0g-dev

# Arch Linux
sudo pacman -S base-devel clang pam
```

### 1. Configure Environment

```bash
cp .env.example .env
```

Edit `.env` and set the required values:

```env
PORT=8080
JWT_SECRET=your-strong-random-secret

# GitHub OAuth (create app at https://github.com/settings/developers)
GITHUB_CLIENT_ID=your-client-id
GITHUB_CLIENT_SECRET=your-client-secret
GITHUB_REDIRECT_URI=http://YOUR_IP:8080/api/auth/github/callback

# Frontend URL (for OAuth redirect after login)
FRONTEND_URL=http://YOUR_IP:3000

# CORS — allow frontend origin to access backend API
CORS_ORIGIN=http://YOUR_IP:3000
```

### 2. Start the Backend

```bash
chmod +x start.sh && ./start.sh
# Backend running at http://YOUR_IP:8080
```

### 3. Start the Frontend

**Development (recommended):**
```bash
cd frontend-vue
npm install
npm run dev
# Frontend at http://YOUR_IP:3000
# /api/* requests proxied to :8080 automatically
```

**Production (Docker/Podman):**
```bash
podman compose up -d --build
# Frontend served via Nginx at http://YOUR_IP:3000
```

### 4. Open the Dashboard

1. Go to `http://YOUR_IP:3000`
2. Click **Sign in with GitHub**
3. After OAuth, you'll see the server dashboard
4. Click **Add Server** → enter backend URL → login with a Linux sudo user

---

## Environment Variables

| Variable | Default | Required | Description |
|---|---|---|---|
| `PORT` | `8080` | No | Backend HTTP port |
| `FILE_ROOT` | `$HOME` | No | Root path for file write operations |
| `DB_PATH` | `sqlite:./data.db` | No | SQLite database path |
| `JWT_SECRET` | *(insecure fallback)* | **Yes** | JWT signing secret — use a strong random value |
| `GITHUB_CLIENT_ID` | — | **Yes** | GitHub OAuth App client ID |
| `GITHUB_CLIENT_SECRET` | — | **Yes** | GitHub OAuth App client secret |
| `GITHUB_REDIRECT_URI` | — | **Yes** | Must match the callback URL in your GitHub OAuth App |
| `FRONTEND_URL` | `http://localhost:3000` | **Yes** | Frontend URL — used for OAuth redirect after login |
| `CORS_ORIGIN` | `http://localhost:3000` | **Yes** | Allowed CORS origin for frontend requests |

---

## License

MIT License — see `LICENSE` for details.

---

*InfoIn Server is an independent open-source project. Not affiliated with any commercial monitoring product.*
