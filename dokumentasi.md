# Dokumentasi Internal (Untuk AI Agent & Developer)

Dokumen ini ditujukan bagi AI Agent atau Developer yang akan memelihara, membaca, atau melanjutkan pengembangan proyek **Server Monitoring Dashboard**. Proyek ini adalah aplikasi pemantauan dengan arsitektur *Decoupled* (Backend API dan Frontend SPA terpisah secara logika).

## 1. Arsitektur Utama
- **Backend Framework**: `axum` (berbasis `tokio` dan `hyper`). Menggunakan port `8080`.
- **Database Backend**: SQLite via `sqlx` (asynchronous, pure Rust).
- **Frontend Framework**: Vue.js 3, Vite, Tailwind CSS 4, dan Lucide Icons.
- **Terminal Web**: Menggunakan `portable-pty` di sisi backend (Rust) dan `xterm.js` di sisi frontend (Vue) yang saling berkomunikasi menggunakan protokol `WebSockets`.
- **Paradigma Komunikasi**: Frontend Vue berkomunikasi dengan backend Rust menggunakan API HTTP standar (`fetch`) dan memanfaatkan `localStorage` browser untuk menyimpan daftar IP Backend Server secara statis.

## 2. Struktur Direktori dan File
```text
.
├── Cargo.toml          # Dependensi Rust (axum, tokio, sqlx, sysinfo, reqwest, dll)
├── .env                # Konfigurasi environment (PORT, FILE_ROOT, DB_PATH)
├── start.sh            # Script utilitas build & run backend ke background
├── docker-compose.yml  # Deployment frontend terisolasi (Nginx Alpine)
├── Dockerfile.frontend # Multi-stage build Vite -> Nginx
├── src/
│   ├── main.rs         # Entry point, inisialisasi state, CORS, & Axum Router
│   ├── routes/         # Layer HTTP Handlers (Menerima request, membalas JSON)
│   │   ├── system.rs       # OS Info
│   │   ├── system_mgmt.rs  # Git Pull (Update) & Reboot Server
│   │   ├── firewall.rs     # Manipulasi UFW rules
│   │   ├── cloudflare.rs   # Manajemen daemon cloudflared
│   │   ├── cloudflare_api.rs # Integrasi Cloudflare Zero Trust (Ingress Routes)
│   │   ├── files.rs        # Download, Upload (Multipart), Fetch URL (wget)
│   │   ├── process_mgmt.rs # Top CPU Process & Kill task
│   │   ├── terminal_ws.rs  # PTY WebSocket handler
│   │   └── podman_*.rs     # Manajemen Container & Modal Logs
│   ├── services/       # Layer Bisnis Logik & OS Wrapper
│   │   └── ...         # Berbagai fungsi parser & command execution
│   └── background/     # Scheduler berjalan di tokio::spawn
└── frontend-vue/       # Folder source code Vue 3 (Vite)
    ├── vite.config.js  # Konfigurasi output build Vue ke direktori ../static
    └── src/
        ├── App.vue             # Entry point Vue & Modal Terminal Global
        ├── views/              # Halaman: Home, Dashboard, Podman, Cloudflare, dll
        ├── components/         # Reusable UI (ToastAlert, NativeTerminal, dll)
        └── stores/             # State Management lokal (serverStore.js, toastStore.js)
```

## 3. Catatan Keamanan Penting (Security Context)

Saat mengembangkan lebih lanjut, aturan berikut **WAJIB** dipatuhi:

1. **Subprocess (Command Injection Prevention)**:
   - JANGAN PERNAH menggunakan `sh -c` yang digabung (*concatenate*) dengan string input dari user.
   - Gunakan `std::process::Command` dengan meneruskan parameter lewat pemanggilan `.args()` secara terpisah. Pengecualian hanya untuk perintah sederhana tanpa variabel dinamis atau yang sudah tervalidasi secara *hardcoded regex*.
2. **File Explorer (Path Traversal Protection)**:
   - Modul `files.rs` menggunakan fungsi `resolve_and_validate_path()` yang melakukan `canonicalize()` lalu mengecek apakah target path berawal dari `FILE_ROOT` `.env`. Jangan pernah mem- *bypass* ini.
3. **CORS & Autentikasi**:
   - Backend mem- *bypass* CORS (`Any`) agar frontend Vue dari mesin lain bisa terhubung. Jangan *expose* port Backend ke publik (`0.0.0.0`) tanpa menggunakan VPN atau Reverse Proxy seperti Nginx/Caddy (TLS & Basic Auth) pada tahap *production*.

## 4. Cara Kerja Fitur Spesifik

- **Terminal**: Backend merentangkan (spawn) `/bin/bash` ke dalam virtual PTY (`portable-pty`). Input dari WebSocket `xterm.js` ditulis langsung ke *stdin* PTY, sementara output PTY dikirim per *byte* kembali ke WebSocket secara sinkron.
- **Top Processes**: Memanfaatkan `sysinfo` untuk mengambil 50 list proses terberat (berdasarkan CPU).
- **Cloudflare API**: Pengguna menyimpan Token Cloudflare. Dashboard akan menarik seluruh Ingress Rules dari *Cloudflare Zero Trust API* dan dapat menyisipkan sub-domain (seperti `test.domain.com`) untuk merouting lokal port (`127.0.0.1:8080`) secara otomatis.
- **File Upload/Fetch**: Menggunakan *Multipart streaming* agar efisien dalam RAM. Fitur *Fetch* mengeksekusi `wget -nc <url>` dari direktori saat itu.

## 5. Deployment Flow
1. Vue `npm run build-only` mengekspor JS/CSS/HTML ke folder `/static` induk.
2. Backend Rust `cargo build --release` mengompilasi binary dan siap berjalan mem- *serve* port 8080 API beserta fallback `/static`.
3. Alternatif lain: Backend berjalan di *host* fisik (untuk kemudahan `sudo`, `nmap`, dsb) sementara Frontend dilayani oleh kontainer Nginx (`docker-compose.yml`) port 3000.
