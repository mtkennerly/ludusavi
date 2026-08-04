#!/usr/bin/env bash
# Wrapper for `pnpm tauri dev` that works around a WebKitGTK/Wayland bug:
# some compositor/GPU driver combos crash WebKitGTK's DMA-BUF renderer with
# "Error 71 (Protocol error) dispatching to Wayland display".
#
# Usage: ./scripts/dev.sh [args passed through to `tauri dev`]

set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

export WEBKIT_DISABLE_DMABUF_RENDERER=1

if pnpm tauri dev "$@"; then
    exit 0
fi

status=$?
if [ "${GDK_BACKEND:-}" = "x11" ]; then
    # Already tried the X11 fallback (or user forced it) - nothing more to retry.
    exit "$status"
fi

echo "tauri dev failed (exit $status); retrying with GDK_BACKEND=x11 (XWayland fallback)..." >&2
export GDK_BACKEND=x11
exec pnpm tauri dev "$@"
