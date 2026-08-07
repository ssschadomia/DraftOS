//! DraftOS App Center — a Microsoft-Store-style storefront for Flatpak apps.
//!
//! A libcosmic application: a left nav rail of curated sections, a top search
//! bar, a Home page of featured/section shelves, per-app detail pages with
//! screenshots and one-click install, plus a Library and Updates view. The
//! catalog is the local Flatpak AppStream data (ADR 0006).

mod app;
mod catalog;
mod flatpak;
mod model;
mod views;

use cosmic::app::Settings;
use cosmic::iced::Size;

fn main() -> cosmic::iced::Result {
    let settings = Settings::default()
        .size(Size::new(1180.0, 820.0))
        .size_limits(cosmic::iced::Limits::NONE.min_width(900.0).min_height(600.0));
    cosmic::app::run::<app::Store>(settings, ())
}
