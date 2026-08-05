//! The configuration the wizard assembles, plus the fixed choice lists it offers.
//!
//! This is the data the (future) install engine will consume. Keeping it separate
//! from the UI keeps the boundary clean: the wizard only fills in an
//! [`InstallConfig`]; nothing here touches the system.

/// How the target disk should be provisioned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallType {
    /// Erase a whole disk and install DraftOS on it.
    Clean,
    /// Install into free space next to an existing operating system.
    Alongside,
    /// Replace an existing DraftOS installation, keeping the disk layout.
    Reinstall,
    /// Hand-assign existing partitions (advanced).
    Manual,
}

impl InstallType {
    /// Short label for summaries.
    pub fn label(self) -> &'static str {
        match self {
            InstallType::Clean => "Clean install",
            InstallType::Alongside => "Install alongside",
            InstallType::Reinstall => "Reinstall",
            InstallType::Manual => "Manual partitioning",
        }
    }
}

/// Everything the wizard collects, consumed later by the install engine.
///
/// Fields are added as each step lands; the disk, encryption and account steps
/// will extend this once their screens are built.
#[derive(Debug, Default, Clone)]
pub struct InstallConfig {
    /// Chosen locale (e.g. `en_US.UTF-8`), set on the Language step.
    pub locale: Option<String>,
    /// Ordered XKB layout codes (the first is primary), set on the Keyboard step.
    pub keyboard_layouts: Vec<String>,
    /// Index into [`KEYBOARD_SWITCHES`] for how to cycle layouts (when 2+).
    pub keyboard_switch: Option<usize>,
    /// IANA time zone (e.g. `Europe/Moscow`), set on the Timezone step.
    pub timezone: Option<String>,
    pub install_type: Option<InstallType>,
    /// Target disk device path (e.g. `/dev/nvme0n1`) for whole-disk installs.
    pub disk: Option<String>,
    /// Manual partitioning: the partition to use as root (`/`).
    pub root_partition: Option<String>,
    /// Manual partitioning: the EFI system partition (`/boot/efi`), if any.
    pub efi_partition: Option<String>,
    /// Whether to set up LUKS full-disk encryption.
    pub encrypt: bool,
    /// LUKS passphrase; kept in memory only for the length of the install.
    pub luks_password: String,
    // Account (filled on the Account step).
    pub full_name: String,
    pub username: String,
    pub hostname: String,
    /// Kept in memory only for the length of the install; never persisted here.
    pub password: String,
    /// When true, root gets its own password ([`Self::root_password`]); when
    /// false (the default), root shares the user's password.
    pub root_separate: bool,
    /// Root password, used only when [`Self::root_separate`] is true.
    pub root_password: String,
}

impl InstallConfig {
    /// Convert the wizard's choices into the engine's [`InstallRequest`].
    /// Returns a human-readable error if a required choice is missing or the
    /// selected install type isn't supported by the engine yet.
    pub fn to_request(&self) -> Result<draftos_install::InstallRequest, String> {
        use draftos_install::config as eng;

        let locale = self.locale.clone().ok_or("no language selected")?;
        let timezone = self.timezone.clone().ok_or("no time zone selected")?;
        if self.keyboard_layouts.is_empty() {
            return Err("no keyboard layout selected".into());
        }
        let keymap = self.keyboard_layouts[0].clone();
        let hostname = if self.hostname.trim().is_empty() {
            "draftos".to_string()
        } else {
            self.hostname.clone()
        };

        let target = match self.install_type {
            Some(InstallType::Clean) => eng::Target::WholeDisk {
                device: self.disk.clone().ok_or("no disk selected")?,
            },
            Some(InstallType::Manual) => eng::Target::Manual {
                root: self.root_partition.clone().ok_or("no root partition assigned")?,
                esp: self.efi_partition.clone().ok_or("no EFI partition assigned")?,
            },
            Some(InstallType::Alongside) | Some(InstallType::Reinstall) => {
                return Err("alongside/reinstall installs are not supported by the engine yet".into())
            }
            None => return Err("no installation type selected".into()),
        };

        let luks_passphrase = (self.encrypt && !self.luks_password.is_empty())
            .then(|| eng::Secret(self.luks_password.clone()));

        let root = if self.root_separate {
            eng::RootPolicy::Separate(eng::Secret(self.root_password.clone()))
        } else {
            eng::RootPolicy::SameAsUser
        };

        let request = eng::InstallRequest {
            locale,
            keymap,
            x11_layouts: self.keyboard_layouts.clone(),
            timezone,
            hostname,
            target,
            luks_passphrase,
            account: eng::Account {
                username: self.username.clone(),
                full_name: self.full_name.clone(),
                password: eng::Secret(self.password.clone()),
            },
            root,
            kernel: eng::Kernel::Standard,
        };
        request.validate()?;
        Ok(request)
    }
}

/// (label, XKB `grp:` option) for cycling between layouts, offered when two or
/// more layouts are selected on the Keyboard step.
pub const KEYBOARD_SWITCHES: &[(&str, &str)] = &[
    ("Alt+Shift", "grp:alt_shift_toggle"),
    ("Ctrl+Shift", "grp:ctrl_shift_toggle"),
    ("Super+Space", "grp:win_space_toggle"),
    ("Alt+Space", "grp:alt_space_toggle"),
    ("Caps Lock", "grp:caps_toggle"),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn filled() -> InstallConfig {
        InstallConfig {
            locale: Some("en_US.UTF-8".into()),
            keyboard_layouts: vec!["us".into(), "ru".into()],
            keyboard_switch: Some(0),
            timezone: Some("Europe/Moscow".into()),
            install_type: Some(InstallType::Clean),
            disk: Some("/dev/sda".into()),
            root_partition: None,
            efi_partition: None,
            encrypt: false,
            luks_password: String::new(),
            full_name: "Alex".into(),
            username: "alex".into(),
            hostname: String::new(),
            password: "pw".into(),
            root_separate: false,
            root_password: String::new(),
        }
    }

    #[test]
    fn clean_install_converts() {
        let req = filled().to_request().expect("should convert");
        assert_eq!(req.keymap, "us");
        assert_eq!(req.x11_layouts, vec!["us".to_string(), "ru".into()]);
        assert_eq!(req.hostname, "draftos"); // empty → default
        assert!(!req.encrypted());
        // engine accepts the request
        draftos_install::plan(&req).expect("engine should plan it");
    }

    #[test]
    fn manual_needs_partitions() {
        let mut c = filled();
        c.install_type = Some(InstallType::Manual);
        assert!(c.to_request().is_err()); // no partitions assigned
        c.root_partition = Some("/dev/sda3".into());
        c.efi_partition = Some("/dev/sda1".into());
        assert!(c.to_request().is_ok());
    }

    #[test]
    fn alongside_reinstall_rejected_for_now() {
        let mut c = filled();
        c.install_type = Some(InstallType::Alongside);
        assert!(c.to_request().is_err());
        c.install_type = Some(InstallType::Reinstall);
        assert!(c.to_request().is_err());
    }

    #[test]
    fn missing_choices_error() {
        let mut c = filled();
        c.locale = None;
        assert!(c.to_request().is_err());
    }
}
