#!/usr/bin/env bash
#
# upload.sh — sync a local distribution tree to the DraftOS R2 bucket.
#
# Usage:
#   infra/cloudflare/upload.sh <local-dir> [remote-prefix]
#
# Examples:
#   infra/cloudflare/upload.sh dist/            # mirror the whole tree
#   infra/cloudflare/upload.sh dist/iso iso     # just the ISO directory
#
# Requires rclone with an [r2] remote (see rclone.conf.example). Override the
# bucket with DRAFTOS_R2_BUCKET; point at a specific config with RCLONE_CONFIG.
set -euo pipefail

SRC="${1:?usage: upload.sh <local-dir> [remote-prefix]}"
PREFIX="${2:-}"
BUCKET="${DRAFTOS_R2_BUCKET:-draftos-cdn}"
DEST="r2:${BUCKET}/${PREFIX}"

command -v rclone >/dev/null || { echo "rclone not found — install it first" >&2; exit 1; }
[ -d "$SRC" ] || { echo "source dir not found: $SRC" >&2; exit 1; }

# --- free-tier storage guard -------------------------------------------------
# R2's free tier is 10 GB of storage (egress is always free). Refuse to push if
# the resulting bucket would exceed a safe cap, so you never cross into billing.
CAP_GB="${DRAFTOS_R2_MAX_GB:-9}"
bytes_of() { rclone size "$1" --json 2>/dev/null | python3 -c 'import sys,json;print(json.load(sys.stdin).get("bytes",0))' 2>/dev/null || echo 0; }
SRC_B=$(du -sb "$SRC" | cut -f1)
TOTAL_B=$(bytes_of "r2:${BUCKET}")
OLD_B=$(bytes_of "$DEST")                 # this prefix is replaced by sync
PROJ_B=$(( TOTAL_B - OLD_B + SRC_B ))
CAP_B=$(( CAP_GB * 1000000000 ))
if [ "$PROJ_B" -gt "$CAP_B" ]; then
    echo "ABORT: projected bucket ~$((PROJ_B/1000000000)) GB would exceed the ${CAP_GB} GB free-tier guard." >&2
    echo "Prune old artifacts, or raise DRAFTOS_R2_MAX_GB (this may incur charges)." >&2
    exit 1
fi
echo ">> storage check: projected ~$((PROJ_B/1000000000)) GB / ${CAP_GB} GB cap — OK"

echo ">> syncing $SRC -> $DEST"
rclone sync "$SRC" "$DEST" \
    --progress \
    --s3-no-check-bucket \
    --transfers 8 \
    --checkers 16 \
    --s3-chunk-size 64M

echo ">> done"
