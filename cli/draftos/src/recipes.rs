//! Discoverable task recipes — DraftOS's answer to Bazzite's `ujust`.
//!
//! A recipe is a small TOML file describing a named, documented sequence of
//! shell commands. They live in `/usr/share/draftos/recipes/` (shipped by
//! packages) and `~/.config/draftos/recipes/` (user overrides/additions), so
//! `draftos do` doubles as living, runnable documentation.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// One task recipe, deserialized from a `.toml` file.
#[derive(Debug, Deserialize)]
pub struct Recipe {
    /// Short identifier used as `draftos do <name>` (defaults to the file stem).
    #[serde(default)]
    pub name: String,
    /// One-line human description shown in the listing.
    pub description: String,
    /// Whether the commands need root (they're run through `sudo` when needed).
    #[serde(default)]
    pub root: bool,
    /// The shell commands, run in order; any failure stops the recipe.
    pub commands: Vec<String>,
}

/// The directories searched for recipes, most-general first (user dir wins on
/// name collisions).
pub fn search_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/usr/share/draftos/recipes")];
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".config/draftos/recipes"));
    }
    // Dev override: run against the in-repo recipes without installing.
    if let Some(dev) = std::env::var_os("DRAFTOS_RECIPES_DIR") {
        dirs.push(PathBuf::from(dev));
    }
    dirs
}

/// Load every recipe found, keyed by name (later dirs override earlier ones).
pub fn load_all() -> Vec<Recipe> {
    let mut by_name: std::collections::BTreeMap<String, Recipe> = Default::default();
    for dir in search_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            if let Some(r) = load_one(&path) {
                by_name.insert(r.name.clone(), r);
            }
        }
    }
    by_name.into_values().collect()
}

/// Load a single recipe file, defaulting `name` to the file stem.
fn load_one(path: &Path) -> Option<Recipe> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut recipe: Recipe = toml::from_str(&text)
        .map_err(|e| eprintln!("draftos: skipping {}: {e}", path.display()))
        .ok()?;
    if recipe.name.is_empty() {
        recipe.name = path.file_stem()?.to_string_lossy().into_owned();
    }
    Some(recipe)
}

/// Find one recipe by name.
pub fn find(name: &str) -> Option<Recipe> {
    load_all().into_iter().find(|r| r.name == name)
}
