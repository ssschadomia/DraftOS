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
    UpdatesLoaded(HashSet<String>),
    Search(String),
    OpenApp(usize),
    Back,
    GoSection(String),
    Install(String),
    Uninstall(String),
    Update(String),
    UpdateAll,
    Launch(String),
    OpDone(String, bool),
    CheckUpdates,
    ShotDone(String, Option<PathBuf>),
    OpenUrl(String),
}

pub struct Store {
    core: Core,
    nav: nav_bar::Model,
    pub catalog: Option<Arc<Catalog>>,
    pub load_error: Option<String>,
    pub installed: HashSet<String>,
    /// `None` until first checked; then the set of ids with an update available.
    pub updates: Option<HashSet<String>>,
    pub updates_checking: bool,
    pub search: String,
    pub results: Vec<usize>,
    pub page: Page,
    return_to: Page,
    /// Ids with an install/uninstall/update in flight.
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
                self.catalog = Some(cat);
                self.installed = installed;
            }
            Message::Loaded(Err(e)) => self.load_error = Some(e),
            Message::RefreshedInstalled(set) => self.installed = set,
            Message::UpdatesLoaded(set) => {
                self.updates = Some(set);
                self.updates_checking = false;
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
                self.return_to = self.page.clone();
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
                if let Some(ids) = self.updates.clone() {
                    let tasks: Vec<_> = ids.into_iter().map(|id| self.op(id, Op::Update)).collect();
                    return cosmic::app::Task::batch(tasks);
                }
            }
            Message::OpDone(id, ok) => {
                self.busy.remove(&id);
                if ok {
                    if let Some(set) = self.updates.as_mut() {
                        set.remove(&id);
                    }
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
        if let Some(err) = &self.load_error {
            return views::message_page("Couldn't load the catalog", err);
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
        self.busy.insert(id.clone());
        cosmic::task::future(async move {
            let ok = match op {
                Op::Install => flatpak::install(&id).await,
                Op::Uninstall => flatpak::uninstall(&id).await,
                Op::Update => flatpak::update(&id).await,
            };
            Message::OpDone(id, ok)
        })
    }

    fn check_updates(&mut self) -> cosmic::app::Task<Message> {
        self.updates_checking = true;
        cosmic::task::future(async {
            let set = tokio::task::spawn_blocking(flatpak::updatable_ids)
                .await
                .unwrap_or_default();
            Message::UpdatesLoaded(set)
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
