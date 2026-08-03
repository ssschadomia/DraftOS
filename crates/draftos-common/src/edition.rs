//! Which DraftOS edition are we running on?
//!
//! DraftOS ships two editions (see `docs/decisions/0003-two-editions-shared-layer.md`).
//! The [`Edition`] a tool detects decides its system-management backend — the
//! `draftos` CLI drives `arkdep` on [`Edition::Immutable`] and `pacman` + `snapper`
//! on [`Edition::Desktop`].
//!
//! Detection is split in two so it can be unit-tested without a real system:
//! [`resolve`] is a pure function of its inputs, and [`Edition::detect`] gathers
//! those inputs from the running host and delegates to it.

use std::fs;
use std::path::Path;

/// A DraftOS edition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edition {
    /// Mutable Arch + CachyOS base, managed with `pacman` + `snapper`.
    Desktop,
    /// Atomic arkdep image (A/B, btrfs), managed with `arkdep`.
    Immutable,
}

impl Edition {
    /// Stable lowercase identifier, matching `os-release`'s `VARIANT_ID` and the
    /// `DRAFTOS_EDITION` override.
    pub fn id(self) -> &'static str {
        match self {
            Edition::Desktop => "desktop",
            Edition::Immutable => "immutable",
        }
    }

    /// Whether this edition uses an atomic (immutable, A/B) update model.
    pub fn is_atomic(self) -> bool {
        matches!(self, Edition::Immutable)
    }

    /// Parse an [`Edition`] from its [`id`](Edition::id), case-insensitively.
    /// Returns `None` for anything unrecognized.
    pub fn from_id(s: &str) -> Option<Edition> {
        match s.trim().to_ascii_lowercase().as_str() {
            "desktop" => Some(Edition::Desktop),
            "immutable" => Some(Edition::Immutable),
            _ => None,
        }
    }

    /// Detect the edition of the running system.
    ///
    /// Precedence: the `DRAFTOS_EDITION` env override, then `os-release`'s
    /// `VARIANT_ID`, then the presence of `/arkdep`, then [`Edition::Desktop`].
    pub fn detect() -> Edition {
        let env_override = std::env::var("DRAFTOS_EDITION").ok();
        let os_release = fs::read_to_string("/etc/os-release")
            .or_else(|_| fs::read_to_string("/usr/lib/os-release"))
            .ok();
        let arkdep_present = Path::new("/arkdep").is_dir();
        resolve(env_override.as_deref(), os_release.as_deref(), arkdep_present)
    }
}

impl std::fmt::Display for Edition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}

/// Resolve an [`Edition`] from raw inputs, in precedence order.
///
/// Kept pure (no filesystem or environment access) so the decision logic is
/// fully unit-testable; [`Edition::detect`] supplies the real-world inputs.
///
/// - `env_override`: value of `DRAFTOS_EDITION`, if set.
/// - `os_release`: contents of `os-release`, if readable.
/// - `arkdep_present`: whether `/arkdep` exists.
pub fn resolve(env_override: Option<&str>, os_release: Option<&str>, arkdep_present: bool) -> Edition {
    if let Some(e) = env_override.and_then(Edition::from_id) {
        return e;
    }
    if let Some(e) = os_release.and_then(variant_id).as_deref().and_then(Edition::from_id) {
        return e;
    }
    if arkdep_present {
        return Edition::Immutable;
    }
    Edition::Desktop
}

/// Extract the `VARIANT_ID` value from `os-release` contents, unquoting it.
/// Returns `None` if the key is absent.
fn variant_id(os_release: &str) -> Option<String> {
    for line in os_release.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("VARIANT_ID=") {
            let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_override_wins_over_everything() {
        // Even with an arkdep marker and a conflicting os-release, the override wins.
        let os = "VARIANT_ID=immutable\n";
        assert_eq!(resolve(Some("desktop"), Some(os), true), Edition::Desktop);
        assert_eq!(resolve(Some("IMMUTABLE"), Some("VARIANT_ID=desktop"), false), Edition::Immutable);
    }

    #[test]
    fn unknown_env_override_is_ignored() {
        // Falls through to the next signal rather than panicking or defaulting blindly.
        assert_eq!(resolve(Some("nonsense"), Some("VARIANT_ID=immutable"), false), Edition::Immutable);
    }

    #[test]
    fn os_release_variant_id_is_used() {
        assert_eq!(resolve(None, Some("VARIANT_ID=desktop\n"), true), Edition::Desktop);
        assert_eq!(resolve(None, Some("VARIANT_ID=\"immutable\"\n"), false), Edition::Immutable);
    }

    #[test]
    fn arkdep_presence_is_the_fallback_signal() {
        assert_eq!(resolve(None, None, true), Edition::Immutable);
        assert_eq!(resolve(None, Some("NAME=DraftOS\n"), true), Edition::Immutable);
    }

    #[test]
    fn defaults_to_desktop_with_no_signals() {
        assert_eq!(resolve(None, None, false), Edition::Desktop);
    }

    #[test]
    fn variant_id_parsing() {
        assert_eq!(variant_id("VARIANT_ID=desktop").as_deref(), Some("desktop"));
        assert_eq!(variant_id("  VARIANT_ID='immutable'  ").as_deref(), Some("immutable"));
        assert_eq!(variant_id("NAME=DraftOS\nVARIANT_ID=\"desktop\"\n").as_deref(), Some("desktop"));
        assert_eq!(variant_id("VARIANT_ID=\n"), None);
        assert_eq!(variant_id("NAME=DraftOS\n"), None);
    }

    #[test]
    fn round_trip_id() {
        for e in [Edition::Desktop, Edition::Immutable] {
            assert_eq!(Edition::from_id(e.id()), Some(e));
        }
    }
}
