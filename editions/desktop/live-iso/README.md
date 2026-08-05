# DraftOS Desktop — live ISO

An `archiso` profile that builds the DraftOS Desktop live/install medium. It boots
into a live **COSMIC** desktop and auto-launches **Install DraftOS**
(`draftos-installer`), which drives the [`draftos-install`](../../../crates/draftos-install)
engine to install a normal Arch base onto disk.

## Layout

```
Containerfile.iso-builder   Arch image with mkarchiso + the Rust/libcosmic toolchain
build-iso.sh                orchestrates: build binaries → stage → mkarchiso → out/
archiso/
  profiledef.sh             archiso profile (name, boot modes, permissions)
  pacman.conf               stock Arch repos (no third-party repo/keyring)
  packages.x86_64           live essentials + install tooling + COSMIC + fonts
  airootfs/                 overlay merged over archiso's `releng`:
    …/draftos-live-setup     live user + greetd autologin + Install launcher
    …/polkit-1/…             passwordless pkexec for the live user (live only)
    …/multi-user.target.wants/  enables greetd, NetworkManager, live-setup
```

The installer GUI (`draftos-installer`) and engine (`draftos-install`) are compiled
during the build and staged into `airootfs/usr/bin/` (gitignored).

## Build

Requires `podman` on the host (the ISO is built inside an Arch container; the
Fedora host has no `mkarchiso`). Needs root for loop devices and privileged
`mkarchiso`:

```bash
sudo ./build-iso.sh
```

The ISO lands in `./out/`.

## Test in a VM (UEFI)

```bash
qemu-system-x86_64 -enable-kvm -m 4096 -cpu host \
    -bios /usr/share/edk2/ovmf/OVMF_CODE.fd \
    -cdrom out/*.iso -drive file=draftos-test.qcow2,if=virtio,size=30G
```

The engine's safety gate allows real installs only when `/run/archiso` is present
(i.e. booted from this ISO), so it cannot touch the host by accident.

> Status: profile authored; first `mkarchiso` build + VM install pass pending.
