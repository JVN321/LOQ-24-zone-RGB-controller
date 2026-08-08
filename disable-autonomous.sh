#!/usr/bin/env bash
# disable-autonomous.sh — Safely disable autonomous mode on the Lenovo LOQ RGB Controller

set -e

# Harmonious colors for terminal styling
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color
BOLD='\033[1m'

echo -e "${BLUE}${BOLD}🎹 Lenovo LOQ RGB Controller — Autonomous Mode Disabler${NC}"
echo -e "--------------------------------------------------------"

# Parse arguments
AUTO_START=false
for arg in "$@"; do
    case $arg in
        --start|-s)
            AUTO_START=true
            shift
            ;;
    esac
done

# 1. Stop the systemd user service if running
if systemctl --user is-active --quiet rgb-controller.service 2>/dev/null; then
    echo -e "${YELLOW}Stopping active rgb-controller.service to release device lock...${NC}"
    systemctl --user stop rgb-controller.service
    WAS_SERVICE_ACTIVE=true
else
    WAS_SERVICE_ACTIVE=false
fi

# 2. Terminate any manually running rgb-server instances
if pgrep -x rgb-server >/dev/null || pgrep -f "target/release/rgb-server" >/dev/null; then
    echo -e "${YELLOW}Terminating running rgb-server processes...${NC}"
    pkill -f rgb-server || true
    # Wait a moment for ports/sockets and device handles to release
    sleep 1.5
fi

# 3. Compile and run USB reset tool to clear any stuck firmware states
echo -e "${BLUE}Compiling and running the USB reset tool...${NC}"
cargo build --release --example usb_reset
if ./target/release/examples/usb_reset; then
    echo -e "${GREEN}USB device reset successfully.${NC}"
    # Wait for the USB device to re-enumerate
    sleep 1.5
else
    echo -e "${YELLOW}Warning: USB device reset failed. Proceeding anyway...${NC}"
fi

# 4. Build and execute the disable_autonomous Rust tool
echo -e "${BLUE}Compiling and running the disabler tool...${NC}"
if cargo run --manifest-path rust-backend/Cargo.toml --example disable_autonomous; then
    echo -e "${GREEN}${BOLD}Autonomous mode disabled successfully! Keyboard is now host-controlled.${NC}"
else
    echo -e "${RED}Error: Failed to disable autonomous mode.${NC}"
    # Re-enable service if it was running before
    if [ "$WAS_SERVICE_ACTIVE" = true ]; then
        echo -e "${YELLOW}Re-starting rgb-controller.service due to failure...${NC}"
        systemctl --user start rgb-controller.service
    fi
    exit 1
fi

# 4. Handle startup of the server/service
echo -e "--------------------------------------------------------"
if [ "$WAS_SERVICE_ACTIVE" = true ]; then
    echo -e "${GREEN}Restarting the rgb-controller systemd service...${NC}"
    systemctl --user start rgb-controller.service
    echo -e "${GREEN}Service started successfully!${NC}"
else
    # Check if we should automatically start or if stdin is a TTY
    if [ "$AUTO_START" = true ]; then
        echo -e "${GREEN}Starting rgb-server in the background...${NC}"
        ./target/release/rgb-server >/dev/null 2>&1 &
        echo -e "${GREEN}rgb-server started in the background (PID $!).${NC}"
    elif [ -t 0 ]; then
        read -p "Would you like to start the rgb-server now? (y/N) " -n 1 -r
        echo ""
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            echo -e "${GREEN}Starting rgb-server in the background...${NC}"
            # Ensure it is compiled
            cargo build --release --bin rgb-server
            ./target/release/rgb-server >/dev/null 2>&1 &
            echo -e "${GREEN}rgb-server started in the background (PID $!).${NC}"
        else
            echo -e "${YELLOW}You can start the server manually using: ./target/release/rgb-server${NC}"
        fi
    else
        echo -e "${YELLOW}You can start the server manually using: ./target/release/rgb-server${NC}"
    fi
fi
