#!/usr/bin/env bash
# package-release.sh — Bundle prebuilt release files into a universal .tar.gz archive
set -e

BOLD='\033[1m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

ARCHIVE_NAME="loq-rgb-controller-linux-x86_64"
DIST_DIR="$SCRIPT_DIR/dist"
BUNDLE_DIR="$DIST_DIR/$ARCHIVE_NAME"

echo -e "${BLUE}${BOLD}📦 Packaging LOQ RGB Controller Universal Release (${ARCHIVE_NAME})...${NC}"

# 1. Compile release binary if needed
if [ ! -f "$SCRIPT_DIR/target/release/rgb-server" ]; then
    echo -e "${BLUE}Compiling release binary...${NC}"
    cargo build --release
fi

# 2. Prepare clean staging directory
rm -rf "$DIST_DIR"
mkdir -p "$BUNDLE_DIR"

# 3. Copy release files
cp "$SCRIPT_DIR/target/release/rgb-server" "$BUNDLE_DIR/rgb-server"
cp "$SCRIPT_DIR/rgb-server-wrapper.sh" "$BUNDLE_DIR/rgb-server-wrapper.sh"
cp "$SCRIPT_DIR/rgb-controller.service" "$BUNDLE_DIR/rgb-controller.service"
cp "$SCRIPT_DIR/99-loq-rgb.rules" "$BUNDLE_DIR/99-loq-rgb.rules"
cp "$SCRIPT_DIR/install.sh" "$BUNDLE_DIR/install.sh"
cp "$SCRIPT_DIR/deploy.sh" "$BUNDLE_DIR/deploy.sh"
cp "$SCRIPT_DIR/LICENSE" "$BUNDLE_DIR/LICENSE"
cp "$SCRIPT_DIR/README.md" "$BUNDLE_DIR/README.md"

# 4. Set executable permissions
chmod +x "$BUNDLE_DIR/rgb-server"
chmod +x "$BUNDLE_DIR/rgb-server-wrapper.sh"
chmod +x "$BUNDLE_DIR/install.sh"
chmod +x "$BUNDLE_DIR/deploy.sh"

# 5. Create .tar.gz archive
echo -e "${BLUE}Compressing tarball...${NC}"
tar -czf "$SCRIPT_DIR/${ARCHIVE_NAME}.tar.gz" -C "$DIST_DIR" "$ARCHIVE_NAME"

# 6. Generate SHA256 Checksum
cd "$SCRIPT_DIR"
sha256sum "${ARCHIVE_NAME}.tar.gz" > "${ARCHIVE_NAME}.tar.gz.sha256"

echo -e "${GREEN}${BOLD}✓ Release bundle created:${NC}"
echo -e "  Archive:  ${BOLD}${ARCHIVE_NAME}.tar.gz${NC} ($(du -h "${ARCHIVE_NAME}.tar.gz" | cut -f1))"
echo -e "  Checksum: ${BOLD}${ARCHIVE_NAME}.tar.gz.sha256${NC}"
