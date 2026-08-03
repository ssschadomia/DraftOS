#!/usr/bin/env bash
#
# shot.sh — screenshot a DraftOS libcosmic app for development review.
#
# Runs the app under XWayland (winit's X11 backend) and grabs its window pixmap
# directly from the X server with ImageMagick `import`. This is the reliable
# capture path on a locked-down GNOME Wayland host: it needs no compositor
# screencopy (grim) and no portal/permission dialog, and it side-steps
# cosmic-comp's nested-EGL issues.
#
# Usage:
#   tools/shot.sh <wm_class> <out.png> -- <command...>
#
# Example:
#   tools/shot.sh draftos-welcome /tmp/welcome.png -- \
#       env DRAFTOS_WELCOME_PAGE=1 target/debug/draftos-welcome
#
# Requirements: xprop, ImageMagick (`import`), a running X server (XWayland is
# fine). Force X11 mode by unsetting WAYLAND_DISPLAY (done below).
set -u

WM_CLASS="${1:?usage: shot.sh <wm_class> <out.png> -- <command...>}"
OUT="${2:?usage: shot.sh <wm_class> <out.png> -- <command...>}"
shift 2
[ "${1:-}" = "--" ] && shift
[ "$#" -gt 0 ] || { echo "shot.sh: missing command after --" >&2; exit 2; }

export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export DISPLAY="${DISPLAY:-:0}"
unset WAYLAND_DISPLAY   # force winit's X11 backend so the window is X-capturable

find_win() {
    for id in $(xprop -root _NET_CLIENT_LIST 2>/dev/null | sed 's/.*# //; s/,//g'); do
        xprop -id "$id" WM_CLASS 2>/dev/null | grep -q "\"$WM_CLASS\"" && { echo "$id"; return; }
    done
}

"$@" >/tmp/draftos-shot-app.log 2>&1 &
APP=$!

win=""
for _ in $(seq 1 25); do
    win=$(find_win); [ -n "$win" ] && break
    kill -0 "$APP" 2>/dev/null || { echo "shot.sh: app exited early; see /tmp/draftos-shot-app.log" >&2; exit 1; }
    sleep 0.3
done
[ -z "$win" ] && { echo "shot.sh: no window with WM_CLASS=$WM_CLASS" >&2; kill "$APP" 2>/dev/null; exit 1; }

sleep 1.2                       # let the first frame settle
import -window "$win" "$OUT"; rc=$?
kill "$APP" 2>/dev/null; wait 2>/dev/null

[ $rc -eq 0 ] && [ -s "$OUT" ] && echo "shot.sh: $OUT ($win)" || { echo "shot.sh: capture failed" >&2; exit 1; }
