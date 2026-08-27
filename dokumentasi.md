# Dokumentasi Internal (Untuk AI Agent & Developer)

Dokumen ini ditujukan bagi AI Agent atau Developer yang akan memelihara, membaca, atau melanjutkan pengembangan proyek **Server Monitoring Dashboard**. Proyek ini adalah aplikasi *single-binary* berbasis Rust yang menyajikan dashboard *monitoring* server murni lewat HTTP Polling (tanpa WebSocket).

## 1. Arsitektur Utama
- **Backend Framework**: `axum` (dibangun di atas `tokio` dan `hyper`).
- **Database**: SQLite via `sqlx` (asynchronous, pure Rust).
- **Frontend**: HTML5, Vanilla JavaScript (ES6+), dan CSS3. Disajikan statis melalui `tower_http::services::ServeDir`.
- **Paradigma Komunikasi**: Frontend menggunakan `fetch()` API dengan interval `setInterval()` untuk melakukan *polling* data ke endpoint `/api/*`. Tidak ada WebSocket/SSE demi kesederhanaan *deployment* dan efisiensi memori.

## 2. Struktur Direktori dan File
```text
.
├── Cargo.toml          # Dependensi Rust (axum, tokio, sqlx, sysinfo, dll)
├── .env                # Konfigurasi environment (PORT, FILE_ROOT, DB_PATH)
├── src/
│   ├── main.rs         # Entry point, inisialisasi state, dan definisi Axum Router
│   ├── routes/         # Layer HTTP Handlers (Menerima request, membalas JSON)
│   │   ├── system.rs   # Handler /api/system
│   │   ├── network.rs  # Handler /api/network
│   │   ├── ports.rs    # Handler /api/ports & /api/ports/scan
│   │   ├── files.rs    # Handler /api/files/*
│   │   ├── speedtest.rs# Handler /api/speedtest/*
│   │   └── podman*.rs  # Handler /api/podman/*
│   ├── services/       # Layer Bisnis Logik & Interaksi OS / Subprocess
│   │   ├── system_info.rs # Wrapper crate `sysinfo`
│   │   ├── port_scanner.rs# Parsing `ss -tulnp`
│   │   ├── nmap_scanner.rs# Spawn subprocess `nmap`
│   │   ├── podman_cli.rs  # Spawn subprocess `podman`
│   │   ├── speedtest_cli.rs# Spawn subprocess `speedtest-cli`
│   │   └── file_manager.rs# Navigasi filesystem ter-sandbox
│   ├── background/
│   │   └── scheduler.rs# Task tokio::spawn interval (misal: speedtest per jam)
│   ├── db/
│   │   ├── migrations.sql # Skema SQLite awal
│   │   └── mod.rs      # Koneksi pool Sqlx
│   └── auth/
│       └── middleware.rs  # (Disiapkan) Basic Auth middleware (dimatikan di root)
└── static/             # Frontend statis
    ├── index.html      # UI Dashboard utama
    ├── css/style.css
    └── js/             # Skrip JS terpisah per modul (system.js, ports.js, dll)
```

## 3. Catatan Keamanan Penting (Security Context)

Saat mengembangkan lebih lanjut, aturan berikut **WAJIB** dipatuhi:

1. **Subprocess (Command Injection Prevention)**:
   - JANGAN PERNAH menggunakan `sh -c` yang digabung (*concatenate*) dengan string input dari user.
   - Gunakan `std::process::Command` dengan meneruskan parameter lewat pemanggilan `.args()` secara terpisah. Contoh yang benar ada di `src/services/podman_cli.rs` (validasi regex/alfanumerik) dan `src/routes/podman_create.rs`.
2. **File Explorer (Path Traversal Protection)**:
   - Modul `file_manager.rs` menggunakan `canonicalize()` dan mengecek apakah target path berawal dari `FILE_ROOT` yang dikonfigurasi di `.env`. 
   - Jangan pernah by-pass fungsi `resolve_and_validate_path()` saat membaca atau memanipulasi file agar user tidak bisa membaca `/etc/passwd` atau root direktori.
3. **Middleware Authentication**:
   - Terdapat benturan *type trait* pada `axum::Router` jika `axum::middleware::from_fn` diimplementasikan secara global (menggunakan `.layer()` atau `.route_layer()`) yang mencakup rute `ServeDir` fallback. Untuk mengaktifkan kembali Basic Auth di masa depan, pertimbangkan untuk membungkus keseluruhan `app` menggunakan `tower::ServiceBuilder` yang kompatibel, atau implementasikan *auth middleware* per-route/grup.

## 4. Cara Kerja Fitur Spesifik

- **Podman**: Menggunakan perintah `podman ps -a --format json` untuk membaca kontainer. Ini mengharuskan host OS menginstal Podman.
- **Port Scanner**:
  - *Quick Scan*: Mengambil data secara pasif dari output `ss -tulnp`.
  - *Deep Scan (Nmap)*: Men-spawn background task `nmap -p 1-1000 -T4 <target>`. Endpoint mereturn ID Job (`uuid` diganti dengan Auto-increment SQLite ID `job_id`), lalu frontend melakukan *polling* setiap 2 detik ke `/api/ports/scan/{id}` untuk menunggu status `done`.
- **Speedtest**: Menggunakan `speedtest-cli --json`. Secara default dijalankan 1 jam sekali oleh `tokio::time::interval` di `src/background/scheduler.rs`.

## 5. Pengembangan Lanjutan (TODO)
- Pemasangan Caddy/Nginx reverse proxy untuk menyediakan akses HTTPS (TLS).
- Menghidupkan Auth Middleware yang solid.
- Penambahan fungsi *File Upload* dan *Delete* (Backend routing sudah disiapkan ruangnya, tapi fungsinya belum ditambahkan demi menjaga keamanan di fase prototipe).
