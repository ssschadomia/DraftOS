//! The install request the engine consumes.
//!
//! The GUI (or any front-end) fills this in and hands it to the engine as JSON.
//! It is intentionally decoupled from the wizard's own state so the engine has a
//! single, stable contract.

use serde::{Deserialize, Serialize};

/// A secret string (password / passphrase). It never appears in logs or `Debug`
/// output — only its presence is shown. It still (de)serializes as a plain
/// string so it can travel in the JSON request.
#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Secret(pub String);

impl Secret {
    pub fn expose(&self) -> &str {
        &self.0
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<redacted>")
    }
}

/// Where DraftOS will be installed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Target {
    /// Erase and partition a whole disk (e.g. `/dev/nvme0n1`).
    WholeDisk { device: String },
    /// Use existing partitions the user assigned (manual partitioning).
    Manual { root: String, esp: String },
}

/// Administrator (root) password policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RootPolicy {
    /// root shares the user's password.
    SameAsUser,
    /// root has its own password.
    Separate(Secret),
    /// root account is locked (sudo-only).
    Locked,
}

/// The user account created during install (first admin).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub username: String,
    #[serde(default)]
    pub full_name: String,
    pub password: Secret,
}

/// Which kernel to install.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Kernel {
    /// Arch's stock `linux`.
    #[default]
    Standard,
    /// CachyOS's optimized `linux-cachyos` (adds the CachyOS repo).
    Cachyos,
}

/// Everything the engine needs to install a system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallRequest {
    /// Locale, e.g. `en_US.UTF-8`.
    pub locale: String,
    /// Console keymap, e.g. `us`.
    pub keymap: String,
    /// X11/Wayland keyboard layouts in priority order, e.g. `["us", "ru"]`.
    #[serde(default)]
    pub x11_layouts: Vec<String>,
    /// IANA time zone, e.g. `Europe/Moscow`.
    pub timezone: String,
    /// System hostname.
    pub hostname: String,
    pub target: Target,
    /// LUKS passphrase; when set, the root filesystem is encrypted.
    #[serde(default)]
    pub luks_passphrase: Option<Secret>,
    pub account: Account,
    pub root: RootPolicy,
    #[serde(default)]
    pub kernel: Kernel,
}

impl InstallRequest {
    /// Whether the install should set up full-disk (LUKS) encryption.
    pub fn encrypted(&self) -> bool {
        self.luks_passphrase.as_ref().is_some_and(|p| !p.is_empty())
    }

    /// Basic validation before planning.
    pub fn validate(&self) -> Result<(), String> {
        // These rules mirror useradd/hostname(7); catching bad input here means
        // the install fails in seconds, not after a ten-minute pacstrap.
        if !valid_username(&self.account.username) {
            return Err(
                "username must start with a lowercase letter and contain only \
                 lowercase letters, digits, '-' and '_' (max 32 chars); 'root' is reserved"
                    .into(),
            );
        }
        if self.account.password.is_empty() {
            return Err("user password is empty".into());
        }
        if self.account.password.expose().contains(['\n', ':']) {
            return Err("the password must not contain ':' or line breaks".into());
        }
        if self.account.full_name.contains([':', '\n']) {
            return Err("the full name must not contain ':' or line breaks".into());
        }
        if !valid_hostname(&self.hostname) {
            return Err(
                "hostname must be 1-63 characters: letters, digits and '-', \
                 not starting or ending with '-'"
                    .into(),
            );
        }
        if self.locale.trim().is_empty() {
            return Err("locale is empty".into());
        }
        if let RootPolicy::Separate(p) = &self.root {
            if p.is_empty() {
                return Err("separate root password is empty".into());
            }
        }
        if let Target::Manual { root, esp } = &self.target {
            if root.trim().is_empty() || esp.trim().is_empty() {
                return Err("manual target needs both root and ESP partitions".into());
            }
            if root == esp {
                return Err("root and ESP must be different partitions".into());
            }
        }
        Ok(())
    }
}

/// useradd's default NAME_REGEX: `[a-z_][a-z0-9_-]*`, at most 32 chars; plus we
/// reserve `root` (the wizard manages root separately).
pub fn valid_username(u: &str) -> bool {
    let mut chars = u.chars();
    let first_ok = matches!(chars.next(), Some(c) if c.is_ascii_lowercase() || c == '_');
    first_ok
        && u.len() <= 32
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        && u != "root"
}

/// RFC-1123 single label: 1-63 chars of `[A-Za-z0-9-]`, no leading/trailing `-`.
pub fn valid_hostname(h: &str) -> bool {
    !h.is_empty()
        && h.len() <= 63
        && h.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        && !h.starts_with('-')
        && !h.ends_with('-')
}

/// Shared sample request for tests across the crate.
#[cfg(test)]
pub(crate) mod tests_support {
    use super::*;

    pub fn sample() -> InstallRequest {
        InstallRequest {
            locale: "en_US.UTF-8".into(),
            keymap: "us".into(),
            x11_layouts: vec!["us".into(), "ru".into()],
            timezone: "Europe/Moscow".into(),
            hostname: "draftos".into(),
            target: Target::WholeDisk { device: "/dev/sda".into() },
            luks_passphrase: None,
            account: Account {
                username: "user".into(),
                full_name: "A User".into(),
                password: Secret("hunter2".into()),
            },
            root: RootPolicy::SameAsUser,
            kernel: Kernel::Standard,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::tests_support::sample;
    use super::*;

    #[test]
    fn username_and_hostname_rules() {
        for ok in ["alice", "_svc", "a", "user-01", "x_y"] {
            assert!(valid_username(ok), "{ok} should be valid");
        }
        for bad in ["", "Root", "root", "1abc", "-a", "a b", "юзер", "a:b"] {
            assert!(!valid_username(bad), "{bad} should be invalid");
        }
        for ok in ["draftos", "my-pc", "PC01"] {
            assert!(valid_hostname(ok), "{ok} should be valid");
        }
        for bad in ["", "-x", "x-", "my pc", "a.b", &"h".repeat(64)] {
            assert!(!valid_hostname(bad), "{bad} should be invalid");
        }
    }

    #[test]
    fn validate_rejects_wasteful_late_failures() {
        let mut r = sample();
        r.account.full_name = "Team: DraftOS".into(); // ':' breaks chpasswd/GECOS
        assert!(r.validate().is_err());
        let mut r = sample();
        r.hostname = "my pc".into();
        assert!(r.validate().is_err());
    }

    #[test]
    fn secret_is_redacted_in_debug() {
        let s = Secret("hunter2".into());
        assert_eq!(format!("{s:?}"), "<redacted>");
        assert_eq!(s.expose(), "hunter2");
    }

    #[test]
    fn request_debug_hides_passwords() {
        let req = sample();
        let dbg = format!("{req:?}");
        assert!(!dbg.contains("hunter2"), "password leaked in Debug: {dbg}");
    }

    #[test]
    fn validation_catches_empties() {
        let mut req = sample();
        assert!(req.validate().is_ok());
        req.account.username = "  ".into();
        assert!(req.validate().is_err());
    }
}
