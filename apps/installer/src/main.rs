//! DraftOS installer — a libcosmic wizard that installs the Desktop edition.
//!
//! The flow is deliberately simple and System76/Pop!_OS-like: one decision per
//! screen. This binary is the UI only; the system-changing work (partitioning,
//! package install, bootloader) will live behind a separate engine crate so the
//! wizard never touches the disk directly.

mod app;
mod config;
mod steps;
mod system;

use cosmic::app::Settings;
use cosmic::iced::Size;

fn main() -> cosmic::iced::Result {
    let settings = Settings::default()
        .size(Size::new(920.0, 680.0))
        .size_limits(cosmic::iced::Limits::NONE.min_width(760.0).min_height(560.0));
    cosmic::app::run::<app::Installer>(settings, ())
}
