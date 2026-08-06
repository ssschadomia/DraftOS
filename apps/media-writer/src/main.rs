//! DraftOS Media Writer — write a DraftOS ISO to a USB drive.
//!
//! A small libcosmic app in the spirit of Fedora Media Writer: choose an ISO,
//! choose a removable drive, write it, done. Only removable drives are offered,
//! so the system disk can never be selected; the write runs via `pkexec`.

mod app;
mod system;

use cosmic::app::Settings;
use cosmic::iced::Size;

fn main() -> cosmic::iced::Result {
    let settings = Settings::default()
        .size(Size::new(720.0, 560.0))
        .size_limits(cosmic::iced::Limits::NONE.min_width(560.0).min_height(480.0));
    cosmic::app::run::<app::MediaWriter>(settings, ())
}
