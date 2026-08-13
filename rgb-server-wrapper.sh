#!/usr/bin/env bash
# rgb-server-wrapper.sh — Environment wrapper script for rgb-server under systemd user service

set -e

# Export standard user paths and environment
export PATH="$HOME/.local/bin:/usr/local/bin:/usr/bin:$PATH"

# Execute rgb-server
exec "$HOME/.local/bin/rgb-server" "$@"
