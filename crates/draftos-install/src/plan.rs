//! Turn an [`InstallRequest`] into an ordered list of [`Step`]s.
//!
//! Pure and deterministic: no disk is touched here, so the whole plan can be
//! inspected and unit-tested. Execution happens in [`crate::exec`].
//!
//! Target layout (UEFI): GPT with a FAT32 ESP mounted at `/boot` and a btrfs
//! root with subvolumes (`@`, `@home`, `@snapshots`, `@log`, `@cache`).
//! Optional LUKS2 encryption on the root partition. Bootloader: systemd-boot.

use crate::config::{InstallRequest, Kernel, RootPolicy, Secret, Target};
use crate::files;
use crate::step::{Phase, Step};

/// Partition device name, e.g. (`/dev/sda`, 1) → `/dev/sda1`; (`/dev/nvme0n1`, 2)
/// → `/dev/nvme0n1p2`.
pub fn part_name(disk: &str, n: u32) -> String {
    match disk.chars().last() {
        Some(c) if c.is_ascii_digit() => format!("{disk}p{n}"),
        _ => format!("{disk}{n}"),
    }
}

/// Packages installed via pacstrap.
fn packages(req: &InstallRequest) -> Vec<&'static str> {
    let mut p = vec![
        // base system
        "base", "base-devel", "linux-firmware", "sof-firmware",
        "amd-ucode", "intel-ucode",
        "btrfs-progs", "snapper", "zram-generator",
        "networkmanager", "sudo", "polkit",
        "git", "vim", "nano", "man-db", "xdg-user-dirs",
        "efibootmgr",
        // quiet, graphical boot (Plymouth splash instead of terminal spew)
        "plymouth",
        // COSMIC desktop — the session, login greeter, portal, and core apps
        // (mirrors what the live ISO ships so the installed system matches).
        "cosmic-session", "cosmic-greeter", "xdg-desktop-portal-cosmic",
        "cosmic-settings", "cosmic-files", "cosmic-terminal",
        "cosmic-text-editor", "cosmic-store", "cosmic-wallpapers",
        // graphics stack + keyboard maps for a usable desktop on real hardware
        "mesa", "vulkan-intel", "vulkan-radeon", "vulkan-icd-loader",
        "xorg-xwayland", "xkeyboard-config",
        // audio
        "pipewire", "pipewire-pulse", "pipewire-alsa", "wireplumber",
        // bluetooth + power management (laptops are first-class hardware)
        "bluez", "bluez-utils", "power-profiles-daemon",
        // the App Center's backend (ADR 0006: the image must ship flatpak)
        "flatpak",
        // fonts
        "inter-font", "ttf-jetbrains-mono", "noto-fonts", "noto-fonts-emoji",
        "ttf-dejavu",
    ];
    match req.kernel {
        Kernel::Standard => {
            p.push("linux");
            p.push("linux-headers");
        }
        Kernel::Cachyos => {
            p.push("linux-cachyos");
            p.push("linux-cachyos-headers");
        }
    }
    if req.encrypted() {
        p.push("cryptsetup");
    }
    p
}

/// Build the full install plan.
pub fn plan(req: &InstallRequest) -> anyhow::Result<Vec<Step>> {
    if let Err(e) = req.validate() {
        anyhow::bail!("invalid install request: {e}");
    }
    if req.kernel == Kernel::Cachyos {
        // The CachyOS repo/keyring must be wired into the live env before pacstrap;
        // that path isn't implemented yet, so refuse rather than mis-install.
        anyhow::bail!("CachyOS kernel is not supported by the engine yet — use Standard");
    }

    let encrypted = req.encrypted();
    let (rootpart, esp, whole_disk) = match &req.target {
        Target::WholeDisk { device } => (part_name(device, 2), part_name(device, 1), Some(device.clone())),
        Target::Manual { root, esp } => (root.clone(), esp.clone(), None),
    };
    // The filesystem lives on the LUKS mapper when encrypted, else on the partition.
    let fsdev = if encrypted { "/dev/mapper/cryptroot".to_string() } else { rootpart.clone() };

    let mut steps = Vec::new();

    // --- Preflight ---
    // A previous failed attempt leaves /mnt mounted (six mounts deep) and the
    // LUKS mapper open; every retry would then fail confusingly at partprobe/
    // mkfs/luksFormat. Start every run from a clean slate, best-effort.
    steps.push(Step::sh(
        Phase::Partition,
        "Clean up any previous attempt",
        "umount -R /mnt 2>/dev/null || true; cryptsetup close cryptroot 2>/dev/null || true",
    ));

    // --- Partition (whole-disk only) ---
    if let Some(disk) = &whole_disk {
        steps.push(Step::run(Phase::Partition, "Wipe partition table", &["sgdisk", "--zap-all", disk]).danger());
        steps.push(Step::run(Phase::Partition, "Create EFI partition", &["sgdisk", "-n1:0:+1G", "-t1:ef00", "-c1:DRAFTOS_ESP", disk]).danger());
        steps.push(Step::run(Phase::Partition, "Create root partition", &["sgdisk", "-n2:0:0", "-t2:8300", "-c2:DRAFTOS_ROOT", disk]).danger());
        steps.push(Step::run(Phase::Partition, "Refresh partition table", &["partprobe", disk]));
    }

    // --- Format ---
    if encrypted {
        let pass = req.luks_passphrase.clone().unwrap();
        steps.push(
            Step::run_secret(Phase::Format, "Encrypt root (LUKS2)", &["cryptsetup", "-q", "luksFormat", "--type", "luks2", &rootpart], pass.clone()).danger(),
        );
        steps.push(Step::run_secret(Phase::Format, "Open encrypted root", &["cryptsetup", "open", &rootpart, "cryptroot"], pass));
    }
    if whole_disk.is_some() {
        // Only format the ESP on a fresh whole-disk install; a manually-assigned
        // ESP is left intact (it may be shared with another OS).
        steps.push(Step::run(Phase::Format, "Format EFI partition", &["mkfs.fat", "-F32", &esp]).danger());
    }
    steps.push(Step::run(Phase::Format, "Format root (btrfs)", &["mkfs.btrfs", "-f", &fsdev]).danger());

    // btrfs subvolumes
    steps.push(Step::run(Phase::Format, "Mount root for subvolumes", &["mount", &fsdev, "/mnt"]));
    for sv in ["@", "@home", "@snapshots", "@log", "@cache"] {
        steps.push(Step::run(Phase::Format, format!("Create subvolume {sv}"), &["btrfs", "subvolume", "create", &format!("/mnt/{sv}")]));
    }
    steps.push(Step::run(Phase::Format, "Unmount root", &["umount", "/mnt"]));

    // mount the layout
    let opts = "subvol=@,compress=zstd,noatime";
    steps.push(Step::run(Phase::Format, "Mount @ at /mnt", &["mount", "-o", opts, &fsdev, "/mnt"]));
    steps.push(Step::run(Phase::Format, "Create mount points", &["mkdir", "-p", "/mnt/home", "/mnt/.snapshots", "/mnt/var/log", "/mnt/var/cache", "/mnt/boot"]));
    for (sv, mp) in [("@home", "/mnt/home"), ("@snapshots", "/mnt/.snapshots"), ("@log", "/mnt/var/log"), ("@cache", "/mnt/var/cache")] {
        steps.push(Step::run(Phase::Format, format!("Mount {sv}"), &["mount", "-o", &format!("subvol={sv},compress=zstd,noatime"), &fsdev, mp]));
    }
    steps.push(Step::run(Phase::Format, "Mount ESP at /mnt/boot", &["mount", &esp, "/mnt/boot"]));

    // --- Base system ---
    let mut pacstrap = vec!["pacstrap", "-K", "/mnt"];
    let pkgs = packages(req);
    pacstrap.extend(pkgs.iter().copied());
    steps.push(Step::run(Phase::BaseSystem, "Install base system + COSMIC", &pacstrap));
    steps.push(Step::sh(Phase::BaseSystem, "Generate fstab", "genfstab -U /mnt >> /mnt/etc/fstab"));

    // --- Configure ---
    steps.push(Step::write(Phase::Configure, "Locale", "/etc/locale.gen", files::locale_gen_line(&req.locale), 0o644));
    steps.push(Step::write(Phase::Configure, "Locale config", "/etc/locale.conf", files::locale_conf(&req.locale), 0o644));
    steps.push(Step::chroot(Phase::Configure, "Generate locale", &["locale-gen"]));
    steps.push(Step::write(Phase::Configure, "Console keymap", "/etc/vconsole.conf", files::vconsole_conf(&req.keymap), 0o644));
    if !req.x11_layouts.is_empty() {
        steps.push(Step::write(Phase::Configure, "Keyboard layout", "/etc/X11/xorg.conf.d/00-keyboard.conf", files::x11_keyboard_conf(&req.x11_layouts), 0o644));
    }
    steps.push(Step::write(Phase::Configure, "Hostname", "/etc/hostname", files::hostname_file(&req.hostname), 0o644));
    steps.push(Step::write(Phase::Configure, "Hosts", "/etc/hosts", files::hosts_file(&req.hostname), 0o644));
    steps.push(Step::chroot(Phase::Configure, "Set time zone", &["ln", "-sf", &format!("/usr/share/zoneinfo/{}", req.timezone), "/etc/localtime"]));
    steps.push(Step::chroot(Phase::Configure, "Sync hardware clock", &["hwclock", "--systohc"]));
    steps.push(Step::write(Phase::Configure, "DraftOS identity", "/etc/os-release", files::os_release(), 0o644));
    steps.push(Step::write(Phase::Configure, "Swap on zram", "/etc/systemd/zram-generator.conf", files::zram_conf(), 0o644));
    steps.push(Step::write(Phase::Configure, "Boot splash theme", "/etc/plymouth/plymouthd.conf", files::plymouthd_conf(), 0o644));
    steps.push(Step::sh(Phase::Configure, "Set mkinitcpio hooks", format!("sed -i 's/^HOOKS=.*/{}/' /mnt/etc/mkinitcpio.conf", files::mkinitcpio_hooks(encrypted))));
    steps.push(Step::chroot(Phase::Configure, "Build initramfs", &["mkinitcpio", "-P"]));

    // --- Bootloader (systemd-boot) ---
    steps.push(Step::chroot(Phase::Bootloader, "Install systemd-boot", &["bootctl", "install"]));
    steps.push(Step::write(Phase::Bootloader, "Loader config", "/boot/loader/loader.conf", files::loader_conf(), 0o644));
    let (kimg, initrd) = match req.kernel {
        Kernel::Standard => ("vmlinuz-linux", "initramfs-linux.img"),
        Kernel::Cachyos => ("vmlinuz-linux-cachyos", "initramfs-linux-cachyos.img"),
    };
    steps.push(Step::sh(Phase::Bootloader, "Write boot entry", boot_entry_script(&rootpart, encrypted, kimg, initrd)));

    // --- Users ---
    steps.push(Step::write(Phase::Users, "Enable sudo for wheel", "/etc/sudoers.d/10-wheel", "%wheel ALL=(ALL:ALL) ALL\n", 0o440));
    let user = &req.account.username;
    steps.push(Step::chroot(Phase::Users, "Create user", &["useradd", "-m", "-G", "wheel", "-s", "/bin/bash", "-c", &req.account.full_name, user]));
    steps.push(Step::chroot_secret(Phase::Users, "Set user password", &["chpasswd"], Secret(format!("{user}:{}", req.account.password.expose()))));
    match &req.root {
        RootPolicy::SameAsUser => steps.push(Step::chroot_secret(Phase::Users, "Set root password (same as user)", &["chpasswd"], Secret(format!("root:{}", req.account.password.expose())))),
        RootPolicy::Separate(p) => steps.push(Step::chroot_secret(Phase::Users, "Set root password", &["chpasswd"], Secret(format!("root:{}", p.expose())))),
        RootPolicy::Locked => steps.push(Step::chroot(Phase::Users, "Lock root account", &["passwd", "-l", "root"])),
    }
    steps.push(Step::chroot(Phase::Users, "Enable services", &["systemctl", "enable", "NetworkManager", "cosmic-greeter", "systemd-timesyncd", "fstrim.timer", "bluetooth", "power-profiles-daemon"]));
    // Flathub for the App Center (system-wide; arch-chroot binds resolv.conf so
    // the .flatpakrepo fetch works). Best-effort: a failure here must not waste
    // the whole install — the App Center can also self-provision later.
    steps.push(Step::sh(Phase::Users, "Add Flathub remote", "arch-chroot /mnt flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo || true"));

    // --- Finish ---
    steps.push(Step::run(Phase::Finish, "Unmount everything", &["umount", "-R", "/mnt"]));
    if encrypted {
        steps.push(Step::run(Phase::Finish, "Close encrypted root", &["cryptsetup", "close", "cryptroot"]));
    }

    Ok(steps)
}

/// A bash snippet that resolves the root UUID at run time and writes the
/// systemd-boot entry. Deterministic given its inputs (unit-tested).
fn boot_entry_script(rootpart: &str, encrypted: bool, kimg: &str, initrd: &str) -> String {
    let cmdline = files::kernel_cmdline("$RUUID", encrypted);
    format!(
        "set -e\n\
         RUUID=$(blkid -s UUID -o value {rootpart})\n\
         cat > /mnt/boot/loader/entries/draftos.conf <<EOF\n\
         title   DraftOS\n\
         linux   /{kimg}\n\
         initrd  /amd-ucode.img\n\
         initrd  /intel-ucode.img\n\
         initrd  /{initrd}\n\
         options {cmdline}\n\
         EOF\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::tests_support::sample;

    #[test]
    fn part_naming() {
        assert_eq!(part_name("/dev/sda", 1), "/dev/sda1");
        assert_eq!(part_name("/dev/nvme0n1", 2), "/dev/nvme0n1p2");
        assert_eq!(part_name("/dev/mmcblk0", 1), "/dev/mmcblk0p1");
    }

    #[test]
    fn clean_plan_has_core_phases_in_order() {
        let steps = plan(&sample()).unwrap();
        let phases: Vec<_> = steps.iter().map(|s| s.phase).collect();
        // partition precedes format precedes base precedes bootloader precedes finish
        let first = |p| phases.iter().position(|x| *x == p).unwrap();
        assert!(first(Phase::Partition) < first(Phase::Format));
        assert!(first(Phase::Format) < first(Phase::BaseSystem));
        assert!(first(Phase::BaseSystem) < first(Phase::Bootloader));
        assert!(first(Phase::Bootloader) < first(Phase::Finish));
    }

    #[test]
    fn clean_plan_partitions_and_pacstraps() {
        let s = plan(&sample()).unwrap();
        let sums: Vec<String> = s.iter().map(|x| x.summary()).collect();
        assert!(sums.iter().any(|c| c.contains("sgdisk --zap-all /dev/sda")));
        assert!(sums.iter().any(|c| c.contains("mkfs.btrfs -f /dev/sda2")));
        assert!(sums.iter().any(|c| c.contains("pacstrap -K /mnt") && c.contains("cosmic")));
        assert!(sums.iter().any(|c| c.contains("bootctl install")));
    }

    #[test]
    fn destructive_steps_are_flagged() {
        let s = plan(&sample()).unwrap();
        assert!(s.iter().any(|x| x.destructive && x.summary().contains("zap-all")));
        assert!(s.iter().any(|x| x.destructive && x.summary().contains("mkfs.btrfs")));
    }

    #[test]
    fn no_secret_leaks_in_summaries() {
        let mut req = sample();
        req.luks_passphrase = Some(Secret("diskpass".into()));
        let s = plan(&req).unwrap();
        for step in &s {
            let sum = step.summary();
            assert!(!sum.contains("hunter2"), "user pw leaked: {sum}");
            assert!(!sum.contains("diskpass"), "luks pw leaked: {sum}");
        }
    }

    #[test]
    fn encryption_changes_the_plan() {
        let mut req = sample();
        req.luks_passphrase = Some(Secret("diskpass".into()));
        let s = plan(&req).unwrap();
        let sums: Vec<String> = s.iter().map(|x| x.summary()).collect();
        assert!(sums.iter().any(|c| c.contains("luksFormat")));
        assert!(sums.iter().any(|c| c.contains("mkfs.btrfs -f /dev/mapper/cryptroot")));
    }

    #[test]
    fn manual_target_skips_partitioning_and_esp_format() {
        let mut req = sample();
        req.target = Target::Manual { root: "/dev/sda3".into(), esp: "/dev/sda1".into() };
        let s = plan(&req).unwrap();
        let sums: Vec<String> = s.iter().map(|x| x.summary()).collect();
        assert!(!sums.iter().any(|c| c.contains("sgdisk")), "manual should not partition");
        assert!(!sums.iter().any(|c| c.contains("mkfs.fat")), "manual should not format ESP");
        assert!(sums.iter().any(|c| c.contains("mkfs.btrfs -f /dev/sda3")));
    }

    #[test]
    fn cachyos_is_rejected_for_now() {
        let mut req = sample();
        req.kernel = Kernel::Cachyos;
        assert!(plan(&req).is_err());
    }
}
