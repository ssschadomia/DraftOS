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
    /// Index into [`LANGUAGES`].
    pub language: Option<usize>,
    /// Index into [`KEYBOARDS`].
    pub keyboard: Option<usize>,
    /// IANA time zone (e.g. `Europe/Moscow`), set on the Timezone step.
    pub timezone: Option<String>,
    pub install_type: Option<InstallType>,
    /// Target disk device path (e.g. `/dev/nvme0n1`), set on the Disk step.
    pub disk: Option<String>,
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

/// (display name, locale) offered on the Language step.
pub const LANGUAGES: &[(&str, &str)] = &[
    ("English", "en_US.UTF-8"),
    ("Русский", "ru_RU.UTF-8"),
    ("Deutsch", "de_DE.UTF-8"),
    ("Español", "es_ES.UTF-8"),
    ("Français", "fr_FR.UTF-8"),
    ("Italiano", "it_IT.UTF-8"),
    ("Português", "pt_BR.UTF-8"),
    ("日本語", "ja_JP.UTF-8"),
];

/// (display name, XKB layout) offered on the Keyboard step.
pub const KEYBOARDS: &[(&str, &str)] = &[
    ("English (US)", "us"),
    ("Russian", "ru"),
    ("German", "de"),
    ("Spanish", "es"),
    ("French", "fr"),
    ("Italian", "it"),
    ("Portuguese (Brazil)", "br"),
];
