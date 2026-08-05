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

echo ">> syncing $SRC -> $DEST"
rclone sync "$SRC" "$DEST" \
    --progress \
    --s3-no-check-bucket \
    --transfers 8 \
    --checkers 16 \
    --s3-chunk-size 64M

echo ">> done"
