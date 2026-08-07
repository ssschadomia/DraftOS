//! View builders for the App Center. Every helper returns an owned `Element` to
//! keep libcosmic's renderer type inference happy.

use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::image::Handle as ImageHandle;
use cosmic::widget::{self};

use crate::app::{Message, Shot, Store};
use crate::model::{self, App, SECTIONS};

const CARD_W: f32 = 200.0;
/// How many cards a Home shelf shows (a wrapping flex row caps it visually too).
const SHELF_COLS: usize = 5;
/// Screenshots and wide cards scale to the window but never exceed this.
const CONTENT_MAX: f32 = 760.0;
/// The page content is capped to this width and centred, so on a wide window
/// the shelves fill their row instead of leaving a lopsided gap on the right
/// (5 cards × 200 + gaps + padding ≈ this). Narrower windows use full width and
/// the flex rows wrap.
const PAGE_MAX: f32 = 1128.0;

/// Full-page centered loading state while the catalog parses.
pub fn loading<'a>() -> Element<'a, Message> {
    loading_msg("Loading the App Center…", "Reading the Flathub catalog")
}

/// Full-page centered loading state with a custom message.
pub fn loading_msg<'a>(title: &'a str, body: &'a str) -> Element<'a, Message> {
    center(
        widget::column::with_capacity(2)
            .spacing(12)
            .align_x(Alignment::Center)
            .push(widget::text::title2(title.to_string()))
            .push(widget::text::body(body.to_string())),
    )
}

/// Shown when the catalog is missing (a fresh system that never synced it):
/// offer to provision Flathub and pull the catalog, rather than dead-ending.
pub fn bootstrap_page<'a>(err: &'a str) -> Element<'a, Message> {
    center(
        widget::column::with_capacity(4)
            .spacing(16)
            .max_width(520.0)
            .align_x(Alignment::Center)
            .push(widget::text::title2("Set up the App Center"))
            .push(
                widget::text::body(
                    "The Flathub app catalog isn't available yet. Set it up now to \
                     browse and install thousands of apps.",
                )
                .center(),
            )
            .push(widget::button::suggested("Set up the App Center").on_press(Message::Bootstrap))
            .push(widget::text::caption(err.to_string())),
    )
}

/// Full-page centered message (also used for errors).
pub fn message_page<'a>(title: &'a str, body: &'a str) -> Element<'a, Message> {
    center(
        widget::column::with_capacity(2)
            .spacing(12)
            .max_width(560.0)
            .align_x(Alignment::Center)
            .push(widget::text::title2(title))
            .push(widget::text::body(body)),
    )
}

// ---------------------------------------------------------------- Home --------

pub fn home(store: &Store) -> Element<'_, Message> {
    let cat = store.catalog().expect("home called with a catalog");
    let mut col = widget::column::with_capacity(16).spacing(28).padding(32);

    col = col.push(widget::text::title1("Discover"));

    // Hero: the first featured app that exists in this catalog.
    if let Some(idx) = model::FEATURED.iter().find_map(|id| cat.index_of(id)) {
        col = col.push(hero(store, idx));
    }

    // "Popular" shelf from the featured list.
    let popular: Vec<usize> = model::FEATURED
        .iter()
        .filter_map(|id| cat.index_of(id))
        .take(SHELF_COLS)
        .collect();
    if !popular.is_empty() {
        col = col.push(shelf(store, "Popular right now", None, &popular));
    }

    // One shelf per curated section.
    for s in SECTIONS {
        let idx = cat.in_categories(s.cats);
        if idx.is_empty() {
            continue;
        }
        let row: Vec<usize> = idx.iter().copied().take(SHELF_COLS).collect();
        col = col.push(shelf(store, s.title, Some(s.key), &row));
    }

    scroll(col)
}

fn hero(store: &Store, idx: usize) -> Element<'_, Message> {
    let app = &store.catalog().unwrap().apps[idx];
    let text = widget::column::with_capacity(4)
        .spacing(10)
        .push(widget::text::caption("FEATURED"))
        .push(widget::text::title1(app.name.clone()))
        .push(widget::text::body(app.summary.clone()))
        .push(widget::button::suggested("View").on_press(Message::OpenApp(idx)))
        .width(Length::Fill);

    let card = widget::container(
        widget::row::with_capacity(2)
            .spacing(28)
            .align_y(Alignment::Center)
            .push(text)
            .push(app_icon(app, 132)),
    )
    .padding(32)
    .width(Length::Fill)
    .class(cosmic::theme::Container::Card);

    card.into()
}

/// A titled horizontal shelf of app cards, with an optional "See all" link.
fn shelf<'a>(
    store: &'a Store,
    title: &'a str,
    section_key: Option<&'a str>,
    indices: &[usize],
) -> Element<'a, Message> {
    let mut header = widget::row::with_capacity(2)
        .align_y(Alignment::Center)
        .push(widget::text::title3(title))
        .push(widget::Space::new().width(Length::Fill));
    if let Some(key) = section_key {
        header = header.push(
            widget::button::text("See all")
                .on_press(Message::GoSection(key.to_string())),
        );
    }

    widget::column::with_capacity(2)
        .spacing(14)
        .push(header)
        .push(grid(store, indices))
        .into()
}

// ------------------------------------------------------------- Section --------

pub fn section<'a>(store: &'a Store, key: &'a str) -> Element<'a, Message> {
    let cat = store.catalog().unwrap();
    let Some(sec) = model::section(key) else {
        return message_page("Unknown section", key);
    };
    let indices = cat.in_categories(sec.cats);
    let shown = indices.len().min(96);

    let mut col = widget::column::with_capacity(3)
        .spacing(20)
        .padding(32)
        .push(widget::text::title1(sec.title));
    col = col.push(widget::text::body(format!(
        "{} apps{}",
        indices.len(),
        if indices.len() > shown {
            format!(" · showing the first {shown}")
        } else {
            String::new()
        }
    )));
    col = col.push(grid(store, &indices[..shown]));
    scroll(col)
}

/// A responsive grid of fixed-width cards — `flex_row` wraps them to as many
/// columns as the current window width allows, so nothing is ever clipped.
fn grid<'a>(store: &'a Store, indices: &[usize]) -> Element<'a, Message> {
    let cards: Vec<Element<'a, Message>> = indices.iter().map(|&i| app_card(store, i)).collect();
    widget::flex_row(cards)
        .spacing(16)
        .justify_items(Alignment::Start)
        .into()
}

// -------------------------------------------------------------- Search --------

pub fn search(store: &Store) -> Element<'_, Message> {
    let n = store.results.len();
    let mut col = widget::column::with_capacity(3)
        .spacing(20)
        .padding(32)
        .push(widget::text::title2(format!(
            "Results for \u{201c}{}\u{201d}",
            store.search.trim()
        )));
    if n == 0 {
        col = col.push(widget::text::body("No matching apps."));
    } else {
        col = col.push(grid(store, &store.results));
    }
    scroll(col)
}

// ------------------------------------------------------------- Library --------

pub fn library(store: &Store) -> Element<'_, Message> {
    let cat = store.catalog().unwrap();
    let mut ids: Vec<&String> = store.installed.iter().collect();
    ids.sort();

    let mut col = widget::column::with_capacity(ids.len() + 2)
        .spacing(20)
        .padding(32)
        .push(widget::text::title1("Library"))
        .push(widget::text::body(format!("{} installed apps", ids.len())));

    let mut list = widget::column::with_capacity(ids.len()).spacing(8);
    for id in ids {
        // `id` is a true flatpak id (from `flatpak list`); match on that index.
        list = list.push(match cat.index_of_flatpak(id) {
            Some(idx) => installed_row(store, idx),
            None => unknown_row(store, id),
        });
    }
    col = col.push(widget::container(list).class(cosmic::theme::Container::List));
    scroll(col)
}

fn installed_row(store: &Store, idx: usize) -> Element<'_, Message> {
    let app = &store.catalog().unwrap().apps[idx];
    let actions = widget::row::with_capacity(2)
        .spacing(8)
        .push(widget::button::standard("Open").on_press(Message::Launch(app.flatpak_id.clone())))
        .push(uninstall_button(store, &app.flatpak_id));
    list_row(app_icon(app, 44), &app.name, &app.summary, actions.into(), Some(idx))
}

fn unknown_row<'a>(store: &'a Store, id: &'a str) -> Element<'a, Message> {
    let actions = widget::row::with_capacity(2)
        .spacing(8)
        .push(widget::button::standard("Open").on_press(Message::Launch(id.to_string())))
        .push(uninstall_button(store, id));
    list_row(
        widget::icon::from_name("application-x-executable-symbolic").size(44).icon().into(),
        id,
        "Installed (not in the Flathub catalog)",
        actions.into(),
        None,
    )
}

// ------------------------------------------------------------- Updates --------

pub fn updates(store: &Store) -> Element<'_, Message> {
    let mut col = widget::column::with_capacity(4)
        .spacing(20)
        .padding(32)
        .push(widget::text::title1("Updates"));

    if store.updates_checking {
        col = col.push(widget::text::body("Checking for updates…"));
        return scroll(col);
    }
    if let Some(e) = &store.updates_error {
        col = col.push(error_banner(&format!("Couldn't check for updates: {e}")));
        col = col.push(
            widget::button::suggested("Try again").on_press(Message::CheckUpdates),
        );
        return scroll(col);
    }
    let Some(set) = &store.updates else {
        col = col.push(widget::text::body("Check Flathub for newer versions."));
        col = col.push(
            widget::button::suggested("Check for updates").on_press(Message::CheckUpdates),
        );
        return scroll(col);
    };
    if set.is_empty() {
        col = col.push(widget::text::body("Everything is up to date. 🎉"));
        return scroll(col);
    }

    let cat = store.catalog().unwrap();
    col = col.push(
        widget::row::with_capacity(2)
            .align_y(Alignment::Center)
            .push(widget::text::body(format!("{} updates available", set.len())))
            .push(widget::Space::new().width(Length::Fill))
            .push(widget::button::suggested("Update all").on_press(Message::UpdateAll)),
    );

    let mut ids: Vec<&String> = set.iter().collect();
    ids.sort();
    let mut list = widget::column::with_capacity(ids.len()).spacing(8);
    for id in ids {
        let app = cat.index_of_flatpak(id).map(|i| &cat.apps[i]);
        let (icon, name, summary): (Element<_>, String, String) = match app {
            Some(a) => (app_icon(a, 44), a.name.clone(), a.summary.clone()),
            None => (
                widget::icon::from_name("application-x-executable-symbolic").size(44).icon().into(),
                id.clone(),
                String::new(),
            ),
        };
        let action: Element<_> = if store.busy.contains(id) {
            widget::button::standard("Updating…").into()
        } else {
            widget::button::suggested("Update").on_press(Message::Update(id.clone())).into()
        };
        let row = widget::row::with_capacity(3)
            .spacing(16)
            .align_y(Alignment::Center)
            .padding(12)
            .push(icon)
            .push(
                widget::column::with_capacity(2)
                    .push(widget::text::heading(name))
                    .push(widget::text::caption(summary))
                    .width(Length::Fill),
            )
            .push(action);
        list = list.push(row);
    }
    col = col.push(widget::container(list).class(cosmic::theme::Container::List));
    scroll(col)
}

// -------------------------------------------------------------- Detail --------

pub fn detail(store: &Store, idx: usize) -> Element<'_, Message> {
    let app = &store.catalog().unwrap().apps[idx];

    // Header: icon + identity + primary action.
    let mut identity = widget::column::with_capacity(3)
        .spacing(6)
        .push(widget::text::title1(app.name.clone()));
    if let Some(dev) = &app.developer {
        identity = identity.push(widget::text::body(dev.clone()));
    }
    if !app.categories.is_empty() {
        identity = identity.push(widget::text::caption(app.categories.join(" · ")));
    }

    let header = widget::row::with_capacity(3)
        .spacing(24)
        .align_y(Alignment::Center)
        .push(app_icon(app, 112))
        .push(identity.width(Length::Fill))
        .push(action_column(store, app));

    let mut col = widget::column::with_capacity(8)
        .spacing(24)
        .padding(32)
        .push(header);

    // Screenshots, each stacked full-width.
    let mut shots = widget::column::with_capacity(app.screenshots.len()).spacing(16);
    let mut any = false;
    for url in &app.screenshots {
        match store.shots.get(url) {
            Some(Shot::Ready(path)) => {
                any = true;
                shots = shots.push(
                    widget::container(
                        widget::image(ImageHandle::from_path(path)).width(Length::Fill),
                    )
                    .max_width(CONTENT_MAX)
                    .center_x(Length::Fill),
                );
            }
            Some(Shot::Loading) | None => {
                shots = shots.push(shot_placeholder("Loading screenshot…"));
            }
            Some(Shot::Failed) => {}
        }
    }
    if any || !app.screenshots.is_empty() {
        col = col.push(shots);
    }

    // Description.
    if !app.description.is_empty() {
        col = col.push(
            widget::container(widget::text::body(app.description.clone()))
                .padding(20)
                .width(Length::Fill)
                .class(cosmic::theme::Container::Card),
        );
    }

    // Metadata card.
    col = col.push(metadata(app));

    scroll(col)
}

fn action_column<'a>(store: &'a Store, app: &'a App) -> Element<'a, Message> {
    let fid = &app.flatpak_id;
    let installed = store.installed.contains(fid);
    let busy = store.busy.contains(fid);

    let primary: Element<_> = if busy {
        widget::button::standard("Working…").into()
    } else if installed {
        widget::row::with_capacity(2)
            .spacing(8)
            .push(widget::button::suggested("Open").on_press(Message::Launch(fid.clone())))
            .push(uninstall_button(store, fid))
            .into()
    } else {
        widget::button::suggested("Get").on_press(Message::Install(fid.clone())).into()
    };

    let mut col = widget::column::with_capacity(3)
        .spacing(8)
        .align_x(Alignment::End)
        .push(primary);
    if let Some(v) = &app.version {
        col = col.push(widget::text::caption(format!("v{v}")));
    }
    // Surface the last failed op right where the user acted.
    if let Some(e) = &store.op_error {
        if e.starts_with(fid.as_str()) {
            col = col.push(widget::text::caption(e.clone()));
        }
    }
    col.into()
}

fn metadata<'a>(app: &'a App) -> Element<'a, Message> {
    let mut col = widget::column::with_capacity(6).spacing(10);
    col = col.push(widget::text::title3("Details"));
    col = col.push(meta_row("Version", app.version.as_deref().unwrap_or("—")));
    col = col.push(meta_row("License", app.license.as_deref().unwrap_or("—")));
    col = col.push(meta_row(
        "Developer",
        app.developer.as_deref().unwrap_or("—"),
    ));
    col = col.push(meta_row("App ID", &app.id));
    if let Some(home) = &app.homepage {
        col = col.push(
            widget::button::link("Visit website").on_press(Message::OpenUrl(home.clone())),
        );
    }
    widget::container(col)
        .padding(20)
        .width(Length::Fill)
        .class(cosmic::theme::Container::Card)
        .into()
}

fn meta_row<'a>(label: &'a str, value: &'a str) -> Element<'a, Message> {
    widget::row::with_capacity(2)
        .push(widget::text::body(label).width(Length::Fixed(120.0)))
        .push(widget::text::body(value.to_string()).width(Length::Fill))
        .into()
}

// ----------------------------------------------------------- primitives -------

/// A clickable app card: icon, name, one-line summary, optional "Installed" tag.
fn app_card(store: &Store, idx: usize) -> Element<'_, Message> {
    let app = &store.catalog().unwrap().apps[idx];
    let mut col = widget::column::with_capacity(3)
        .spacing(8)
        .align_x(Alignment::Center)
        .push(app_icon(app, 88))
        .push(widget::text::heading(truncate(&app.name, 26)).center());
    if store.installed.contains(&app.flatpak_id) {
        col = col.push(widget::text::caption("Installed"));
    } else {
        col = col.push(widget::text::caption(truncate(&app.summary, 48)).center());
    }

    let card = widget::container(col)
        .padding(16)
        .width(Length::Fixed(CARD_W))
        .height(Length::Fixed(190.0))
        .class(cosmic::theme::Container::Card);

    widget::mouse_area(card)
        .on_press(Message::OpenApp(idx))
        .into()
}

fn app_icon<'a>(app: &App, size: u16) -> Element<'a, Message> {
    match &app.icon {
        Some(path) => widget::image(ImageHandle::from_path(path))
            .width(Length::Fixed(size as f32))
            .height(Length::Fixed(size as f32))
            .into(),
        None => widget::icon::from_name("application-x-executable-symbolic")
            .size(size)
            .icon()
            .into(),
    }
}

fn uninstall_button<'a>(store: &Store, id: &str) -> Element<'a, Message> {
    if store.busy.contains(id) {
        widget::button::standard("Working…").into()
    } else {
        widget::button::standard("Uninstall")
            .on_press(Message::Uninstall(id.to_string()))
            .into()
    }
}

fn list_row<'a>(
    icon: Element<'a, Message>,
    name: &str,
    summary: &str,
    actions: Element<'a, Message>,
    open: Option<usize>,
) -> Element<'a, Message> {
    let body = widget::column::with_capacity(2)
        .push(widget::text::heading(name.to_string()))
        .push(widget::text::caption(summary.to_string()))
        .width(Length::Fill);
    let row = widget::row::with_capacity(3)
        .spacing(16)
        .align_y(Alignment::Center)
        .padding(12)
        .push(icon)
        .push(body)
        .push(actions);
    match open {
        Some(idx) => widget::mouse_area(row).on_press(Message::OpenApp(idx)).into(),
        None => row.into(),
    }
}

fn shot_placeholder<'a>(label: &'a str) -> Element<'a, Message> {
    widget::container(
        widget::container(widget::text::caption(label))
            .center_x(Length::Fill)
            .center_y(Length::Fixed(200.0))
            .max_width(CONTENT_MAX)
            .width(Length::Fill)
            .height(Length::Fixed(200.0))
            .class(cosmic::theme::Container::Card),
    )
    .center_x(Length::Fill)
    .into()
}

/// An inline error card for a recoverable failure.
fn error_banner<'a>(msg: &str) -> Element<'a, Message> {
    widget::container(widget::text::body(msg.to_string()))
        .padding(16)
        .width(Length::Fill)
        .class(cosmic::theme::Container::Card)
        .into()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}

fn scroll<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    // Cap the content width and centre it: balanced margins on wide windows,
    // full width (with wrapping flex rows) on narrow ones.
    let capped = widget::container(widget::container(content).max_width(PAGE_MAX))
        .center_x(Length::Fill);
    widget::scrollable(capped).height(Length::Fill).into()
}

fn center<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    widget::container(content)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}
