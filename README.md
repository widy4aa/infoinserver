<table>
<tr>
<td><img src="frontend-vue/public/server-icon-blue.svg" width="72" alt="InfoIn Server"></td>
<td><h1>InfoIn Server</h1><p><em>Server Linux kamu. Terlihat penuh. Terkendali penuh.</em></p></td>
</tr>
</table>

Dashboard monitoring dan administrasi server Linux yang bisa di-host sendiri. Membaca metrik langsung dari kernel, streaming data secara real-time, dan memberikan antarmuka web modern untuk mengelola seluruh aspek server — tanpa perlu SSH.

Tidak ada agen. Tidak ada telemetri. Tidak ada ketergantungan cloud. Hanya servermu, berbicara langsung denganmu.

---

## Fitur Utama

### Monitoring Real-time
- **Metrik Sistem Live** — CPU, RAM, Disk, dan Network di-stream dari `/proc` setiap 3 detik
- **Grafik Historis** — Data dicatat setiap 5 menit ke SQLite, ditampilkan sebagai grafik interaktif dengan filter rentang waktu
- **Info Sistem** — Hostname, nama OS dengan ikon distro, versi kernel, uptime, user yang login

### Administrasi Sistem
- **Systemd Services** — Lihat, start, stop, restart, enable, dan disable daemon background
- **Process Manager** — Daftar proses teratas (sort CPU/RAM), cari by nama atau PID, force-kill
- **OS Package Updates** — Deteksi paket yang bisa diupgrade via `apt` atau `pacman`, apply upgrade dengan terminal streaming live
- **Cron Job Manager** — Baca, tambah, edit, dan hapus jadwal `/etc/crontab` via form UI
- **System Reset** — Reset satu klik untuk Cloudflare, UFW Firewall, dan Fail2Ban ke kondisi default dari halaman Settings
- **Reboot & Update** — Reboot server dan self-update dashboard langsung dari UI

### Manajemen User & Grup
- **Multi-user Session** — Switch antara beberapa user sudo tanpa logout
- **Linux User Management** — Buat user, ganti password, assign grup, hapus user
- **SSH Key Manager** — Kelola `~/.ssh/authorized_keys` per user (tambah, hapus, validasi format)

### File Explorer
- **Akses Filesystem Penuh** — Browse seluruh filesystem Linux dari `/`. Operasi tulis hanya di `$HOME` dan USB
- **USB/SD Card Sidebar** — Auto-detect media removable, lihat usage, mount/eject dari UI
- **3 Mode Tampilan** — List, Grid (dengan thumbnail gambar), dan Compact
- **File Actions** — Rename, move, copy, delete, compress ZIP, extract, chmod visual
- **Text Editor & Upload** — Editor teks bawaan browser, upload drag-and-drop, download via URL

### Network & Keamanan
- **Network Interfaces** — Interface aktif, MAC, IP, gateway, bar RX/TX
- **Listening Ports** — Daftar port yang listening dengan badge Scope (🌍 Publik / 🔒 Lokal)
- **Port Scanner** — Scan async via `nmap` dengan highlight port berbahaya
- **UFW Firewall** — Lihat status, toggle on/off, kelola rules allow/deny
- **Internet Speedtest** — Uji kecepatan on-demand dengan riwayat 5 tes terakhir
- **Ping Indicator** — Latensi live ke backend server, tampil di navbar

### Intrusion Prevention (Fail2Ban)
- **Status Dashboard** — Jails aktif, jumlah IP ter-ban per jail
- **Manual Ban & Unban** — Ban atau unban IP dari jail manapun
- **Jail Configuration** — Editor visual `jail.local` dengan template service populer (SSH, Nginx, Postfix, dll)
- **Live Activity Log** — Tail `/var/log/fail2ban.log` secara real-time
- **One-click Install** — Install Fail2Ban via `apt` atau `pacman` dari dashboard

### Container Management
- **Multi-runtime** — Otomatis deteksi Docker atau Podman
- **Container Management** — List, start, stop, hapus container; lihat log dan inspect detail
- **Compose** — Deploy project baru via YAML editor, kelola per-service, edit YAML inline

### Cloudflare Tunnel
- **Setup Wizard** — Wizard langkah demi langkah: Install → Authorize → Create Tunnel
- **Command Center** — Status tunnel, start/stop/restart service, kelola routes
- **Health Diagnostics** — Probe HTTP per domain: HEALTHY, ERR_502, ERR_1033, NXDOMAIN
- **Live Logs** — Stream `journalctl -f` cloudflared via WebSocket

### Logs & Audit
- **System Journal** — Viewer `journalctl` dengan filter: All, Auth (SSH), Kernel
- **Dashboard Audit Log** — Setiap aksi admin dicatat ke SQLite (level INFO / WARNING / CRITICAL)
- **Bash History Viewer** — Baca `.bash_history` user aktif dan root

### Developer Tools
- **Terminal Multi-sesi** — Native PTY shell via WebSocket (`xterm.js`), setiap tab browser dapat shell sendiri
- **Dark Mode** — Toggle Moon/Sun di navbar, preferensi tersimpan otomatis

---

## Gambaran Arsitektur

```
┌─────────────────────────────────┐
│   Browser (Vue 3 SPA)           │
│   HTTP REST + 4 WebSocket       │
│   Token JWT per-server          │
└────────────────┬────────────────┘
                 │
                 ▼
┌─────────────────────────────────┐
│   Backend Rust (Axum + Tokio)   │
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

**Backend** dibangun dengan Rust (Axum 0.8, Tokio) — membaca metrik langsung dari kernel, mengeksekusi perintah OS via `sudo`, dan streaming data ke browser melalui WebSocket.

**Frontend** dibangun dengan Vue 3 + Vite + Tailwind CSS v4 — SPA yang berkomunikasi dengan backend via REST API dan 4 channel WebSocket (metrics, terminal PTY, Cloudflare logs, OS upgrade).

**Autentikasi** menggunakan PAM Linux — tidak ada database user terpisah. Login menggunakan kredensial OS asli. Hanya user yang tergabung di grup `sudo` atau `wheel` yang diizinkan.

---

## Cara Menjalankan

```bash
# 1. Salin dan konfigurasi environment
cp .env.example .env
# Edit .env — isi JWT_SECRET dengan string acak yang kuat

# 2. Build dan jalankan backend
chmod +x start.sh && ./start.sh

# 3. Jalankan frontend (pilih salah satu)
podman compose up -d --build   # Frontend di http://localhost:3000
# — atau —
cd frontend-vue && npm install && npm run build  # Serve via backend di http://localhost:8080
```

Setelah berjalan, buka dashboard → **Settings** → **Add Server** → isi URL backend dan login dengan user OS yang ada di grup `sudo`.

---

## Lisensi

MIT License — lihat file `LICENSE` untuk detail.

---

*InfoIn Server adalah proyek open-source independen. Tidak berafiliasi dengan produk monitoring komersial manapun.*
