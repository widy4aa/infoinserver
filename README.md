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

### Developer Tools
- **Multi-session Terminal** — Native PTY shell via WebSocket (`xterm.js`), each browser tab gets its own shell
- **Dark Mode** — Moon/Sun toggle in the navbar, preference saved automatically

---

## Architecture

```
┌─────────────────────────────────┐
│   Browser (Vue 3 SPA)           │
│   HTTP REST + 4 WebSocket       │
│   JWT token per server          │
└────────────────┬────────────────┘
                 │
                 ▼
┌─────────────────────────────────┐
│   Backend — Rust (Axum + Tokio) │
│   PAM Auth · sudo -S injection  │
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

**Backend** — Rust (Axum 0.8, Tokio). Reads metrics directly from the kernel, executes OS commands via `sudo`, and streams data to the browser over WebSocket.

**Frontend** — Vue 3 + Vite + Tailwind CSS v4. Communicates with the backend via REST API and 4 WebSocket channels (metrics, terminal PTY, Cloudflare logs, OS upgrade).

**Authentication** — Linux PAM. No separate user database. Login uses real OS credentials. Only users in the `sudo` or `wheel` group are allowed.

---

## Getting Started

```bash
# 1. Configure environment
cp .env.example .env
# Edit .env — set JWT_SECRET to a strong random string

# 2. Build and start the backend
chmod +x start.sh && ./start.sh

# 3. Start the frontend (pick one)
podman compose up -d --build      # Frontend at http://localhost:3000
# — or —
cd frontend-vue && npm install && npm run build  # Served by backend at http://localhost:8080
```

Once running, open the dashboard → **Settings** → **Add Server** → enter the backend URL and log in with an OS user in the `sudo` group.

---

## License

MIT License — see `LICENSE` for details.

---

*InfoIn Server is an independent open-source project. Not affiliated with any commercial monitoring product.*
