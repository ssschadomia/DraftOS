//! Catalog data model and the curated sections that shape the storefront.

use std::path::PathBuf;

/// One installable application, distilled from an AppStream `<component>`.
#[derive(Clone, Debug)]
pub struct App {
    /// AppStream component id, e.g. `org.telegram.desktop` — keys icons and
    /// catalog lookups. NOT always the flatpak ref id (legacy components carry a
    /// `.desktop` suffix): use [`App::flatpak_id`] for install/compare/launch.
    pub id: String,
    /// The true flatpak application id, parsed from the `<bundle>` ref.
    pub flatpak_id: String,
    pub name: String,
    pub summary: String,
    /// Plain-text description (paragraphs joined, list items bulleted).
    pub description: String,
    pub developer: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub version: Option<String>,
    /// Raw AppStream categories (e.g. `Office`, `Game`).
    pub categories: Vec<String>,
    pub keywords: Vec<String>,
    /// Local icon file from the AppStream icon cache, if present.
    pub icon: Option<PathBuf>,
    /// Screenshot thumbnail URLs (Flathub CDN), best-size first.
    pub screenshots: Vec<String>,
}

impl App {
    /// Does this app belong to any of the given AppStream categories?
    pub fn in_categories(&self, cats: &[&str]) -> bool {
        self.categories.iter().any(|c| cats.contains(&c.as_str()))
    }

    /// A cheap relevance signal for search/sorting: apps with art first.
    pub fn has_art(&self) -> bool {
        self.icon.is_some()
    }

    /// Case-insensitive match against name, summary, id and keywords.
    pub fn matches(&self, needle: &str) -> bool {
        let n = needle.to_lowercase();
        self.name.to_lowercase().contains(&n)
            || self.summary.to_lowercase().contains(&n)
            || self.id.to_lowercase().contains(&n)
            || self.keywords.iter().any(|k| k.to_lowercase().contains(&n))
    }
}

/// A curated storefront section — one nav entry and one home shelf. Each maps to
/// one or more raw AppStream categories.
pub struct Section {
    pub key: &'static str,
    pub title: &'static str,
    pub icon: &'static str,
    pub cats: &'static [&'static str],
}

/// The sections shown in the nav rail and on Home, in order.
pub const SECTIONS: &[Section] = &[
    Section { key: "productivity", title: "Productivity", icon: "text-editor-symbolic", cats: &["Office"] },
    Section { key: "creativity", title: "Creativity", icon: "applications-graphics-symbolic", cats: &["Graphics", "AudioVideo", "Audio", "Video"] },
    Section { key: "games", title: "Games", icon: "applications-games-symbolic", cats: &["Game"] },
    Section { key: "development", title: "Development", icon: "applications-engineering-symbolic", cats: &["Development"] },
    Section { key: "social", title: "Social", icon: "user-available-symbolic", cats: &["Network", "InstantMessaging", "Chat"] },
    Section { key: "science", title: "Education & Science", icon: "accessories-dictionary-symbolic", cats: &["Education", "Science"] },
    Section { key: "utilities", title: "Utilities", icon: "applications-utilities-symbolic", cats: &["Utility", "System"] },
];

/// Look up a section by its key.
pub fn section(key: &str) -> Option<&'static Section> {
    SECTIONS.iter().find(|s| s.key == key)
}

/// Hand-picked flagships for the Home hero + "Popular" shelf. Only those actually
/// present in the (possibly filtered) catalog are shown.
pub const FEATURED: &[&str] = &[
    "org.blender.Blender",
    "org.mozilla.firefox",
    "org.videolan.VLC",
    "com.obsproject.Studio",
    "org.gimp.GIMP",
    "org.kde.krita",
    "org.inkscape.Inkscape",
    "org.telegram.desktop",
    "org.libreoffice.LibreOffice",
    "org.audacityteam.Audacity",
    "org.kde.kdenlive",
    "org.signal.Signal",
    "io.github.shiftey.Desktop",
    "md.obsidian.Obsidian",
    "com.spotify.Client",
    "org.gnome.Boxes",
];
