//! DraftOS brand identity constants.
//!
//! These are the canonical strings for the project name, application id prefix
//! and home URL. Keeping them in one place means tools and `.desktop`/`os-release`
//! generation never disagree on spelling or casing.

/// Lowercase machine id, used for paths, binaries and `os-release` `ID`.
pub const ID: &str = "draftos";

/// Human-facing product name.
pub const NAME: &str = "DraftOS";

/// `os-release`-style pretty name.
pub const PRETTY_NAME: &str = "DraftOS";

/// Reverse-DNS prefix for application ids (e.g. `org.draftos.Store`).
pub const APP_ID_PREFIX: &str = "org.draftos";

/// Project home / source URL.
pub const HOME_URL: &str = "https://github.com/schadomia/DraftOS";

/// Crate version, taken from Cargo at build time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build a fully-qualified application id, e.g. `app_id("Store")` →
/// `"org.draftos.Store"`.
pub fn app_id(name: &str) -> String {
    format!("{APP_ID_PREFIX}.{name}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_id_is_reverse_dns() {
        assert_eq!(app_id("Store"), "org.draftos.Store");
        assert_eq!(app_id("Welcome"), "org.draftos.Welcome");
    }
}
