//! Thin async wrapper over the `flatpak` CLI + a screenshot cache.
//!
//! Everyday actions target the **user** installation (`--user`) so no root prompt
//! is needed while browsing and installing. The store never links libflatpak; it
//! drives the same tool a user would, keeping the UI decoupled from the package
//! machinery (mirrors the install-engine split, ADR 0005).

use std::collections::HashSet;
use std::path::PathBuf;

/// Ids of every installed app (user + system).
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

/// Ids that have an update available on their remote (needs network; may be slow).
pub fn updatable_ids() -> HashSet<String> {
    let out = std::process::Command::new("flatpak")
        .args(["remote-ls", "--updates", "--app", "--columns=application"])
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

/// Install `id` from Flathub into the user installation. Returns success.
///
/// The user installation may have no Flathub remote yet (fresh systems, or hosts
/// where Flathub is only a filtered system remote), so the store is
/// self-sufficient: it idempotently adds the user-level Flathub remote first.
pub async fn install(id: &str) -> bool {
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

/// Uninstall `id` (user installation). Returns success.
pub async fn uninstall(id: &str) -> bool {
    run(&["uninstall", "--user", "--noninteractive", "--assumeyes", id]).await
}

/// Update `id` (user installation). Returns success.
pub async fn update(id: &str) -> bool {
    run(&["update", "--user", "--noninteractive", "--assumeyes", id]).await
}

async fn run(args: &[&str]) -> bool {
    tokio::process::Command::new("flatpak")
        .args(args)
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Launch an installed app, detached. Fire-and-forget.
pub fn launch(id: &str) {
    let _ = std::process::Command::new("flatpak").args(["run", id]).spawn();
}

/// Download a screenshot to the on-disk cache, returning its local path. Cached
/// hits skip the network.
pub async fn fetch_screenshot(url: String) -> Option<PathBuf> {
    let path = cache_path(&url);
    if path.exists() {
        return Some(path);
    }
    if let Some(parent) = path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    let ok = tokio::process::Command::new("curl")
        .args(["-sL", "--max-time", "25", "-o"])
        .arg(&path)
        .arg(&url)
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false);
    if ok && path.exists() {
        Some(path)
    } else {
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
