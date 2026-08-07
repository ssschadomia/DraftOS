//! Load the Flatpak AppStream catalog into an in-memory [`Catalog`].
//!
//! Flatpak keeps a synced AppStream document per remote on disk; we parse it once
//! (in a background task) and resolve icons from the sibling icon cache. No
//! network is needed to render the storefront — only to install and to fetch
//! screenshots on demand.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use roxmltree::Node;

use crate::model::App;

/// The parsed catalog plus an id index.
#[derive(Debug)]
pub struct Catalog {
    pub apps: Vec<App>,
    by_id: HashMap<String, usize>,
}

impl Catalog {
    pub fn get(&self, id: &str) -> Option<&App> {
        self.by_id.get(id).map(|&i| &self.apps[i])
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.by_id.get(id).copied()
    }

    /// Indices of apps in any of the given AppStream categories, art-first then A→Z.
    pub fn in_categories(&self, cats: &[&str]) -> Vec<usize> {
        let mut idx: Vec<usize> = self
            .apps
            .iter()
            .enumerate()
            .filter(|(_, a)| a.in_categories(cats))
            .map(|(i, _)| i)
            .collect();
        idx.sort_by(|&a, &b| art_then_name(&self.apps[a], &self.apps[b]));
        idx
    }
}

fn art_then_name(a: &App, b: &App) -> std::cmp::Ordering {
    b.has_art()
        .cmp(&a.has_art())
        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
}

/// Parse the catalog. Cheap enough to call off the UI thread once at startup.
pub fn load() -> Result<Catalog, String> {
    let base = discover_base()
        .ok_or_else(|| "No Flatpak AppStream catalog found. Is Flathub set up?".to_string())?;
    let xml = read_catalog_xml(&base)?;
    let doc = roxmltree::Document::parse(&xml).map_err(|e| format!("catalog parse error: {e}"))?;
    let icons = base.join("icons");

    let mut apps = Vec::new();
    for node in doc
        .root_element()
        .children()
        .filter(|n| n.has_tag_name("component"))
    {
        if node.attribute("type") != Some("desktop-application") {
            continue;
        }
        if let Some(app) = parse_component(node, &icons) {
            apps.push(app);
        }
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    let by_id = apps
        .iter()
        .enumerate()
        .map(|(i, a)| (a.id.clone(), i))
        .collect();
    Ok(Catalog { apps, by_id })
}

/// Find the `active` catalog dir of a configured remote (Flathub preferred).
fn discover_base() -> Option<PathBuf> {
    let arch = "x86_64";
    let home = std::env::var("HOME").unwrap_or_default();
    let roots = [
        "/var/lib/flatpak/appstream".to_string(),
        format!("{home}/.local/share/flatpak/appstream"),
    ];
    // Prefer flathub, then any other remote that has a catalog.
    for remote in ["flathub"] {
        for root in &roots {
            let p = PathBuf::from(root).join(remote).join(arch).join("active");
            if has_catalog(&p) {
                return Some(p);
            }
        }
    }
    for root in &roots {
        if let Ok(rd) = std::fs::read_dir(root) {
            for e in rd.flatten() {
                let p = e.path().join(arch).join("active");
                if has_catalog(&p) {
                    return Some(p);
                }
            }
        }
    }
    None
}

fn has_catalog(dir: &Path) -> bool {
    dir.join("appstream.xml").exists() || dir.join("appstream.xml.gz").exists()
}

fn read_catalog_xml(base: &Path) -> Result<String, String> {
    let plain = base.join("appstream.xml");
    if plain.exists() {
        return std::fs::read_to_string(&plain).map_err(|e| format!("read catalog: {e}"));
    }
    let bytes = std::fs::read(base.join("appstream.xml.gz")).map_err(|e| format!("read catalog: {e}"))?;
    let mut s = String::new();
    flate2::read::GzDecoder::new(&bytes[..])
        .read_to_string(&mut s)
        .map_err(|e| format!("gunzip catalog: {e}"))?;
    Ok(s)
}

/// True for a child element in the untranslated (default) locale.
fn is_default_lang(n: &Node) -> bool {
    !n.attributes().any(|a| a.name() == "lang")
}

/// Text of the first default-locale child with `tag`.
fn child_text(parent: Node, tag: &str) -> Option<String> {
    parent
        .children()
        .find(|n| n.has_tag_name(tag) && is_default_lang(n))
        .and_then(|n| n.text())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn parse_component(node: Node, icons: &Path) -> Option<App> {
    let id = child_text(node, "id")?;
    let name = child_text(node, "name").unwrap_or_else(|| id.clone());
    let summary = child_text(node, "summary").unwrap_or_default();
    let description = node
        .children()
        .find(|n| n.has_tag_name("description") && is_default_lang(n))
        .map(description_text)
        .unwrap_or_default();

    let developer = child_text(node, "developer_name").or_else(|| {
        node.children()
            .find(|n| n.has_tag_name("developer"))
            .and_then(|d| child_text(d, "name"))
    });
    let license = child_text(node, "project_license");
    let homepage = node
        .children()
        .find(|n| n.has_tag_name("url") && n.attribute("type") == Some("homepage"))
        .and_then(|n| n.text())
        .map(str::to_string);

    let version = node
        .children()
        .find(|n| n.has_tag_name("releases"))
        .and_then(|r| r.children().find(|n| n.has_tag_name("release")))
        .and_then(|rel| rel.attribute("version").map(str::to_string));

    let categories = collect(node, "categories", "category");
    let keywords = node
        .children()
        .find(|n| n.has_tag_name("keywords") && is_default_lang(n))
        .map(|k| {
            k.children()
                .filter(|n| n.has_tag_name("keyword"))
                .filter_map(|n| n.text())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let icon = resolve_icon(&id, icons);
    let screenshots = collect_screenshots(node);

    Some(App {
        id,
        name,
        summary,
        description,
        developer,
        license,
        homepage,
        version,
        categories,
        keywords,
        icon,
        screenshots,
    })
}

/// Texts of `<wrapper><item>…</item></wrapper>` default-locale children.
fn collect(node: Node, wrapper: &str, item: &str) -> Vec<String> {
    node.children()
        .find(|n| n.has_tag_name(wrapper))
        .map(|w| {
            w.children()
                .filter(|n| n.has_tag_name(item))
                .filter_map(|n| n.text())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Flatten `<description>` (paragraphs + bulleted list items) into plain text.
fn description_text(desc: Node) -> String {
    let mut out = String::new();
    for child in desc.children() {
        if child.has_tag_name("p") && is_default_lang(&child) {
            if let Some(t) = child.text() {
                let t = normalize(t);
                if !t.is_empty() {
                    out.push_str(&t);
                    out.push_str("\n\n");
                }
            }
        } else if (child.has_tag_name("ul") || child.has_tag_name("ol")) && is_default_lang(&child) {
            for li in child.children().filter(|n| n.has_tag_name("li")) {
                if let Some(t) = li.text() {
                    let t = normalize(t);
                    if !t.is_empty() {
                        out.push_str("•  ");
                        out.push_str(&t);
                        out.push('\n');
                    }
                }
            }
            out.push('\n');
        }
    }
    out.trim().to_string()
}

/// Collapse internal whitespace/newlines from wrapped XML text into one line.
fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// One representative thumbnail URL per screenshot (largest ≤ 800px wide), max 4.
fn collect_screenshots(node: Node) -> Vec<String> {
    let Some(shots) = node.children().find(|n| n.has_tag_name("screenshots")) else {
        return Vec::new();
    };
    let mut urls = Vec::new();
    for shot in shots.children().filter(|n| n.has_tag_name("screenshot")) {
        let mut best: Option<(u32, String)> = None;
        for img in shot.children().filter(|n| n.has_tag_name("image")) {
            let url = match img.text() {
                Some(u) => u.trim().to_string(),
                None => continue,
            };
            let w: u32 = img.attribute("width").and_then(|w| w.parse().ok()).unwrap_or(0);
            let is_thumb = img.attribute("type") == Some("thumbnail");
            // Prefer thumbnails up to ~800px; fall back to source if nothing else.
            let score = if is_thumb && w <= 800 { w } else if best.is_none() { 1 } else { 0 };
            if score > 0 && best.as_ref().map(|(bw, _)| score > *bw).unwrap_or(true) {
                best = Some((score, url));
            }
        }
        if let Some((_, url)) = best {
            urls.push(url);
        }
        if urls.len() >= 4 {
            break;
        }
    }
    urls
}

fn resolve_icon(id: &str, icons: &Path) -> Option<PathBuf> {
    for size in ["128x128", "64x64"] {
        let p = icons.join(size).join(format!("{id}.png"));
        if p.exists() {
            return Some(p);
        }
    }
    None
}
