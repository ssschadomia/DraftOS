//! Thin async wrapper over the `flatpak` CLI + a screenshot cache.
//!
//! Everyday actions target the **user** installation (`--user`) so no root prompt
//! is needed while browsing and installing. The store never links libflatpak; it
//! drives the same tool a user would, keeping the UI decoupled from the package
//! machinery (mirrors the install-engine split, ADR 0005).

use std::collections::HashSet;
use std::path::PathBuf;

/// Ids of every installed app (user + system). These are true flatpak ids —
/// compare them against [`crate::model::App::flatpak_id`] (parsed from the
/// AppStream `<bundle>`), never against the raw component id: ~155 legacy
/// components carry a `.desktop` suffix their flatpak ref doesn't have.
pub fn installed_ids() -> HashSet<String> {
    let out = std::process::Command::new("flatpak")
        .args(["list", "--app", "--columns=application"])
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => HashSet::new(),
    }
}

/// Ids with an update available (needs network; may take seconds). `Err` means
/// the check itself failed — callers must NOT render that as "up to date".
pub fn updatable_ids() -> Result<HashSet<String>, String> {
    let out = std::process::Command::new("flatpak")
        .args(["remote-ls", "--updates", "--app", "--columns=application"])
        .output()
        .map_err(|e| format!("could not run flatpak: {e}"))?;
    if !out.status.success() {
        return Err(last_line(&out.stderr, "the update check failed"));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

/// Install `id` from Flathub into the user installation.
///
/// The user installation may have no Flathub remote yet (fresh systems, or hosts
/// where Flathub is only a filtered system remote), so the store is
/// self-sufficient: it idempotently adds the user-level Flathub remote first.
pub async fn install(id: &str) -> Result<(), String> {
    let _ = run(&[
        "remote-add",
        "--user",
        "--if-not-exists",
        "flathub",
        "https://dl.flathub.org/repo/flathub.flatpakrepo",
    ])
    .await;
    run(&["install", "--user", "--noninteractive", "--assumeyes", "flathub", id]).await
}

/// Uninstall `id`, wherever it is installed (`flatpak` resolves the installation).
pub async fn uninstall(id: &str) -> Result<(), String> {
    run(&["uninstall", "--noninteractive", "--assumeyes", id]).await
}

/// Update `id` (any installation).
pub async fn update(id: &str) -> Result<(), String> {
    run(&["update", "--noninteractive", "--assumeyes", id]).await
}

/// Run `flatpak` with `args`; on failure return the most relevant stderr line.
async fn run(args: &[&str]) -> Result<(), String> {
    let out = tokio::process::Command::new("flatpak")
        .args(args)
        .output()
        .await
        .map_err(|e| format!("could not run flatpak: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(last_line(&out.stderr, "the operation failed"))
    }
}

fn last_line(stderr: &[u8], fallback: &str) -> String {
    String::from_utf8_lossy(stderr)
        .lines()
        .map(str::trim)
        .rfind(|l| !l.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

/// Launch an installed app, detached. Fire-and-forget.
pub fn launch(id: &str) {
    let _ = std::process::Command::new("flatpak").args(["run", id]).spawn();
}

/// Download a screenshot to the on-disk cache, returning its local path. Cached
/// hits skip the network. `--fail` keeps HTTP error pages out of the cache.
pub async fn fetch_screenshot(url: String) -> Option<PathBuf> {
    let path = cache_path(&url);
    if path.exists() {
        return Some(path);
    }
    if let Some(parent) = path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    let ok = tokio::process::Command::new("curl")
        .args(["-sfL", "--max-time", "25", "-o"])
        .arg(&path)
        .arg(&url)
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false);
    if ok && path.exists() {
        Some(path)
    } else {
        // Never leave a partial/failed body behind to poison the cache.
        let _ = tokio::fs::remove_file(&path).await;
        None
    }
}

fn cache_path(url: &str) -> PathBuf {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut h);
    let name = format!("{:016x}.img", h.finish());
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".cache/draftos-store/shots").join(name)
}
