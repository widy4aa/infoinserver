#!/bin/bash

# stop.sh - Script untuk menghentikan Server Monitoring Dashboard

APP_NAME="infoinserver"
APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY_PATH="$APP_DIR/target/release/$APP_NAME"

echo "=== Menghentikan Server Monitoring Dashboard ==="

# Cari proses yang berjalan
PID=$(pgrep -f "$BINARY_PATH")

if [ -z "$PID" ]; then
    echo "Dashboard tidak sedang berjalan."
    exit 0
fi

echo "Ditemukan proses Dashboard dengan PID: $PID"
kill "$PID"

if [ $? -eq 0 ]; then
    echo "Dashboard berhasil dihentikan."
else
    echo "Gagal menghentikan Dashboard. Coba gunakan: kill -9 $PID"
fi
