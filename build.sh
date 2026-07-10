#!/usr/bin/env bash
# build.sh — Build the LOQ RGB Controller for Linux
set -e

echo "▶ Building Rust workspace (backend + server)..."
cargo build --release

echo ""
echo "✅ Build complete!"
echo ""
echo "Binary: ./target/release/rgb-server"
echo ""
echo "────────────────────────────────────────────────────────"
echo "IMPORTANT: First-time setup — install udev rule for HID:"
echo ""
echo "  sudo cp 99-loq-rgb.rules /etc/udev/rules.d/"
echo "  sudo udevadm control --reload-rules"
echo "  sudo udevadm trigger"
echo "  sudo usermod -aG plugdev \$USER"
echo "  # Then log out and back in"
echo ""
echo "Run the server:  ./target/release/rgb-server"
echo "Open the UI at:  http://127.0.0.1:7070"
echo "────────────────────────────────────────────────────────"
