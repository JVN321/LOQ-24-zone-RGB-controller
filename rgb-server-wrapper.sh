#!/usr/bin/env bash
# rgb-server-wrapper.sh — Environment wrapper script for rgb-server under systemd user service

set -e

# Export standard user paths and environment
export PATH="$HOME/.local/bin:/usr/local/bin:/usr/bin:$PATH"

# Auto-detect XDG_RUNTIME_DIR if missing
if [ -z "$XDG_RUNTIME_DIR" ]; then
    export XDG_RUNTIME_DIR="/run/user/$(id -u 2>/dev/null || echo 1000)"
fi

# Auto-detect WAYLAND_DISPLAY if missing
if [ -z "$WAYLAND_DISPLAY" ]; then
    # Find active Wayland domain socket (filter out .lock files and non-sockets)
    for sock in $(ls -t "$XDG_RUNTIME_DIR"/wayland-* 2>/dev/null); do
        if [ -S "$sock" ] && [[ "$sock" != *.lock ]]; then
            export WAYLAND_DISPLAY="$(basename "$sock")"
            break
        fi
    done
fi

# If WAYLAND_DISPLAY is still not set, check systemd environment
if [ -z "$WAYLAND_DISPLAY" ] && command -v systemctl >/dev/null 2>&1; then
    SYS_WAYLAND=$(systemctl --user show-environment 2>/dev/null | grep "^WAYLAND_DISPLAY=" | cut -d= -f2-)
    if [ -n "$SYS_WAYLAND" ]; then
        export WAYLAND_DISPLAY="$SYS_WAYLAND"
    fi
fi

# Execute rgb-server
exec "$HOME/.local/bin/rgb-server" "$@"
