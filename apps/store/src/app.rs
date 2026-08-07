//! Application state, navigation and message handling for the App Center.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use cosmic::iced::Length;
use cosmic::prelude::*;
use cosmic::widget::{self, nav_bar};
use cosmic::{executor, Core};

use crate::catalog::{self, Catalog};
use crate::flatpak;
use crate::model::SECTIONS;
use crate::views;

/// Which content page is showing. Search is an overlay keyed off `search`, and
/// `Detail` always wins so a tapped card opens even mid-search.
#[derive(Clone)]
pub enum Page {
    Home,
    Section(String),
    Library,
    Updates,
    Detail(usize),
}

/// Load state of a single screenshot.
#[derive(Clone)]
pub enum Shot {
    Loading,
    Ready(PathBuf),
    Failed,
}

#[derive(Clone, Debug)]
pub enum Message {
    Loaded(Result<(Arc<Catalog>, HashSet<String>), String>),
    RefreshedInstalled(HashSet<String>),
    UpdatesLoaded(Result<HashSet<String>, String>),
    Search(String),
    OpenApp(usize),
    Back,
    GoSection(String),
    Install(String),
    Uninstall(String),
    Update(String),
    UpdateAll,
    Launch(String),
    OpDone(String, Result<(), String>),
    CheckUpdates,
    ShotDone(String, Option<PathBuf>),
    OpenUrl(String),
    /// Provision Flathub + sync AppStream, then reload — used when the catalog
    /// is missing (a freshly installed system that never synced it).
    Bootstrap,
    Reload,
}

pub struct Store {
    core: Core,
    nav: nav_bar::Model,
    pub catalog: Option<Arc<Catalog>>,
    pub load_error: Option<String>,
    /// True flatpak ids of installed apps (compare with `App::flatpak_id`).
    pub installed: HashSet<String>,
    /// `None` until first checked; then the set of ids with an update available.
    pub updates: Option<HashSet<String>>,
    pub updates_checking: bool,
    /// The update *check* failed (network/remotes) — distinct from "no updates".
    pub updates_error: Option<String>,
    /// Provisioning Flathub + syncing the catalog after a missing-catalog start.
    pub bootstrapping: bool,
    /// The most recent failed operation, surfaced as a banner.
    pub op_error: Option<String>,
    pub search: String,
    pub results: Vec<usize>,
    pub page: Page,
    return_to: Page,
    /// Flatpak ids with an install/uninstall/update in flight.
    pub busy: HashSet<String>,
    pub shots: HashMap<String, Shot>,
}

impl Store {
    /// Convenience for views: the catalog (empty-safe callers check `catalog`).
    pub fn catalog(&self) -> Option<&Catalog> {
        self.catalog.as_deref()
    }
}

impl cosmic::Application for Store {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "org.draftos.Store";

    fn core(&self) -> &Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: ()) -> (Self, cosmic::app::Task<Message>) {
        let mut nav = nav_bar::Model::default();
        nav.insert()
            .text("Discover")
            .icon(widget::icon::from_name("user-home-symbolic"))
            .data(Page::Home)
            .activate();
        for s in SECTIONS {
            nav.insert()
                .text(s.title)
                .icon(widget::icon::from_name(s.icon))
                .data(Page::Section(s.key.to_string()));
        }
        nav.insert()
            .text("Library")
            .icon(widget::icon::from_name("object-select-symbolic"))
            .data(Page::Library);
        nav.insert()
            .text("Updates")
            .icon(widget::icon::from_name("software-update-available-symbolic"))
            .data(Page::Updates);

        // Parse the ~48 MB AppStream catalog + read installed apps off-thread.
        let load = cosmic::task::future(async {
            let res = tokio::task::spawn_blocking(|| {
                catalog::load().map(|c| (Arc::new(c), flatpak::installed_ids()))
            })
            .await
            .unwrap_or_else(|e| Err(format!("catalog task failed: {e}")));
            Message::Loaded(res)
        });

        let store = Store {
            core,
            nav,
            catalog: None,
            load_error: None,
            installed: HashSet::new(),
            updates: None,
            updates_checking: false,
            updates_error: None,
            bootstrapping: false,
            op_error: None,
            search: String::new(),
            results: Vec::new(),
            page: Page::Home,
            return_to: Page::Home,
            busy: HashSet::new(),
            shots: HashMap::new(),
        };
        (store, load)
    }

    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    fn on_nav_select(&mut self, id: nav_bar::Id) -> cosmic::app::Task<Message> {
        self.nav.activate(id);
        if let Some(page) = self.nav.data::<Page>(id).cloned() {
            self.search.clear();
            self.page = page.clone();
            self.return_to = page;
            if matches!(self.page, Page::Updates) && self.updates.is_none() && !self.updates_checking {
                return self.check_updates();
            }
        }
        cosmic::app::Task::none()
    }

    fn header_start(&self) -> Vec<Element<'_, Message>> {
        if matches!(self.page, Page::Detail(_)) {
            vec![widget::button::text("‹  Back")
                .on_press(Message::Back)
                .into()]
        } else {
            Vec::new()
        }
    }

    fn header_center(&self) -> Vec<Element<'_, Message>> {
        vec![widget::search_input("Search apps and games", &self.search)
            .on_input(Message::Search)
            .width(Length::Fixed(460.0))
            .into()]
    }

    fn update(&mut self, message: Message) -> cosmic::app::Task<Message> {
        match message {
            Message::Loaded(Ok((cat, installed))) => {
                self.installed = installed;
                self.load_error = None;
                self.bootstrapping = false;
                // Dev hook: DRAFTOS_STORE_APP=<id> opens a detail page on load so
                // it can be previewed/screenshot without navigating.
                let open = std::env::var("DRAFTOS_STORE_APP")
                    .ok()
                    .and_then(|id| cat.index_of(&id).or_else(|| cat.index_of_flatpak(&id)));
                self.catalog = Some(cat);
                if let Some(idx) = open {
                    self.page = Page::Detail(idx);
                    self.return_to = Page::Home;
                    return self.load_shots(idx);
                }
            }
            Message::Loaded(Err(e)) => {
                self.load_error = Some(e);
                self.bootstrapping = false;
            }
            Message::Bootstrap => {
                self.bootstrapping = true;
                self.load_error = None;
                return cosmic::task::future(async {
                    // Add the user Flathub remote and pull the AppStream catalog.
                    let _ = tokio::process::Command::new("flatpak")
                        .args([
                            "remote-add", "--user", "--if-not-exists", "flathub",
                            "https://dl.flathub.org/repo/flathub.flatpakrepo",
                        ])
                        .status()
                        .await;
                    let _ = tokio::process::Command::new("flatpak")
                        .args(["update", "--user", "--appstream", "--noninteractive"])
                        .status()
                        .await;
                    Message::Reload
                });
            }
            Message::Reload => {
                return cosmic::task::future(async {
                    let res = tokio::task::spawn_blocking(|| {
                        catalog::load().map(|c| (Arc::new(c), flatpak::installed_ids()))
                    })
                    .await
                    .unwrap_or_else(|e| Err(format!("catalog task failed: {e}")));
                    Message::Loaded(res)
                });
            }
            Message::RefreshedInstalled(set) => self.installed = set,
            Message::UpdatesLoaded(result) => {
                self.updates_checking = false;
                match result {
                    Ok(set) => {
                        self.updates = Some(set);
                        self.updates_error = None;
                    }
                    Err(e) => self.updates_error = Some(e),
                }
            }
            Message::Search(q) => {
                self.search = q;
                self.recompute_search();
                // Leaving a detail page keeps search live via view()'s overlay,
                // but a search from within Detail should surface results.
                if matches!(self.page, Page::Detail(_)) && !self.search.trim().is_empty() {
                    self.page = self.return_to.clone();
                }
            }
            Message::OpenApp(idx) => {
                // A double-click delivers OpenApp twice; recording the second
                // one would make return_to = Detail(idx) and turn Back into a
                // self-loop.
                if !matches!(self.page, Page::Detail(i) if i == idx) {
                    self.return_to = self.page.clone();
                }
                self.page = Page::Detail(idx);
                return self.load_shots(idx);
            }
            Message::Back => self.page = self.return_to.clone(),
            Message::GoSection(key) => {
                self.search.clear();
                let p = Page::Section(key);
                self.select_nav(&p);
                self.page = p.clone();
                self.return_to = p;
            }
            Message::Install(id) => return self.op(id, Op::Install),
            Message::Uninstall(id) => return self.op(id, Op::Uninstall),
            Message::Update(id) => return self.op(id, Op::Update),
            Message::UpdateAll => {
                // Sequential on purpose: flatpak takes a per-installation lock,
                // so N parallel updates would just fail against each other.
                if let Some(set) = self.updates.clone() {
                    let mut ids: Vec<String> =
                        set.into_iter().filter(|id| !self.busy.contains(id)).collect();
                    ids.sort();
                    if ids.is_empty() {
                        return cosmic::app::Task::none();
                    }
                    self.busy.extend(ids.iter().cloned());
                    self.op_error = None;
                    return cosmic::task::stream(update_all_stream(ids));
                }
            }
            Message::OpDone(id, result) => {
                self.busy.remove(&id);
                match result {
                    Ok(()) => {
                        if let Some(set) = self.updates.as_mut() {
                            set.remove(&id);
                        }
                    }
                    Err(e) => self.op_error = Some(format!("{id}: {e}")),
                }
                // Re-read installed apps so buttons flip to the real state.
                return cosmic::task::future(async {
                    let set = tokio::task::spawn_blocking(flatpak::installed_ids)
                        .await
                        .unwrap_or_default();
                    Message::RefreshedInstalled(set)
                });
            }
            Message::CheckUpdates => return self.check_updates(),
            Message::ShotDone(url, path) => {
                self.shots.insert(
                    url,
                    match path {
                        Some(p) => Shot::Ready(p),
                        None => Shot::Failed,
                    },
                );
            }
            Message::Launch(id) => flatpak::launch(&id),
            Message::OpenUrl(url) => {
                let _ = std::process::Command::new("xdg-open").arg(url).spawn();
            }
        }
        cosmic::app::Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        if self.bootstrapping {
            return views::loading_msg("Setting up the App Center…", "Fetching the Flathub catalog");
        }
        if let Some(err) = &self.load_error {
            return views::bootstrap_page(err);
        }
        let Some(_) = self.catalog else {
            return views::loading();
        };
        match &self.page {
            Page::Detail(idx) => views::detail(self, *idx),
            _ if !self.search.trim().is_empty() => views::search(self),
            Page::Home => views::home(self),
            Page::Section(key) => views::section(self, key),
            Page::Library => views::library(self),
            Page::Updates => views::updates(self),
        }
    }
}

/// The kind of package operation in flight.
enum Op {
    Install,
    Uninstall,
    Update,
}

impl Store {
    fn recompute_search(&mut self) {
        let q = self.search.trim().to_lowercase();
        self.results = match (self.catalog(), q.is_empty()) {
            (Some(cat), false) => {
                let mut idx: Vec<usize> = cat
                    .apps
                    .iter()
                    .enumerate()
                    .filter(|(_, a)| a.matches(&q))
                    .map(|(i, _)| i)
                    .collect();
                idx.sort_by(|&a, &b| {
                    let (aa, bb) = (&cat.apps[a], &cat.apps[b]);
                    // exact name-start matches first, then art, then A→Z
                    let sa = aa.name.to_lowercase().starts_with(&q);
                    let sb = bb.name.to_lowercase().starts_with(&q);
                    sb.cmp(&sa)
                        .then(bb.has_art().cmp(&aa.has_art()))
                        .then(aa.name.to_lowercase().cmp(&bb.name.to_lowercase()))
                });
                idx.truncate(120);
                idx
            }
            _ => Vec::new(),
        };
    }

    fn select_nav(&mut self, page: &Page) {
        let want = page_key(page);
        let ids: Vec<_> = self.nav.iter().collect();
        for id in ids {
            if self.nav.data::<Page>(id).map(page_key) == Some(want.clone()) {
                self.nav.activate(id);
                break;
            }
        }
    }

    fn op(&mut self, id: String, op: Op) -> cosmic::app::Task<Message> {
        if self.busy.contains(&id) {
            return cosmic::app::Task::none();
        }
        self.busy.insert(id.clone());
        self.op_error = None;
        cosmic::task::future(async move {
            let result = match op {
                Op::Install => flatpak::install(&id).await,
                Op::Uninstall => flatpak::uninstall(&id).await,
                Op::Update => flatpak::update(&id).await,
            };
            Message::OpDone(id, result)
        })
    }

    fn check_updates(&mut self) -> cosmic::app::Task<Message> {
        self.updates_checking = true;
        self.updates_error = None;
        cosmic::task::future(async {
            let result = tokio::task::spawn_blocking(flatpak::updatable_ids)
                .await
                .unwrap_or_else(|e| Err(format!("update check crashed: {e}")));
            Message::UpdatesLoaded(result)
        })
    }

    /// Kick off downloads for an app's screenshots that aren't cached yet.
    fn load_shots(&mut self, idx: usize) -> cosmic::app::Task<Message> {
        let Some(cat) = self.catalog() else {
            return cosmic::app::Task::none();
        };
        let urls = cat.apps[idx].screenshots.clone();
        let mut tasks = Vec::new();
        for url in urls {
            if self.shots.contains_key(&url) {
                continue;
            }
            self.shots.insert(url.clone(), Shot::Loading);
            let u = url.clone();
            tasks.push(cosmic::task::future(async move {
                let path = flatpak::fetch_screenshot(u.clone()).await;
                Message::ShotDone(u, path)
            }));
        }
        cosmic::app::Task::batch(tasks)
    }
}

/// Update the given apps one at a time (flatpak holds a per-installation lock),
/// emitting an [`Message::OpDone`] after each so the UI updates progressively.
fn update_all_stream(ids: Vec<String>) -> impl cosmic::iced::futures::Stream<Item = Message> {
    use cosmic::iced::futures::SinkExt;
    cosmic::iced::stream::channel(
        8,
        move |mut output: cosmic::iced::futures::channel::mpsc::Sender<Message>| async move {
            for id in ids {
                let result = flatpak::update(&id).await;
                let _ = output.send(Message::OpDone(id, result)).await;
            }
        },
    )
}

/// A stable key for comparing `Page`s (Detail is never a nav target).
fn page_key(p: &Page) -> String {
    match p {
        Page::Home => "home".into(),
        Page::Section(k) => format!("section:{k}"),
        Page::Library => "library".into(),
        Page::Updates => "updates".into(),
        Page::Detail(_) => "detail".into(),
    }
}
