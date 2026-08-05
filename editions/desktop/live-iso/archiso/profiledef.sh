#!/usr/bin/env bash
# shellcheck disable=SC2034
#
# DraftOS Desktop live ISO — archiso profile definition.
# Boots into a live COSMIC desktop; "Install DraftOS" runs our libcosmic
# installer, which drives the draftos-install engine to install a normal Arch
# base onto disk. Modeled on archiso's `releng`.

iso_name="draftos"
iso_label="DRAFTOS_$(date --date="@${SOURCE_DATE_EPOCH:-0}" +%Y%m 2>/dev/null || echo 000000)"
iso_publisher="DraftOS <https://github.com/ssschadomia/DraftOS>"
iso_application="DraftOS Desktop Live/Install (COSMIC)"
iso_version="$(date --date="@${SOURCE_DATE_EPOCH:-0}" +%Y.%m.%d 2>/dev/null || echo dev)"
install_dir="draftos"
buildmodes=('iso')
bootmodes=(
  'bios.syslinux.mbr'
  'bios.syslinux.eltorito'
  'uefi-ia32.systemd-boot.esp'
  'uefi-x64.systemd-boot.esp'
  'uefi-ia32.systemd-boot.eltorito'
  'uefi-x64.systemd-boot.eltorito'
)
arch="x86_64"
pacman_conf="pacman.conf"
airootfs_image_type="squashfs"
airootfs_image_tool_options=('-comp' 'zstd' '-Xcompression-level' '15' '-b' '1M')
bootstrap_tarball_compression=('zstd' '-c' '-T0' '--auto-threads=logical' '--long' '-19')
file_permissions=(
  ["/etc/shadow"]="0:0:400"
  ["/etc/gshadow"]="0:0:400"
  ["/root"]="0:0:750"
  ["/usr/local/bin/draftos-live-setup"]="0:0:755"
  ["/usr/bin/draftos-installer"]="0:0:755"
  ["/usr/bin/draftos-install"]="0:0:755"
)
