#!/bin/bash

# start.sh - Script untuk menjalankan Server Monitoring Dashboard
# Script ini akan melakukan build (jika perlu) dan menjalankan aplikasi di background.

APP_NAME="infoinserver"
APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY_PATH="$APP_DIR/target/release/$APP_NAME"

echo "=== Memulai Server Monitoring Dashboard ==="

# Cek apakah Cargo tersedia
if ! command -v cargo &> /dev/null; then
    # Coba load dari environment rustup
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    else
        echo "Error: 'cargo' tidak ditemukan. Pastikan Rust sudah terinstal."
        exit 1
    fi
fi

cd "$APP_DIR" || exit 1

# Lakukan release build
echo "Membangun aplikasi (Release mode)..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "Error: Build gagal. Silakan periksa log di atas."
    echo ""
    echo "Pastikan semua build dependencies sudah terinstall:"
    echo "  Ubuntu/Debian: sudo apt install build-essential pkg-config libclang-dev libpam0g-dev"
    echo "  Arch/CachyOS:  sudo pacman -S base-devel clang pam"
    exit 1
fi

echo "Build berhasil!"

# Cek apakah server sudah berjalan, dan matikan jika ya
PID=$(pgrep -f "$BINARY_PATH")
if [ -n "$PID" ]; then
    echo "Menghentikan instance server yang sudah berjalan (PID: $PID)..."
    kill "$PID"
    sleep 2 # Tunggu sebentar agar port dilepaskan
fi

# Jalankan server di background
echo "Menjalankan aplikasi..."
nohup "$BINARY_PATH" > "$APP_DIR/server.log" 2>&1 &
NEW_PID=$!

echo "Dashboard berhasil dijalankan di background (PID: $NEW_PID)."
echo "Log aplikasi dapat dilihat di: $APP_DIR/server.log"
echo "Untuk menghentikan, jalankan: kill $NEW_PID"

# Tampilkan beberapa baris awal dari log
sleep 1
echo ""
echo "--- Status Log Awal ---"
head -n 5 "$APP_DIR/server.log"
echo "-----------------------"
