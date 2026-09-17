#!/bin/bash

# install.sh — InfoIn Server Backend Installer
# Kompilasi Rust backend dan pasang sebagai systemd user service.
#
# Usage:
#   ./install.sh           → install + enable + start
#   ./install.sh --update  → build ulang binary + restart service (tanpa ubah .env)
#   ./install.sh --remove  → stop + disable + hapus service

set -e

APP_NAME="infoinserver"
APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY_PATH="$APP_DIR/target/release/$APP_NAME"
ENV_FILE="$APP_DIR/.env"
SERVICE_NAME="infoinserver-backend"
SERVICE_FILE="$HOME/.config/systemd/user/${SERVICE_NAME}.service"

# ── Warna output ──────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

info()    { echo -e "${CYAN}[•]${NC} $*"; }
success() { echo -e "${GREEN}[✓]${NC} $*"; }
warn()    { echo -e "${YELLOW}[!]${NC} $*"; }
error()   { echo -e "${RED}[✗]${NC} $*"; }
header()  { echo -e "\n${BOLD}${CYAN}══ $* ══${NC}"; }

# ── Parse argumen ─────────────────────────────────────────────────────────────
MODE="install"
case "${1:-}" in
    --update) MODE="update" ;;
    --remove) MODE="remove" ;;
    "")       MODE="install" ;;
    *)
        echo "Usage: $0 [--update | --remove]"
        echo "  (tanpa flag)  → install lengkap: compile + .env + systemd"
        echo "  --update      → build ulang binary + restart service"
        echo "  --remove      → stop + disable + hapus service files"
        exit 1
        ;;
esac

# ═════════════════════════════════════════════════════════════════════════════
# MODE: REMOVE
# ═════════════════════════════════════════════════════════════════════════════
if [ "$MODE" = "remove" ]; then
    header "Menghapus InfoIn Backend Service"

    if systemctl --user is-active --quiet "$SERVICE_NAME" 2>/dev/null; then
        info "Menghentikan service..."
        systemctl --user stop "$SERVICE_NAME"
        success "Service dihentikan."
    fi

    if systemctl --user is-enabled --quiet "$SERVICE_NAME" 2>/dev/null; then
        info "Menonaktifkan service..."
        systemctl --user disable "$SERVICE_NAME"
    fi

    if [ -f "$SERVICE_FILE" ]; then
        rm "$SERVICE_FILE"
        systemctl --user daemon-reload
        success "Service file dihapus: $SERVICE_FILE"
    else
        warn "Service file tidak ditemukan, sudah terhapus?"
    fi

    success "Selesai. Binary dan .env tidak dihapus."
    exit 0
fi

# ═════════════════════════════════════════════════════════════════════════════
# LANGKAH 1 — Cek Rust/Cargo
# ═════════════════════════════════════════════════════════════════════════════
header "1. Memeriksa Rust toolchain"

if ! command -v cargo &>/dev/null; then
    if [ -f "$HOME/.cargo/env" ]; then
        # shellcheck source=/dev/null
        source "$HOME/.cargo/env"
    fi
fi

if ! command -v cargo &>/dev/null; then
    error "Rust/Cargo tidak ditemukan."
    echo ""
    echo "  Install Rust terlebih dahulu:"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo "  source \$HOME/.cargo/env"
    exit 1
fi

RUST_VER=$(rustc --version)
CARGO_VER=$(cargo --version)
success "Rust ditemukan: $RUST_VER"
success "Cargo: $CARGO_VER"

# ── Cek build dependencies (PAM, clang) ──────────────────────────────────────
if pkg-config --exists libpam 2>/dev/null || ldconfig -p 2>/dev/null | grep -q libpam; then
    success "libpam ditemukan."
else
    warn "libpam tidak terdeteksi via pkg-config."
    echo ""
    echo "  Jika build gagal, install dulu:"
    echo "  Ubuntu/Debian : sudo apt install build-essential pkg-config libclang-dev libpam0g-dev"
    echo "  Arch/CachyOS  : sudo pacman -S base-devel clang pam"
    echo ""
fi

# ═════════════════════════════════════════════════════════════════════════════
# LANGKAH 2 — Compile
# ═════════════════════════════════════════════════════════════════════════════
header "2. Kompilasi backend (release mode)"

cd "$APP_DIR"
info "Menjalankan: cargo build --release"
echo ""

if cargo build --release; then
    echo ""
    success "Build berhasil! Binary: $BINARY_PATH"
else
    echo ""
    error "Build gagal. Periksa error di atas."
    echo ""
    echo "  Pastikan dependencies sudah terinstall:"
    echo "  Ubuntu/Debian : sudo apt install build-essential pkg-config libclang-dev libpam0g-dev"
    echo "  Arch/CachyOS  : sudo pacman -S base-devel clang pam"
    exit 1
fi

# ── Jika mode --update, langsung restart dan selesai ─────────────────────────
if [ "$MODE" = "update" ]; then
    header "Update — Restart service"

    if systemctl --user is-active --quiet "$SERVICE_NAME" 2>/dev/null; then
        info "Merestart $SERVICE_NAME..."
        systemctl --user restart "$SERVICE_NAME"
        sleep 2
        if systemctl --user is-active --quiet "$SERVICE_NAME"; then
            success "Service berhasil direstart."
            echo ""
            systemctl --user status "$SERVICE_NAME" --no-pager -l
        else
            error "Service gagal start setelah restart."
            journalctl --user -u "$SERVICE_NAME" --no-pager -n 20
            exit 1
        fi
    else
        warn "Service belum berjalan. Menjalankan sekarang..."
        systemctl --user start "$SERVICE_NAME"
        success "Service dijalankan."
    fi

    echo ""
    success "Update selesai!"
    exit 0
fi

# ═════════════════════════════════════════════════════════════════════════════
# LANGKAH 3 — Setup .env
# ═════════════════════════════════════════════════════════════════════════════
header "3. Konfigurasi .env"

if [ -f "$ENV_FILE" ]; then
    success ".env sudah ada, dilewati. Edit manual jika perlu: $ENV_FILE"
else
    info "Membuat .env dari .env.example..."
    cp "$APP_DIR/.env.example" "$ENV_FILE"

    # Detect IP lokal
    LOCAL_IP=$(hostname -I 2>/dev/null | awk '{print $1}')
    if [ -z "$LOCAL_IP" ]; then
        LOCAL_IP="127.0.0.1"
    fi

    # Generate random JWT secret
    JWT_SECRET=$(tr -dc 'A-Za-z0-9!@#$%^&*' </dev/urandom 2>/dev/null | head -c 48 || \
                 cat /proc/sys/kernel/random/uuid 2>/dev/null | tr -d '-' || \
                 date +%s%N | sha256sum | cut -c1-48)

    # Ganti placeholder di .env
    sed -i "s|YOUR_IP|${LOCAL_IP}|g" "$ENV_FILE"
    sed -i "s|your-strong-random-secret-here|${JWT_SECRET}|g" "$ENV_FILE"

    success ".env dibuat: $ENV_FILE"
    echo ""
    warn "JWT_SECRET di-generate otomatis. Pastikan nilai ini SAMA di frontend-vue/.env"
    echo ""
    echo "  JWT_SECRET yang di-set: $(grep JWT_SECRET "$ENV_FILE" | cut -d= -f2)"
    echo ""
    warn "Periksa dan sesuaikan CORS_ORIGIN di .env jika IP kamu beda:"
    echo "  IP terdeteksi: $LOCAL_IP"
    echo "  File: $ENV_FILE"
fi

# ═════════════════════════════════════════════════════════════════════════════
# LANGKAH 4 — Buat systemd user service
# ═════════════════════════════════════════════════════════════════════════════
header "4. Membuat systemd user service"

mkdir -p "$HOME/.config/systemd/user"

cat > "$SERVICE_FILE" <<EOF
[Unit]
Description=InfoIn Server — Rust Backend (Axum)
After=network.target
Wants=network.target

[Service]
Type=simple
WorkingDirectory=${APP_DIR}
ExecStart=${BINARY_PATH}
EnvironmentFile=${ENV_FILE}
Restart=on-failure
RestartSec=5

StandardOutput=journal
StandardError=journal
SyslogIdentifier=${SERVICE_NAME}

[Install]
WantedBy=default.target
EOF

success "Service file dibuat: $SERVICE_FILE"

# ═════════════════════════════════════════════════════════════════════════════
# LANGKAH 5 — Enable lingering (auto-start tanpa login)
# ═════════════════════════════════════════════════════════════════════════════
header "5. Mengaktifkan user linger"

if loginctl show-user "$USER" 2>/dev/null | grep -q "Linger=yes"; then
    success "Linger sudah aktif untuk user $USER."
else
    info "Mengaktifkan linger untuk user $USER (agar service start saat boot tanpa login)..."
    if loginctl enable-linger "$USER" 2>/dev/null; then
        success "Linger aktif."
    else
        warn "Tidak bisa aktifkan linger (mungkin perlu sudo)."
        echo "  Jalankan manual: sudo loginctl enable-linger $USER"
    fi
fi

# ═════════════════════════════════════════════════════════════════════════════
# LANGKAH 6 — Enable + Start service
# ═════════════════════════════════════════════════════════════════════════════
header "6. Mengaktifkan dan menjalankan service"

systemctl --user daemon-reload

# Matikan proses lama jika masih ada (dari start.sh misalnya)
OLD_PID=$(pgrep -f "$BINARY_PATH" 2>/dev/null || true)
if [ -n "$OLD_PID" ]; then
    warn "Ditemukan proses backend lama (PID: $OLD_PID), menghentikan..."
    kill "$OLD_PID" 2>/dev/null || true
    sleep 2
fi

systemctl --user enable "$SERVICE_NAME"
success "Service di-enable (akan auto-start saat boot)."

systemctl --user start "$SERVICE_NAME"
info "Menunggu service siap..."
sleep 3

if systemctl --user is-active --quiet "$SERVICE_NAME"; then
    success "Service berjalan!"
else
    error "Service gagal start. Log:"
    journalctl --user -u "$SERVICE_NAME" --no-pager -n 30
    exit 1
fi

# ═════════════════════════════════════════════════════════════════════════════
# SELESAI
# ═════════════════════════════════════════════════════════════════════════════
header "Instalasi Selesai"

echo ""
systemctl --user status "$SERVICE_NAME" --no-pager -l
echo ""
echo -e "${BOLD}Perintah berguna:${NC}"
echo ""
echo "  # Status & log"
echo "  systemctl --user status $SERVICE_NAME"
echo "  journalctl --user -u $SERVICE_NAME -f"
echo ""
echo "  # Start / stop / restart"
echo "  systemctl --user start $SERVICE_NAME"
echo "  systemctl --user stop $SERVICE_NAME"
echo "  systemctl --user restart $SERVICE_NAME"
echo ""
echo "  # Update kode (build ulang + restart otomatis)"
echo "  ./install.sh --update"
echo ""
echo "  # Hapus service"
echo "  ./install.sh --remove"
echo ""
success "Backend berjalan di http://localhost:$(grep '^PORT' "$ENV_FILE" | cut -d= -f2 || echo 8080)"
