//! DraftOS Hello — the first-run welcome and onboarding app.
//!
//! A small, paginated libcosmic application that greets the user, highlights what
//! makes DraftOS distinctive, and hands off to the rest of first-run setup. It is
//! the reference for the DraftOS visual language: generous spacing, a clear
//! hierarchy, and COSMIC's native glass.

mod app;
mod pages;

use cosmic::app::Settings;
use cosmic::iced::Size;

fn main() -> cosmic::iced::Result {
    let settings = Settings::default()
        .size(Size::new(880.0, 640.0))
        .size_limits(cosmic::iced::Limits::NONE.min_width(720.0).min_height(560.0));
    cosmic::app::run::<app::Welcome>(settings, ())
}
