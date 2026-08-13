#!/usr/bin/env bash
# deploy.sh — Deploy LOQ RGB Controller binary, wrapper, and systemd service
# Stops existing service, compiles workspace, copies files, and re-enables startup service.

set -e

# Terminal styling
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color
BOLD='\033[1m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo -e "${BLUE}${BOLD}🎹 Lenovo LOQ RGB Controller — Deployment & Service Updater${NC}"
echo -e "--------------------------------------------------------"

# 1. Stop existing systemd user service and running processes
echo -e "${YELLOW}1/6 Stopping active rgb-controller.service and running processes...${NC}"
if systemctl --user is-active --quiet rgb-controller.service 2>/dev/null; then
    systemctl --user stop rgb-controller.service || true
    echo -e "${GREEN}✓ Active systemd service stopped.${NC}"
fi

if pgrep -x rgb-server >/dev/null || pgrep -f "target/release/rgb-server" >/dev/null; then
    pkill -f rgb-server || true
    sleep 1
    echo -e "${GREEN}✓ Terminated running rgb-server processes.${NC}"
fi

# 2. Build release binaries
echo -e "${BLUE}2/6 Compiling Rust workspace (release mode)...${NC}"
cargo build --release
echo -e "${GREEN}✓ Compilation successful.${NC}"

# 3. Create target directories
echo -e "${BLUE}3/6 Setting up target directories...${NC}"
mkdir -p "$HOME/.local/bin"
mkdir -p "$HOME/.config/systemd/user"

# 4. Copy binaries and wrapper script
echo -e "${BLUE}4/6 Installing server binary and environment wrapper...${NC}"
cp "$SCRIPT_DIR/target/release/rgb-server" "$HOME/.local/bin/rgb-server"
chmod +x "$HOME/.local/bin/rgb-server"

if [ -f "$SCRIPT_DIR/rgb-server-wrapper.sh" ]; then
    cp "$SCRIPT_DIR/rgb-server-wrapper.sh" "$HOME/.local/bin/rgb-server-wrapper.sh"
    chmod +x "$HOME/.local/bin/rgb-server-wrapper.sh"
    echo -e "${GREEN}✓ Copied rgb-server and rgb-server-wrapper.sh to ~/.local/bin/${NC}"
else
    echo -e "${RED}Warning: rgb-server-wrapper.sh not found in repository root.${NC}"
fi

if [ -f "$SCRIPT_DIR/rgb-controller.service" ]; then
    cp "$SCRIPT_DIR/rgb-controller.service" "$HOME/.config/systemd/user/rgb-controller.service"
    echo -e "${GREEN}✓ Copied rgb-controller.service to ~/.config/systemd/user/${NC}"
fi

# 5. Disable autonomous mode & reset hardware state if needed
echo -e "${BLUE}5/6 Clearing hardware firmware state and disabling autonomous mode...${NC}"
if cargo run --release --manifest-path "$SCRIPT_DIR/rust-backend/Cargo.toml" --example disable_autonomous; then
    echo -e "${GREEN}✓ Keyboard host control verified.${NC}"
else
    echo -e "${YELLOW}Warning: Autonomous mode disabler returned non-zero code. Proceeding with service launch...${NC}"
fi

# 6. Reload systemd, enable, and start service
echo -e "${BLUE}6/6 Reloading systemd user daemon and starting service...${NC}"
systemctl --user daemon-reload
systemctl --user enable rgb-controller.service
systemctl --user start rgb-controller.service

echo -e "--------------------------------------------------------"
echo -e "${GREEN}${BOLD}✨ Deployment & Service Update Complete!${NC}"
echo -e "Web UI: ${GREEN}${BOLD}http://127.0.0.1:7070${NC}"
echo -e "Service Status:"
systemctl --user status rgb-controller.service --no-pager || true
echo -e "--------------------------------------------------------"
