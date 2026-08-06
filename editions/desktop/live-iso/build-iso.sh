#!/usr/bin/env bash
#
# Build the DraftOS Desktop live/install ISO from a Fedora host, inside an Arch
# container running mkarchiso (archiso). The ISO boots into a live COSMIC
# desktop; "Install DraftOS" runs our libcosmic installer, which drives the
# draftos-install engine to install a normal Arch base onto disk.
#
# Usage:  sudo ./build-iso.sh
#
# Output: ./out/draftos-*.iso   (gitignored)
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$HERE/../../.." && pwd)"
BUILDER_IMAGE="draftos-iso-builder"
PROFILE="archiso"

cd "$HERE"

# --- Loop-device prep (mkarchiso mounts loopback images) ---
echo ">> Preparing loop devices on the host..."
modprobe loop 2>/dev/null || true
for i in $(seq 0 15); do [ -e "/dev/loop$i" ] || mknod -m660 "/dev/loop$i" b 7 "$i"; done
chown root:disk /dev/loop[0-9]* 2>/dev/null || true

echo ">> Building the ISO builder image (cached after first run)..."
podman build -t "$BUILDER_IMAGE" -f Containerfile.iso-builder .

# --- Compile the installer GUI + engine, staged into the live airootfs --------
# Built inside the Arch builder so they link against the same libraries the live
# COSMIC environment ships. cargo's target/ is cached under the repo root.
echo ">> Building draftos-installer + draftos-install (release)..."
podman run --rm \
    -v "$REPO_ROOT:/src:z" \
    -w /src \
    "$BUILDER_IMAGE" \
    cargo build --release -p draftos-installer -p draftos-install
install -Dm0755 "$REPO_ROOT/target/release/draftos-installer" "$PROFILE/airootfs/usr/bin/draftos-installer"
install -Dm0755 "$REPO_ROOT/target/release/draftos-install"   "$PROFILE/airootfs/usr/bin/draftos-install"
echo ">> Staged both binaries into the live ISO."

# Remove the staged (gitignored) binaries from the working tree afterwards.
cleanup() { rm -f "$PROFILE/airootfs/usr/bin/draftos-installer" "$PROFILE/airootfs/usr/bin/draftos-install"; }
trap cleanup EXIT

# Start from a clean work dir (a failed run leaves a root-owned partial tree).
rm -rf "$HERE/work"

echo ">> Running mkarchiso (releng base + DraftOS overrides)..."
# Assemble archiso's stock `releng` profile inside the container and overlay our
# archiso/ on top (profiledef, packages, pacman.conf, airootfs merge over it).
# --privileged: mkarchiso mounts loopback images and builds the squashfs.
podman run --rm --privileged \
    -v "$HERE:/iso:z" \
    -w /iso \
    "$BUILDER_IMAGE" \
    bash -euo pipefail -c '
        rm -rf /tmp/profile
        cp -a /usr/share/archiso/configs/releng /tmp/profile
        cp -a /iso/'"$PROFILE"'/. /tmp/profile/
        chmod +x /tmp/profile/profiledef.sh
        mkarchiso -v -w /iso/work -o /iso/out /tmp/profile
    '

echo
echo ">> Done. The ISO is in ./out/"
ls -lh "$HERE/out/"*.iso 2>/dev/null || true
echo ">> Test it in a VM (UEFI):"
echo "     qemu-system-x86_64 -enable-kvm -m 4096 -cpu host \\"
echo "         -bios /usr/share/edk2/ovmf/OVMF_CODE.fd \\"
echo "         -cdrom out/*.iso -drive file=draftos-test.qcow2,if=virtio,size=30G"
