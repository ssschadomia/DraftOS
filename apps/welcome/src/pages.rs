//! Page content for DraftOS Hello.
//!
//! Each onboarding step is a pure view function returning a libcosmic element.
//! Keeping content here (separate from the [`crate::app`] state machine) makes the
//! flow easy to extend: add a variant to [`Page`] and a matching view.

use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget;

use crate::app::Message;

/// The ordered steps of the welcome flow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Welcome,
    Highlights,
    Personalize,
    Finish,
}

impl Page {
    /// All pages in display order.
    pub const ALL: [Page; 4] = [Page::Welcome, Page::Highlights, Page::Personalize, Page::Finish];
}

/// A single highlighted feature (icon + title + one-line description).
struct Feature {
    icon: &'static str,
    title: &'static str,
    detail: &'static str,
}

const FEATURES: &[Feature] = &[
    Feature {
        icon: "preferences-desktop-wallpaper-symbolic",
        title: "A desktop with depth",
        detail: "COSMIC with real glass and a layout that stays out of your way.",
    },
    Feature {
        icon: "system-software-install-symbolic",
        title: "Curated, first-party apps",
        detail: "A store, control center and companion tools designed to feel like one system.",
    },
    Feature {
        icon: "power-profile-performance-symbolic",
        title: "Tuned for speed",
        detail: "A performance-first Arch base with the CachyOS kernel and repositories.",
    },
    Feature {
        icon: "security-high-symbolic",
        title: "Open and yours",
        detail: "The polish of a curated OS with the freedom of Linux underneath.",
    },
];

/// Hero greeting.
pub fn welcome<'a>() -> Element<'a, Message> {
    let logo = widget::icon::from_name("start-here-symbolic").size(112).icon();

    widget::column::with_capacity(4)
        .spacing(20)
        .align_x(Alignment::Center)
        .push(logo)
        .push(widget::text::title1("Welcome to DraftOS"))
        .push(
            widget::text::body(
                "A clean, modern Arch — the care of macOS, the freedom of Linux.\n\
                 Let's take a minute to get you set up.",
            )
            .center(),
        )
        .into()
}

/// Feature highlights as a vertical list of cards.
pub fn highlights<'a>() -> Element<'a, Message> {
    let mut list = widget::column::with_capacity(FEATURES.len() + 1)
        .spacing(12)
        .push(widget::text::title2("What makes DraftOS, DraftOS"));

    for f in FEATURES {
        let row = widget::row::with_capacity(2)
            .spacing(16)
            .align_y(Alignment::Center)
            .push(widget::icon::from_name(f.icon).size(40).icon())
            .push(
                widget::column::with_capacity(2)
                    .spacing(2)
                    .push(widget::text::heading(f.title))
                    .push(widget::text::caption(f.detail))
                    .width(Length::Fill),
            );
        list = list.push(
            widget::container(row)
                .padding(16)
                .width(Length::Fill)
                .class(cosmic::theme::Container::Card),
        );
    }

    list.into()
}

/// Placeholder personalization step — real toggles land once the settings backend
/// exists; for now it points at where personalization will live.
pub fn personalize<'a>() -> Element<'a, Message> {
    widget::column::with_capacity(3)
        .spacing(16)
        .push(widget::text::title2("Make it yours"))
        .push(widget::text::body(
            "Appearance, accent color and layout will live here — wired to COSMIC's \
             settings once the setup backend is in place.",
        ))
        .push(
            widget::container(
                widget::column::with_capacity(2)
                    .spacing(6)
                    .push(widget::text::heading("Coming next"))
                    .push(widget::text::caption(
                        "Light / dark, accent color, dock behavior, and curated app picks.",
                    )),
            )
            .padding(16)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card),
        )
        .into()
}

/// Closing step.
pub fn finish<'a>() -> Element<'a, Message> {
    widget::column::with_capacity(3)
        .spacing(20)
        .align_x(Alignment::Center)
        .push(widget::icon::from_name("emblem-ok-symbolic").size(96).icon())
        .push(widget::text::title1("You're all set"))
        .push(
            widget::text::body("Enjoy DraftOS. You can reopen this anytime from the app menu.")
                .center(),
        )
        .into()
}
