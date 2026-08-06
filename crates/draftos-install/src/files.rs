//! Pure generators for the config files written into the new system.
//!
//! Everything here is a deterministic function of the request, which makes it
//! easy to unit-test. Anything that depends on values only known at run time
//! (partition UUIDs, etc.) lives in [`crate::plan`] as executed steps instead.

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
/// splash; for LUKS we use `plymouth-encrypt` so the passphrase prompt is drawn
/// by Plymouth rather than on the bare console.
pub fn mkinitcpio_hooks(encrypted: bool) -> String {
    if encrypted {
        "HOOKS=(base udev plymouth autodetect microcode modconf kms keyboard keymap consolefont block plymouth-encrypt filesystems fsck)".into()
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
        assert!(mkinitcpio_hooks(true).contains(" plymouth-encrypt filesystems"));
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
