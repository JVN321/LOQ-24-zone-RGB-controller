#!/usr/bin/env bash
# install.sh — Automated Installer for Lenovo LOQ 24-Zone RGB Controller (Linux)
set -e

# ANSI Styling
BOLD='\033[1m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}${BOLD}🎹 Lenovo LOQ RGB Controller — Automated Installer${NC}"
echo -e "--------------------------------------------------------"

# 1. Detect Package Manager and Install System Dependencies
echo -e "${BLUE}1/5 Detecting Linux distribution and installing system packages...${NC}"

if command -v pacman >/dev/null 2>&1; then
    echo -e "${GREEN}Detected Arch Linux (pacman). Installing dependencies...${NC}"
    sudo pacman -S --needed --noconfirm \
        alsa-lib libx11 libxi libxrandr hidapi pkgconf base-devel clang wayland libxkbcommon mesa pipewire dbus grim
elif command -v dnf >/dev/null 2>&1; then
    echo -e "${GREEN}Detected Fedora / RHEL (dnf). Installing dependencies...${NC}"
    sudo dnf install -y \
        alsa-lib-devel libX11-devel libXi-devel libXrandr-devel hidapi-devel pkgconf-pkg-config gcc wayland-devel libxkbcommon-devel mesa-libEGL-devel mesa-libgbm-devel pipewire-devel dbus-devel clang grim
elif command -v apt-get >/dev/null 2>&1; then
    echo -e "${GREEN}Detected Debian / Ubuntu / Mint (apt). Installing dependencies...${NC}"
    sudo apt-get update
    sudo apt-get install -y \
        libasound2-dev libx11-dev libxi-dev libxrandr-dev libhidapi-dev pkg-config build-essential libwayland-dev libxkbcommon-dev libegl1-mesa-dev libgbm-dev libpipewire-0.3-dev libdbus-1-dev clang grim
else
    echo -e "${YELLOW}Warning: Distro package manager not recognized. Please ensure build dependencies are installed.${NC}"
fi

# 2. Check Rust & Cargo
if ! command -v cargo >/dev/null 2>&1; then
    echo -e "${YELLOW}Cargo toolchain not found. Installing Rust...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
fi
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env" 2>/dev/null || true
fi
export PATH="$HOME/.cargo/bin:$PATH"

# 3. Hardware Permissions (udev & groups)
echo -e "${BLUE}2/5 Setting up udev rules and user groups...${NC}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ -f "$SCRIPT_DIR/99-loq-rgb.rules" ]; then
    sudo cp "$SCRIPT_DIR/99-loq-rgb.rules" /etc/udev/rules.d/
    sudo udevadm control --reload-rules
    sudo udevadm trigger
    sudo usermod -aG input "$USER" 2>/dev/null || true
    sudo usermod -aG plugdev "$USER" 2>/dev/null || true
    echo -e "${GREEN}✓ udev rules installed and user added to input/plugdev groups.${NC}"
else
    echo -e "${YELLOW}Warning: 99-loq-rgb.rules file not found.${NC}"
fi

# 4. Build Project
echo -e "${BLUE}3/5 Compiling Rust workspace...${NC}"
cd "$SCRIPT_DIR"
cargo build --release
echo -e "${GREEN}✓ Build completed successfully.${NC}"

# 5. Disable Autonomous Mode
echo -e "${BLUE}4/5 Disabling autonomous mode on RGB controller...${NC}"
if cargo run --release --manifest-path "$SCRIPT_DIR/rust-backend/Cargo.toml" --example disable_autonomous; then
    echo -e "${GREEN}✓ Keyboard host control verified.${NC}"
else
    echo -e "${YELLOW}Warning: Autonomous mode disabler returned non-zero code. Proceeding with service setup...${NC}"
fi

# 6. Install & Setup systemd Service
echo -e "${BLUE}5/5 Deploying binary and configuring systemd user service...${NC}"
mkdir -p "$HOME/.local/bin"
cp "$SCRIPT_DIR/target/release/rgb-server" "$HOME/.local/bin/rgb-server"
chmod +x "$HOME/.local/bin/rgb-server"

if [ -f "$SCRIPT_DIR/rgb-server-wrapper.sh" ]; then
    cp "$SCRIPT_DIR/rgb-server-wrapper.sh" "$HOME/.local/bin/rgb-server-wrapper.sh"
    chmod +x "$HOME/.local/bin/rgb-server-wrapper.sh"
    echo -e "${GREEN}✓ Copied rgb-server-wrapper.sh to ~/.local/bin/${NC}"
else
    echo -e "${YELLOW}Warning: rgb-server-wrapper.sh not found.${NC}"
fi

mkdir -p "$HOME/.config/systemd/user"
if [ -f "$SCRIPT_DIR/rgb-controller.service" ]; then
    cp "$SCRIPT_DIR/rgb-controller.service" "$HOME/.config/systemd/user/rgb-controller.service"
    systemctl --user daemon-reload
    systemctl --user enable rgb-controller.service
    systemctl --user restart rgb-controller.service
    echo -e "${GREEN}✓ systemd user service enabled and started.${NC}"
fi

echo -e "--------------------------------------------------------"
echo -e "${GREEN}${BOLD}✨ Installation Complete!${NC}"
echo -e "Open Web UI:   ${GREEN}${BOLD}http://127.0.0.1:7070${NC}"
echo -e "Service Status:"
systemctl --user status rgb-controller.service --no-pager || true
echo -e "--------------------------------------------------------"
