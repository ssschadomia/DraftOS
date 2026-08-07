//! Pure generators for the config files written into the new system.
//!
//! Everything here is a deterministic function of the request, which makes it
//! easy to unit-test. Anything that depends on values only known at run time
//! (partition UUIDs, etc.) lives in [`crate::plan`] as executed steps instead.

/// The DraftOS pacman repository, served from Cloudflare R2 (egress-free CDN).
/// `$arch` is expanded by pacman itself, so it stays literal here.
pub const DRAFTOS_REPO_URL: &str =
    "https://pub-a895d58bfaca4ab49dfcee8b9bb5604f.r2.dev/repo/$arch";

/// Fingerprint of the DraftOS package-signing key (its public half is embedded
/// via [`packaging_pubkey`]). Used to locally sign (trust) the key in the target
/// keyring so pacman accepts our signed packages.
pub const DRAFTOS_KEY_ID: &str = "8D33B2461848D54797DFD0CFC386D8413641AD97";

/// The DraftOS packaging public key (ASCII-armored), embedded at build time.
pub fn packaging_pubkey() -> &'static str {
    include_str!("../../../packaging/keyring/draftos-packaging.asc")
}

/// The `[draftos]` stanza appended to the target `pacman.conf`. `SigLevel`
/// requires package signatures (our key) but treats the db signature as
/// optional, matching how `repo-add` signs.
pub fn draftos_repo_stanza() -> String {
    format!("\n[draftos]\nSigLevel = Required DatabaseOptional\nServer = {DRAFTOS_REPO_URL}\n")
}

/// A minimal resolver used only during install so the chroot's pacman can reach
/// the network. NetworkManager rewrites `/etc/resolv.conf` on first boot.
pub fn install_resolv_conf() -> String {
    "nameserver 1.1.1.1\nnameserver 9.9.9.9\n".to_string()
}

/// `/etc/locale.conf`.
pub fn locale_conf(locale: &str) -> String {
    format!("LANG={locale}\n")
}

/// The line to enable in `/etc/locale.gen` (DraftOS offers only UTF-8 locales).
pub fn locale_gen_line(locale: &str) -> String {
    format!("{locale} UTF-8\n")
}

/// `/etc/vconsole.conf` (TTY keymap).
pub fn vconsole_conf(keymap: &str) -> String {
    format!("KEYMAP={keymap}\n")
}

/// `/etc/hostname`.
pub fn hostname_file(hostname: &str) -> String {
    format!("{hostname}\n")
}

/// `/etc/hosts`.
pub fn hosts_file(hostname: &str) -> String {
    format!(
        "127.0.0.1\tlocalhost\n\
         ::1\t\tlocalhost\n\
         127.0.1.1\t{hostname}.localdomain\t{hostname}\n"
    )
}

/// `/etc/X11/xorg.conf.d/00-keyboard.conf` for the Wayland/X11 layouts.
/// The first layout is primary; extra layouts cycle with Alt+Shift.
pub fn x11_keyboard_conf(layouts: &[String]) -> String {
    let joined = layouts.join(",");
    let options = if layouts.len() > 1 {
        "        Option \"XkbOptions\" \"grp:alt_shift_toggle\"\n"
    } else {
        ""
    };
    format!(
        "Section \"InputClass\"\n\
        \x20       Identifier \"system-keyboard\"\n\
        \x20       MatchIsKeyboard \"on\"\n\
        \x20       Option \"XkbLayout\" \"{joined}\"\n\
         {options}\
         EndSection\n"
    )
}

/// Plymouth daemon config — selects the boot-splash theme. `bgrt` shows the
/// firmware/OEM logo with a spinner (falling back to the plain `spinner` theme
/// when no logo is present), which gives a clean, modern boot on most hardware.
pub fn plymouthd_conf() -> String {
    "[Daemon]\nTheme=bgrt\n".to_string()
}

/// `/etc/os-release` — the installed system identifies as DraftOS, not Arch.
/// Written to /etc (which shadows /usr/lib/os-release), so pacman upgrades of
/// the `filesystem` package don't clobber it.
pub fn os_release() -> String {
    "NAME=\"DraftOS\"\n\
     PRETTY_NAME=\"DraftOS\"\n\
     ID=draftos\n\
     ID_LIKE=arch\n\
     BUILD_ID=rolling\n\
     ANSI_COLOR=\"38;2;23;147;209\"\n\
     HOME_URL=\"https://github.com/ssschadomia/DraftOS\"\n\
     DOCUMENTATION_URL=\"https://github.com/ssschadomia/DraftOS\"\n\
     BUG_REPORT_URL=\"https://github.com/ssschadomia/DraftOS/issues\"\n\
     LOGO=draftos\n"
        .to_string()
}

/// `zram-generator` config: compressed swap on zram, sized to half the RAM
/// capped at 4 GiB. The package alone ships no active default, so without this
/// the installed system has no swap at all.
pub fn zram_conf() -> String {
    "[zram0]\nzram-size = min(ram / 2, 4096)\ncompression-algorithm = zstd\n".to_string()
}

/// `/etc/snapper/configs/root` — written directly rather than via
/// `snapper create-config`, because our btrfs layout already provides the
/// `@snapshots` subvolume mounted at `/.snapshots` (create-config would refuse).
/// Timeline auto-snapshots are off; snapshots come from pacman (snap-pac) and
/// `draftos update`, which is what `draftos rollback` restores.
pub fn snapper_root_config() -> String {
    "SUBVOLUME=\"/\"\n\
     FSTYPE=\"btrfs\"\n\
     QGROUP=\"\"\n\
     SPACE_LIMIT=\"0.5\"\n\
     FREE_LIMIT=\"0.2\"\n\
     ALLOW_USERS=\"\"\n\
     ALLOW_GROUPS=\"\"\n\
     SYNC_ACL=\"no\"\n\
     BACKGROUND_COMPARISON=\"yes\"\n\
     NUMBER_CLEANUP=\"yes\"\n\
     NUMBER_MIN_AGE=\"1800\"\n\
     NUMBER_LIMIT=\"50\"\n\
     NUMBER_LIMIT_IMPORTANT=\"10\"\n\
     TIMELINE_CREATE=\"no\"\n\
     TIMELINE_CLEANUP=\"yes\"\n\
     TIMELINE_MIN_AGE=\"1800\"\n\
     TIMELINE_LIMIT_HOURLY=\"5\"\n\
     TIMELINE_LIMIT_DAILY=\"7\"\n\
     TIMELINE_LIMIT_WEEKLY=\"0\"\n\
     TIMELINE_LIMIT_MONTHLY=\"0\"\n\
     TIMELINE_LIMIT_YEARLY=\"0\"\n\
     EMPTY_PRE_POST_CLEANUP=\"yes\"\n\
     EMPTY_PRE_POST_MIN_AGE=\"1800\"\n"
        .to_string()
}

/// `/etc/conf.d/snapper` — registers the `root` config so the snapper timers act
/// on it.
pub fn snapper_conf_d() -> String {
    "SNAPPER_CONFIGS=\"root\"\n".to_string()
}

/// `systemd-boot` `loader/loader.conf`.
pub fn loader_conf() -> String {
    "default draftos.conf\ntimeout 3\nconsole-mode max\neditor no\n".to_string()
}

/// A `systemd-boot` entry (`loader/entries/draftos.conf`). `options` is the
/// kernel command line, assembled by [`kernel_cmdline`].
pub fn loader_entry(kernel_image: &str, initramfs: &str, options: &str) -> String {
    format!(
        "title   DraftOS\n\
         linux   /{kernel_image}\n\
         initrd  /{initramfs}\n\
         options {options}\n"
    )
}

/// Kernel command line. `root_uuid` is the UUID of the (possibly encrypted)
/// container. With encryption we hand the LUKS partition to mkinitcpio's
/// `encrypt` hook and boot the mapped device.
pub fn kernel_cmdline(root_uuid: &str, encrypted: bool) -> String {
    // `quiet splash` + low log levels hand the screen to Plymouth instead of
    // scrolling boot messages; `vt.global_cursor_default=0` hides the blinking
    // cursor before the greeter appears.
    let quiet = "rw quiet splash loglevel=3 rd.udev.log_level=3 vt.global_cursor_default=0";
    if encrypted {
        format!("cryptdevice=UUID={root_uuid}:cryptroot root=/dev/mapper/cryptroot rootflags=subvol=@ {quiet}")
    } else {
        format!("root=UUID={root_uuid} rootflags=subvol=@ {quiet}")
    }
}

/// The `mkinitcpio` HOOKS line. `plymouth` (right after `udev`) drives the boot
/// splash. For LUKS the standard `encrypt` hook is used — since mkinitcpio 30+
/// it detects a running Plymouth and asks for the passphrase through it, and the
/// old separate `plymouth-encrypt` hook no longer exists (verified: plymouth
/// 26.x ships only `usr/lib/initcpio/hooks/plymouth`).
pub fn mkinitcpio_hooks(encrypted: bool) -> String {
    if encrypted {
        "HOOKS=(base udev plymouth autodetect microcode modconf kms keyboard keymap consolefont block encrypt filesystems fsck)".into()
    } else {
        "HOOKS=(base udev plymouth autodetect microcode modconf kms keyboard keymap consolefont block filesystems fsck)".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_and_vconsole() {
        assert_eq!(locale_conf("en_US.UTF-8"), "LANG=en_US.UTF-8\n");
        assert_eq!(locale_gen_line("ru_RU.UTF-8"), "ru_RU.UTF-8 UTF-8\n");
        assert_eq!(vconsole_conf("us"), "KEYMAP=us\n");
    }

    #[test]
    fn hosts_has_hostname() {
        let h = hosts_file("draftos");
        assert!(h.contains("127.0.1.1\tdraftos.localdomain\tdraftos"));
        assert!(h.contains("localhost"));
    }

    #[test]
    fn keyboard_multi_layout_adds_toggle() {
        let c = x11_keyboard_conf(&["us".into(), "ru".into()]);
        assert!(c.contains("\"us,ru\""));
        assert!(c.contains("grp:alt_shift_toggle"));
        let single = x11_keyboard_conf(&["us".into()]);
        assert!(!single.contains("grp:alt_shift_toggle"));
    }

    #[test]
    fn cmdline_switches_on_encryption() {
        let plain = kernel_cmdline("ABC", false);
        assert!(plain.starts_with("root=UUID=ABC rootflags=subvol=@ rw"));
        assert!(plain.contains("quiet splash"));
        let enc = kernel_cmdline("ABC", true);
        assert!(enc.contains("cryptdevice=UUID=ABC:cryptroot"));
        assert!(enc.contains("quiet splash"));
        assert!(enc.contains("root=/dev/mapper/cryptroot"));
    }

    #[test]
    fn hooks_add_encrypt_only_when_needed() {
        assert!(mkinitcpio_hooks(true).contains(" encrypt filesystems"));
        // `plymouth-encrypt` no longer exists in plymouth 26.x — never emit it.
        assert!(!mkinitcpio_hooks(true).contains("plymouth-encrypt"));
        assert!(!mkinitcpio_hooks(false).contains("encrypt"));
        // Plymouth splash on both paths.
        assert!(mkinitcpio_hooks(false).contains(" plymouth "));
    }

    #[test]
    fn loader_entry_shape() {
        let e = loader_entry("vmlinuz-linux", "initramfs-linux.img", "root=UUID=X rw");
        assert!(e.contains("title   DraftOS"));
        assert!(e.contains("linux   /vmlinuz-linux"));
        assert!(e.contains("options root=UUID=X rw"));
    }
}
