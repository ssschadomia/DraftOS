#!/usr/bin/env bash
#
# Build the DraftOS pacman repository: compiles the first-party apps, assembles
# the draftos-branding / draftos-apps / draftos-core packages, and runs
# `repo-add` to produce a signed (or unsigned) [draftos] repo under
# ./out/repo/x86_64/ — ready to `rclone sync` to Cloudflare R2.
#
# Runs inside the Arch builder container (reused from the ISO build) so it has
# makepkg + a synced pacman DB, regardless of the Fedora host.
#
# Usage:
#   ./build-repo.sh                     # unsigned (local testing)
#   DRAFTOS_GPG_KEY=/path/secret.asc \
#   DRAFTOS_GPG_KEYID=ABCD... ./build-repo.sh   # signed (CI)
#
# Output: ./out/repo/x86_64/{draftos.db,*.pkg.tar.zst[,*.sig]}
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$HERE/.." && pwd)"
BUILDER_IMAGE="draftos-iso-builder"

GPG_KEY="${DRAFTOS_GPG_KEY:-}"
GPG_KEYID="${DRAFTOS_GPG_KEYID:-}"

echo ">> Building the DraftOS repo inside the Arch builder container..."
podman run --rm \
    --security-opt label=disable \
    -e "GPG_KEYID=${GPG_KEYID}" \
    -e "HAVE_KEY=$([ -n "$GPG_KEY" ] && echo 1 || echo 0)" \
    ${GPG_KEY:+-v "$GPG_KEY:/tmp/draftos-signing.asc:ro"} \
    -v "$REPO_ROOT:/src" \
    -w /src/packaging \
    "$BUILDER_IMAGE" \
    bash -euo pipefail -c '
        echo "== ensure build tooling =="
        pacman -Sy --needed --noconfirm base-devel git >/dev/null

        # makepkg refuses to run as root; work under an unprivileged user.
        useradd -m -s /bin/bash builder 2>/dev/null || true

        echo "== compile first-party apps (release) =="
        # Keep the host workspace target/ clean: build into a container-local dir.
        export CARGO_TARGET_DIR=/tmp/draftos-target
        cargo build --release \
            -p draftos-welcome -p draftos-store -p draftos-media-writer

        echo "== stage draftos-apps payload =="
        P=/src/packaging/draftos-apps/payload
        rm -rf "$P"; mkdir -p "$P/bin" "$P/desktop" "$P/icons"
        cp "$CARGO_TARGET_DIR"/release/draftos-welcome \
           "$CARGO_TARGET_DIR"/release/draftos-store \
           "$CARGO_TARGET_DIR"/release/draftos-media-writer "$P/bin/"
        # Ship every app EXCEPT the installer (that belongs to the live ISO).
        for d in /src/apps/*/data/*.desktop; do cp "$d" "$P/desktop/"; done
        for i in /src/apps/*/data/*.svg;     do cp "$i" "$P/icons/";   done
        rm -f "$P/desktop/org.draftos.Installer.desktop" \
              "$P/icons/org.draftos.Installer.svg"

        echo "== import signing key (if provided) =="
        SIGN=""
        if [ "${HAVE_KEY:-0}" = "1" ]; then
            su builder -c "gpg --batch --import /tmp/draftos-signing.asc"
            SIGN="--sign --key ${GPG_KEYID}"
        fi

        OUT=/src/packaging/out
        REPO="$OUT/repo/x86_64"
        rm -rf "$OUT"; mkdir -p "$REPO"
        chown -R builder /src/packaging

        echo "== build packages =="
        for pkg in draftos-branding draftos-apps draftos-core; do
            su builder -c "cd /src/packaging/$pkg && makepkg -dfc --noconfirm --nodeps ${SIGN}"
            cp /src/packaging/$pkg/*.pkg.tar.zst "$REPO/"
            [ -n "$SIGN" ] && cp /src/packaging/$pkg/*.pkg.tar.zst.sig "$REPO/" || true
        done

        echo "== assemble repo db =="
        cd "$REPO"
        su builder -c "cd $REPO && repo-add ${SIGN:+--sign --key ${GPG_KEYID}} draftos.db.tar.zst *.pkg.tar.zst"

        # Clean the staged payload so it never lands in git.
        rm -rf /src/packaging/draftos-apps/payload
        chown -R "$(stat -c %u /src/packaging):$(stat -c %g /src/packaging)" /src/packaging/out
    '

echo
echo ">> Done. Repo at packaging/out/repo/x86_64/"
ls -lh "$HERE/out/repo/x86_64/" 2>/dev/null || true
