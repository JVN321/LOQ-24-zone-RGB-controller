#!/usr/bin/env bash
# setup-fedora.sh — Install all system dependencies for the LOQ RGB Controller
set -e

echo "Installing system dependencies for Fedora..."
sudo dnf install -y \
    alsa-lib-devel \
    libX11-devel \
    libXi-devel \
    hidapi-devel \
    pkgconf-pkg-config \
    gcc \
    wayland-devel \
    libxkbcommon-devel \
    mesa-libEGL-devel \
    libXrandr-devel \
    pipewire-devel \
    dbus-devel \
    clang \
    mesa-libgbm-devel \
    grim

echo ""
echo "✅ System dependencies installed."
echo ""
echo "Now install the udev rule and add yourself to the 'input' group:"
echo ""
echo "  sudo cp 99-loq-rgb.rules /etc/udev/rules.d/"
echo "  sudo udevadm control --reload-rules"
echo "  sudo udevadm trigger"
echo "  sudo usermod -aG input \$USER"
echo "  # Log out and back in (or: newgrp input)"
echo ""
echo "Then build: ./build.sh"
echo "Then run:   ./target/release/rgb-server"
echo "Open UI:    http://127.0.0.1:7070"
