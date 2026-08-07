//! The install wizard: state, flow control, and per-step views.

use std::time::Duration;

use cosmic::iced::{Alignment, Length, Padding, Subscription};
use cosmic::prelude::*;
use cosmic::widget;
use cosmic::{executor, Core};

use crate::config::{InstallConfig, InstallType, KEYBOARD_SWITCHES};
use crate::locale_names;
use crate::steps::Step;
use crate::system::{self, DiskInfo, PartInfo};

/// Wizard events.
#[derive(Clone, Debug)]
pub enum Message {
    Next,
    Back,
    Close,
    /// Reboot the machine (Restart button on the Done step).
    Reboot,
    /// Timer tick that drives the install progress animation.
    Tick,
    /// Real progress streamed from the engine: (percent, current step title).
    InstallProgress(u32, String),
    /// The install engine finished (Ok) or failed with a message (Err).
    InstallFinished(Result<(), String>),
    SelectLocale(usize),
    SetLocaleSearch(String),
    AddLayout(String),
    RemoveLayout(usize),
    MoveLayoutUp(usize),
    MoveLayoutDown(usize),
    SetLayoutSearch(String),
    SetKeyboardSwitch(usize),
    SetKeyboardTest(String),
    SelectTimezone(usize),
    SetTimezoneSearch(String),
    SelectInstallType(InstallType),
    SelectDisk(usize),
    SetRootPartition(usize),
    SetEfiPartition(usize),
    ToggleEncrypt(bool),
    SetLuksPassword(String),
    SetLuksConfirm(String),
    SetFullName(String),
    SetUsername(String),
    SetHostname(String),
    SetPassword(String),
    SetPasswordConfirm(String),
    ToggleRootSame(bool),
    SetRootPassword(String),
    SetRootConfirm(String),
}

/// The DraftOS installer.
pub struct Installer {
    core: Core,
    /// Index into [`Step::ALL`].
    step: usize,
    /// Choices collected so far.
    config: InstallConfig,
    /// Locales known to the system, offered on the Language step.
    locales: Vec<String>,
    /// Index into [`Installer::locales`] of the chosen locale.
    selected_locale: Option<usize>,
    /// Current filter text on the Language step.
    locale_search: String,
    /// All XKB layouts as (code, description), offered on the Keyboard step.
    layouts: Vec<(String, String)>,
    /// Current filter text on the Keyboard step's "add layout" list.
    layout_search: String,
    /// Scratch text for the keyboard test field.
    kbd_test: String,
    /// Time zones known to the system, offered on the Timezone step.
    timezones: Vec<String>,
    /// Index into [`Installer::timezones`] of the chosen zone.
    selected_tz: Option<usize>,
    /// Current filter text on the Timezone step.
    tz_search: String,
    /// Whole disks detected on the host, offered on the Disk step.
    disks: Vec<DiskInfo>,
    /// Index into [`Installer::disks`] of the chosen target.
    selected_disk: Option<usize>,
    /// Partitions on the host, offered on the manual-partitioning Disk step.
    partitions: Vec<PartInfo>,
    /// LUKS passphrase re-entry, UI-only, to check it matches.
    luks_confirm: String,
    /// Password re-entry, kept in the UI only to check it matches.
    password_confirm: String,
    /// Root password re-entry, UI-only, used when root has a separate password.
    root_confirm: String,
    /// Install progress, 0.0..=1.0.
    install_progress: f32,
    /// The engine's plan for the current install, as (phase label, step title).
    plan_steps: Vec<(String, String)>,
    /// Index of the step currently shown on the Install screen.
    install_step: usize,
    /// Set when the Summary → Install transition can't build a valid request,
    /// or when the engine reports the install failed.
    summary_error: Option<String>,
    /// Live medium booted in BIOS/legacy mode — installing is impossible
    /// (systemd-boot needs UEFI), so the Summary step blocks with an explanation.
    bios_boot: bool,
    /// The serialized InstallRequest handed to the engine on the Install step.
    install_json: String,
    /// The current step title, streamed live from the engine during a real install.
    install_current: String,
}

impl cosmic::Application for Installer {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "org.draftos.Installer";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: ()) -> (Self, cosmic::app::Task<Message>) {
        // Dev hook: DRAFTOS_INSTALLER_STEP=<n> opens directly on a given step.
        let step = std::env::var("DRAFTOS_INSTALLER_STEP")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|&i| i < Step::ALL.len())
            .unwrap_or(0);
        let mut config = InstallConfig::default();

        // Detect locales and preselect the system's current one.
        let locales = system::detect_locales();
        let selected_locale = system::current_locale()
            .and_then(|l| locales.iter().position(|x| *x == l))
            .or_else(|| locales.iter().position(|x| x == "en_US.UTF-8"));
        if let Some(i) = selected_locale {
            config.locale = Some(locales[i].clone());
        }

        // Detect keyboard layouts; seed the selection from the current system
        // layouts (falling back to "us"), and default the switch to Alt+Shift.
        let layouts = system::detect_keyboard_layouts();
        let known = |code: &str| layouts.iter().any(|(c, _)| c == code);
        let mut seed: Vec<String> = system::current_layouts()
            .into_iter()
            .filter(|c| known(c))
            .collect();
        if seed.is_empty() {
            seed.push(if known("us") {
                "us".to_string()
            } else {
                layouts.first().map_or_else(|| "us".to_string(), |(c, _)| c.clone())
            });
        }
        config.keyboard_layouts = seed;
        config.keyboard_switch = Some(0);

        // Dev hook: DRAFTOS_INSTALLER_TYPE preselects an install type for previews.
        if let Ok(t) = std::env::var("DRAFTOS_INSTALLER_TYPE") {
            config.install_type = match t.as_str() {
                "clean" => Some(InstallType::Clean),
                "alongside" => Some(InstallType::Alongside),
                "reinstall" => Some(InstallType::Reinstall),
                "manual" => Some(InstallType::Manual),
                _ => None,
            };
        }

        // Detect time zones and preselect the system's current one.
        let timezones = system::detect_timezones();
        let selected_tz = system::current_timezone()
            .and_then(|tz| timezones.iter().position(|z| *z == tz));
        if let Some(i) = selected_tz {
            config.timezone = Some(timezones[i].clone());
        }

        let mut installer = Installer {
            core,
            step,
            config,
            locales,
            selected_locale,
            locale_search: String::new(),
            layouts,
            layout_search: String::new(),
            kbd_test: String::new(),
            timezones,
            selected_tz,
            tz_search: String::new(),
            disks: system::detect_disks(),
            selected_disk: None,
            partitions: system::detect_partitions(),
            luks_confirm: String::new(),
            password_confirm: String::new(),
            root_confirm: String::new(),
            install_progress: 0.0,
            plan_steps: Vec::new(),
            install_step: 0,
            summary_error: None,
            bios_boot: std::path::Path::new("/run/archiso").exists()
                && !std::path::Path::new("/sys/firmware/efi").exists(),
            install_json: String::new(),
            install_current: String::new(),
        };
        // Dev preview: when jumping straight to the Install step, populate a demo
        // plan so the screen renders (normally the plan is built at Summary→Install).
        if Step::ALL[installer.step] == Step::Install {
            installer.plan_steps = demo_plan();
            if !installer.plan_steps.is_empty() {
                installer.install_step = installer.plan_steps.len() / 3;
                installer.install_progress =
                    installer.install_step as f32 / installer.plan_steps.len() as f32;
            }
        }
        (installer, cosmic::app::Task::none())
    }

    fn update(&mut self, message: Message) -> cosmic::app::Task<Message> {
        let last = Step::ALL.len() - 1;
        match message {
            Message::Next => {
                if self.can_advance() {
                    if Step::ALL[self.step] == Step::Summary {
                        // Build the engine plan, enter Install, and kick off the
                        // real install (engine via pkexec). Blocks here on a bad
                        // config; a runtime failure bounces back with an error.
                        match self.build_plan() {
                            Ok(()) => {
                                self.step = (self.step + 1).min(last);
                                self.install_current = String::new();
                                let json = self.install_json.clone();
                                return cosmic::task::stream(engine_stream(json));
                            }
                            Err(e) => self.summary_error = Some(e),
                        }
                    } else {
                        self.step = (self.step + 1).min(last);
                    }
                }
            }
            Message::Back => self.step = self.step.saturating_sub(1),
            Message::Tick => {
                // Animate through the plan, but hold near the end until the engine
                // actually reports it finished (InstallFinished).
                if Step::ALL[self.step] == Step::Install && !self.plan_steps.is_empty() {
                    let cap = self.plan_steps.len() - 1;
                    if self.install_step < cap {
                        self.install_step += 1;
                    }
                    self.install_progress =
                        (self.install_step as f32 / self.plan_steps.len() as f32).min(0.95);
                }
            }
            Message::InstallProgress(pct, title) => {
                self.install_progress = (f32::from(pct as u16) / 100.0).clamp(0.0, 1.0);
                if !title.is_empty() {
                    self.install_current = title;
                }
            }
            Message::InstallFinished(result) => match result {
                Ok(()) => {
                    self.install_progress = 1.0;
                    self.step = (self.step + 1).min(last); // Install → Done
                }
                Err(e) => {
                    self.summary_error = Some(format!("Installation failed: {e}"));
                    // Bounce back to Summary so the user can review and retry.
                    self.step = Step::ALL
                        .iter()
                        .position(|s| *s == Step::Summary)
                        .unwrap_or(self.step);
                }
            },
            Message::SelectLocale(i) => {
                self.selected_locale = Some(i);
                self.config.locale = self.locales.get(i).cloned();
            }
            Message::SetLocaleSearch(v) => self.locale_search = v,
            Message::AddLayout(code) => {
                if !self.config.keyboard_layouts.contains(&code) {
                    self.config.keyboard_layouts.push(code);
                }
            }
            Message::RemoveLayout(i) => {
                if i < self.config.keyboard_layouts.len() {
                    self.config.keyboard_layouts.remove(i);
                }
            }
            Message::MoveLayoutUp(i) => {
                if i > 0 && i < self.config.keyboard_layouts.len() {
                    self.config.keyboard_layouts.swap(i - 1, i);
                }
            }
            Message::MoveLayoutDown(i) => {
                if i + 1 < self.config.keyboard_layouts.len() {
                    self.config.keyboard_layouts.swap(i, i + 1);
                }
            }
            Message::SetLayoutSearch(v) => self.layout_search = v,
            Message::SetKeyboardSwitch(i) => self.config.keyboard_switch = Some(i),
            Message::SetKeyboardTest(v) => self.kbd_test = v,
            Message::SelectTimezone(i) => {
                self.selected_tz = Some(i);
                self.config.timezone = self.timezones.get(i).cloned();
            }
            Message::SetTimezoneSearch(v) => self.tz_search = v,
            Message::SelectInstallType(t) => self.config.install_type = Some(t),
            Message::SelectDisk(i) => {
                self.selected_disk = Some(i);
                self.config.disk = self.disks.get(i).map(DiskInfo::device);
            }
            Message::SetRootPartition(i) => {
                self.config.root_partition = self.partitions.get(i).map(PartInfo::device);
            }
            Message::SetEfiPartition(i) => {
                self.config.efi_partition = self.partitions.get(i).map(PartInfo::device);
            }
            Message::ToggleEncrypt(on) => self.config.encrypt = on,
            Message::SetLuksPassword(v) => self.config.luks_password = v,
            Message::SetLuksConfirm(v) => self.luks_confirm = v,
            Message::SetFullName(v) => self.config.full_name = v,
            Message::SetUsername(v) => self.config.username = v,
            Message::SetHostname(v) => self.config.hostname = v,
            Message::SetPassword(v) => self.config.password = v,
            Message::SetPasswordConfirm(v) => self.password_confirm = v,
            // The toggler reads "Use the same password for root", so checked = shared.
            Message::ToggleRootSame(same) => self.config.root_separate = !same,
            Message::SetRootPassword(v) => self.config.root_password = v,
            Message::SetRootConfirm(v) => self.root_confirm = v,
            Message::Close => {
                if let Some(id) = self.core.main_window_id() {
                    return cosmic::iced::window::close(id);
                }
            }
            Message::Reboot => {
                // Only actually reboot from the live installer; elsewhere (dev
                // host) just close the window so we never reboot a real machine.
                if std::path::Path::new("/run/archiso").exists() {
                    return cosmic::task::future(async {
                        let _ = tokio::process::Command::new("pkexec")
                            .args(["systemctl", "reboot"])
                            .status()
                            .await;
                        Message::Close
                    });
                }
                if let Some(id) = self.core.main_window_id() {
                    return cosmic::iced::window::close(id);
                }
            }
        }
        cosmic::app::Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let step = Step::ALL[self.step];

        // Install and Done are centered hero screens; the rest are titled forms.
        let (inner, centered): (Element<'_, Message>, bool) = match step {
            Step::Install => (self.install_view(), true),
            Step::Done => (self.done_view(), true),
            _ => (self.form_page(step), false),
        };

        let framed = widget::container(widget::container(inner).width(Length::Fixed(620.0)))
            .center_x(Length::Fill);

        let body: Element<'_, Message> = if centered {
            // Hero screens (Install/Done): vertically centered, content is short.
            let content_area = widget::column::with_capacity(3)
                .push(widget::Space::new().height(Length::Fill))
                .push(framed)
                .push(widget::Space::new().height(Length::Fill));
            widget::container(content_area)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(Padding { top: 24.0, right: 32.0, bottom: 24.0, left: 32.0 })
                .into()
        } else {
            // Form steps: scroll when the content is taller than the window, so
            // every option/field is reachable at any size. The footer stays pinned.
            let padded = widget::container(framed)
                .width(Length::Fill)
                .padding(Padding { top: 40.0, right: 32.0, bottom: 24.0, left: 32.0 });
            widget::scrollable(padded)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        };

        widget::column::with_capacity(2)
            .push(body)
            .push(self.footer())
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        // Animate the install screen up to the ~95% hold point; after that we
        // wait for the engine's InstallFinished.
        // Animate only the dev preview (no request built); a real install streams
        // its own progress from the engine, so no ticking is needed there.
        if Step::ALL[self.step] == Step::Install
            && self.install_json.is_empty()
            && self.install_progress < 0.95
        {
            cosmic::iced::time::every(Duration::from_millis(200)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        }
    }
}

impl Installer {
    /// Whether the current step's requirements are met (gates the Continue button).
    fn can_advance(&self) -> bool {
        match Step::ALL[self.step] {
            Step::Language => self.config.locale.is_some(),
            Step::Keyboard => !self.config.keyboard_layouts.is_empty(),
            Step::Timezone => self.config.timezone.is_some(),
            Step::InstallType => self.config.install_type.is_some(),
            Step::Disk => {
                if self.config.install_type == Some(InstallType::Manual) {
                    self.config.root_partition.is_some()
                } else {
                    self.selected_disk.is_some()
                }
            }
            Step::Encryption => {
                !self.config.encrypt
                    || (!self.config.luks_password.is_empty()
                        && self.config.luks_password == self.luks_confirm)
            }
            Step::Account => self.account_ok(),
            Step::Summary => !self.bios_boot,
            _ => true,
        }
    }

    /// Build the engine plan from the collected config, ready for the Install step.
    fn build_plan(&mut self) -> Result<(), String> {
        let request = self.config.to_request()?;
        let steps = draftos_install::plan(&request).map_err(|e| e.to_string())?;
        self.install_json = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        self.plan_steps = steps
            .iter()
            .map(|s| (s.phase.label().to_string(), s.title.clone()))
            .collect();
        self.install_step = 0;
        self.install_progress = 0.0;
        self.summary_error = None;
        Ok(())
    }

    /// The account step is satisfiable when a username and a matching, non-empty
    /// password are present — and, if root has its own password, that it matches too.
    fn account_ok(&self) -> bool {
        let user_ok = draftos_install::config::valid_username(&self.config.username)
            && !self.config.full_name.contains([':', '\n'])
            && !self.config.password.is_empty()
            && !self.config.password.contains([':', '\n'])
            && self.config.password == self.password_confirm;
        // Hostname may be left empty (a default is applied), but a non-empty one
        // must be valid.
        let host = self.config.hostname.trim();
        let host_ok = host.is_empty() || draftos_install::config::valid_hostname(host);
        let root_ok = !self.config.root_separate
            || (!self.config.root_password.is_empty()
                && self.config.root_password == self.root_confirm);
        user_ok && host_ok && root_ok
    }

    /// Titled form layout: heading + subtitle above the step's content.
    fn form_page(&self, step: Step) -> Element<'_, Message> {
        let header = widget::column::with_capacity(2)
            .spacing(4)
            .push(widget::text::title2(step.title()))
            .push(widget::text::body(step.subtitle()));

        let content = match step {
            Step::Language => self.language_view(),
            Step::Keyboard => self.keyboard_view(),
            Step::Timezone => self.timezone_view(),
            Step::InstallType => self.install_type_view(),
            Step::Disk => self.disk_view(),
            Step::Encryption => self.encryption_view(),
            Step::Account => self.account_view(),
            Step::Summary => self.summary_view(),
            // Install/Done are hero screens, never routed through form_page.
            Step::Install | Step::Done => placeholder(step),
        };

        widget::column::with_capacity(2)
            .spacing(24)
            .push(header)
            .push(content)
            .into()
    }

    /// Install progress: the current step (streamed live from the engine, or the
    /// animated preview) and a bar.
    fn install_view(&self) -> Element<'_, Message> {
        // Prefer the live step title streamed from the engine; fall back to the
        // animated preview plan.
        let current = if self.install_current.is_empty() {
            self.plan_steps
                .get(self.install_step)
                .map(|(_, title)| title.clone())
                .unwrap_or_else(|| "Preparing…".to_string())
        } else {
            self.install_current.clone()
        };
        let pct = (self.install_progress * 100.0).round() as u32;

        widget::column::with_capacity(5)
            .spacing(16)
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .push(widget::text::title1("Installing DraftOS"))
            .push(widget::Space::new().height(Length::Fixed(8.0)))
            .push(widget::text::body(current).center())
            .push(widget::Space::new().height(Length::Fixed(8.0)))
            .push(
                widget::container(
                    widget::determinate_linear(self.install_progress).width(Length::Fixed(360.0)),
                )
                .center_x(Length::Fill),
            )
            .push(widget::text::caption(format!("{pct}%")))
            .into()
    }

    /// Success screen shown when the install completes.
    fn done_view(&self) -> Element<'_, Message> {
        widget::column::with_capacity(3)
            .spacing(20)
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .push(widget::icon::from_name("emblem-default-symbolic").size(96).icon())
            .push(widget::text::title1("DraftOS is installed"))
            .push(widget::text::body("Remove the installation media, then restart to begin.").center())
            .into()
    }

    /// Description for a layout code (e.g. `us` → "English (US)").
    fn layout_desc(&self, code: &str) -> String {
        self.layouts
            .iter()
            .find(|(c, _)| c == code)
            .map_or_else(|| code.to_string(), |(_, d)| d.clone())
    }

    /// Keyboard step: ordered layouts with reorder/remove, a switch shortcut, an
    /// "add layout" search list, and a test field.
    fn keyboard_view(&self) -> Element<'_, Message> {
        let layouts = &self.config.keyboard_layouts;
        let last = layouts.len().saturating_sub(1);

        // Selected layouts, each with move up/down and remove controls.
        let mut chosen = widget::column::with_capacity(layouts.len()).spacing(2);
        for (i, code) in layouts.iter().enumerate() {
            let name = if i == 0 {
                format!("{}  (primary)", self.layout_desc(code))
            } else {
                self.layout_desc(code)
            };
            let icon_btn = |icon: &str, msg: Option<Message>| {
                let b = widget::button::icon(widget::icon::from_name(icon).size(16));
                match msg {
                    Some(m) => b.on_press(m),
                    None => b,
                }
            };
            let row = widget::row::with_capacity(4)
                .spacing(4)
                .align_y(Alignment::Center)
                .push(widget::text::body(name).width(Length::Fill))
                .push(icon_btn("go-up-symbolic", (i > 0).then_some(Message::MoveLayoutUp(i))))
                .push(icon_btn("go-down-symbolic", (i < last).then_some(Message::MoveLayoutDown(i))))
                .push(icon_btn("list-remove-symbolic", (layouts.len() > 1).then_some(Message::RemoveLayout(i))));
            chosen = chosen.push(widget::container(row).padding([4, 8]));
        }
        let chosen = widget::container(chosen)
            .padding(8)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card);

        let mut root = widget::column::with_capacity(5).spacing(10).push(chosen);

        // Switch shortcut, relevant only with two or more layouts.
        if layouts.len() > 1 {
            let switch_labels: Vec<&str> = KEYBOARD_SWITCHES.iter().map(|(l, _)| *l).collect();
            root = root.push(widget::settings::item(
                "Switch layouts",
                widget::dropdown(switch_labels, self.config.keyboard_switch, Message::SetKeyboardSwitch),
            ));
        }

        // Add-a-layout searchable list (excludes already-chosen layouts).
        let search = widget::search_input("Add a layout", self.layout_search.as_str())
            .on_input(Message::SetLayoutSearch);
        let query = self.layout_search.trim().to_lowercase();
        let mut add = widget::column::with_capacity(32).spacing(1);
        let mut shown = 0usize;
        for (code, desc) in &self.layouts {
            if layouts.contains(code) {
                continue;
            }
            if !query.is_empty()
                && !desc.to_lowercase().contains(&query)
                && !code.to_lowercase().contains(&query)
            {
                continue;
            }
            if shown >= 200 {
                break;
            }
            add = add.push(
                widget::button::text(desc.clone())
                    .width(Length::Fill)
                    .on_press(Message::AddLayout(code.clone())),
            );
            shown += 1;
        }
        let add_list = widget::scrollable(
            widget::container(add)
                .padding(8)
                .width(Length::Fill)
                .class(cosmic::theme::Container::Card),
        )
        .height(Length::Fixed(140.0));
        root = root.push(search).push(add_list);

        // Test field.
        root = root.push(
            widget::text_input("Type here to test your keyboard", self.kbd_test.as_str())
                .on_input(Message::SetKeyboardTest),
        );

        root.into()
    }

    /// The two install-type choices as radios with a title and description.
    fn install_type_view(&self) -> Element<'_, Message> {
        let selected = self.config.install_type;
        let option = |title: &'static str, detail: &'static str, value: InstallType| {
            let label = widget::column::with_capacity(2)
                .spacing(2)
                .push(widget::text::heading(title))
                .push(widget::text::caption(detail));
            widget::container(
                widget::radio(label, value, selected, Message::SelectInstallType)
                    .width(Length::Fill),
            )
            .padding(16)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card)
        };
        // Not selectable: the engine doesn't implement these yet, and offering a
        // radio that dead-ends at Summary wastes the whole wizard. Shown as
        // roadmap, not as an option.
        let coming_soon = |title: &'static str, detail: &'static str| {
            widget::container(
                widget::column::with_capacity(2)
                    .spacing(2)
                    .push(widget::text::heading(format!("{title} — coming soon")))
                    .push(widget::text::caption(detail)),
            )
            .padding(16)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card)
        };

        widget::column::with_capacity(4)
            .spacing(12)
            .push(option(
                "Clean install",
                "Erase a whole disk and install DraftOS on it. Recommended.",
                InstallType::Clean,
            ))
            .push(option(
                "Manual partitioning",
                "Assign existing partitions yourself (advanced).",
                InstallType::Manual,
            ))
            .push(coming_soon(
                "Install alongside",
                "Keep your existing operating system and install DraftOS beside it.",
            ))
            .push(coming_soon(
                "Reinstall",
                "Replace an existing DraftOS installation, keeping the disk layout.",
            ))
            .into()
    }

    /// A search field over a scrollable, single-select list of strings. `display`
    /// maps each raw item to its shown label; the search matches either.
    fn searchable_list<'a>(
        &'a self,
        items: &'a [String],
        selected: Option<usize>,
        search: &'a str,
        spec: ListSpec,
    ) -> Element<'a, Message> {
        let search_box = widget::search_input(spec.placeholder, search).on_input(spec.on_search);

        let query = search.trim().to_lowercase();
        const CAP: usize = 200;
        let mut col = widget::column::with_capacity(64).spacing(2);
        let mut shown = 0usize;
        let mut more = false;
        for (i, item) in items.iter().enumerate() {
            let label = (spec.display)(item);
            if !query.is_empty()
                && !label.to_lowercase().contains(&query)
                && !item.to_lowercase().contains(&query)
            {
                continue;
            }
            if shown >= CAP {
                more = true;
                break;
            }
            col = col.push(
                widget::radio(widget::text::body(label), i, selected, spec.on_select)
                    .width(Length::Fill),
            );
            shown += 1;
        }
        if items.is_empty() {
            col = col.push(widget::text::caption("Nothing detected on this system."));
        } else if more {
            col = col.push(widget::text::caption("Refine your search to see more."));
        }

        let list = widget::scrollable(
            widget::container(col)
                .padding(12)
                .width(Length::Fill)
                .class(cosmic::theme::Container::Card),
        )
        .height(Length::Fixed(300.0));

        widget::column::with_capacity(2)
            .spacing(8)
            .push(search_box)
            .push(list)
            .into()
    }

    /// Language picker: every system locale, shown as "Language (Country)".
    fn language_view(&self) -> Element<'_, Message> {
        self.searchable_list(
            &self.locales,
            self.selected_locale,
            &self.locale_search,
            ListSpec {
                placeholder: "Search languages",
                display: locale_names::friendly,
                on_select: Message::SelectLocale,
                on_search: Message::SetLocaleSearch,
            },
        )
    }

    /// Time-zone picker: every system zone.
    fn timezone_view(&self) -> Element<'_, Message> {
        self.searchable_list(
            &self.timezones,
            self.selected_tz,
            &self.tz_search,
            ListSpec {
                placeholder: "Search time zones",
                display: |tz| tz.to_string(),
                on_select: Message::SelectTimezone,
                on_search: Message::SetTimezoneSearch,
            },
        )
    }

    /// Disk step — adapts to the chosen install type.
    fn disk_view(&self) -> Element<'_, Message> {
        if self.config.install_type == Some(InstallType::Manual) {
            self.manual_disk_view()
        } else {
            self.whole_disk_view()
        }
    }

    /// Whole-disk picker (Clean / Alongside / Reinstall), with a type-specific note.
    fn whole_disk_view(&self) -> Element<'_, Message> {
        if self.disks.is_empty() {
            return widget::container(
                widget::column::with_capacity(2)
                    .spacing(6)
                    .push(widget::text::heading("No disks found"))
                    .push(widget::text::caption(
                        "No installable disks were detected on this system.",
                    )),
            )
            .padding(16)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card)
            .into();
        }

        let mut col = widget::column::with_capacity(self.disks.len()).spacing(4);
        for (i, disk) in self.disks.iter().enumerate() {
            let label = widget::column::with_capacity(2)
                .spacing(1)
                .push(widget::text::body(disk.label()))
                .push(widget::text::caption(disk.device()));
            col = col.push(
                widget::radio(label, i, self.selected_disk, Message::SelectDisk).width(Length::Fill),
            );
        }
        let list = widget::container(col)
            .padding(12)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card);

        let note = match self.config.install_type {
            Some(InstallType::Alongside) => {
                "DraftOS will be installed in free space on the selected disk."
            }
            Some(InstallType::Reinstall) => {
                "The existing DraftOS on the selected disk will be replaced."
            }
            _ => "The selected disk will be completely erased.",
        };

        widget::column::with_capacity(2)
            .spacing(8)
            .push(list)
            .push(widget::text::caption(note))
            .into()
    }

    /// Manual partitioning: assign existing partitions to root and EFI.
    fn manual_disk_view(&self) -> Element<'_, Message> {
        if self.partitions.is_empty() {
            return widget::container(
                widget::column::with_capacity(2)
                    .spacing(6)
                    .push(widget::text::heading("No partitions found"))
                    .push(widget::text::caption(
                        "Create partitions with a disk tool first, then return here.",
                    )),
            )
            .padding(16)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card)
            .into();
        }

        let labels: Vec<String> = self.partitions.iter().map(PartInfo::label).collect();
        let index_of = |dev: &Option<String>| {
            dev.as_ref()
                .and_then(|d| self.partitions.iter().position(|p| &p.device() == d))
        };

        let rows = widget::column::with_capacity(2)
            .spacing(8)
            .push(widget::settings::item(
                "Root partition (/)",
                widget::dropdown(labels.clone(), index_of(&self.config.root_partition), Message::SetRootPartition),
            ))
            .push(widget::settings::item(
                "EFI partition (/boot/efi)",
                widget::dropdown(labels, index_of(&self.config.efi_partition), Message::SetEfiPartition),
            ));

        widget::column::with_capacity(2)
            .spacing(8)
            .push(
                widget::container(rows)
                    .padding(16)
                    .width(Length::Fill)
                    .class(cosmic::theme::Container::Card),
            )
            .push(widget::text::caption(
                "Assigned partitions will be formatted. Other partitions are left untouched.",
            ))
            .into()
    }

    /// Optional full-disk (LUKS) encryption toggle and passphrase.
    fn encryption_view(&self) -> Element<'_, Message> {
        let toggle = widget::settings::item(
            "Encrypt this installation",
            widget::toggler(self.config.encrypt).on_toggle(Message::ToggleEncrypt),
        );

        let mut col = widget::column::with_capacity(4).spacing(8).push(toggle);
        if self.config.encrypt {
            col = col
                .push(widget::settings::item(
                    "Passphrase",
                    widget::secure_input(String::new(), self.config.luks_password.clone(), None, true)
                        .on_input(Message::SetLuksPassword)
                        .width(Length::Fixed(300.0)),
                ))
                .push(widget::settings::item(
                    "Confirm passphrase",
                    widget::secure_input(String::new(), self.luks_confirm.clone(), None, true)
                        .on_input(Message::SetLuksConfirm)
                        .width(Length::Fixed(300.0)),
                ));
            if !self.luks_confirm.is_empty() && self.config.luks_password != self.luks_confirm {
                col = col.push(widget::text::caption("Passphrases do not match."));
            }
        }

        widget::container(col)
            .padding(16)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card)
            .into()
    }

    /// Account creation form.
    fn account_view(&self) -> Element<'_, Message> {
        let text_field = |label: &'static str,
                          placeholder: &'static str,
                          value: &str,
                          on_input: fn(String) -> Message| {
            widget::settings::item(
                label,
                widget::text_input(placeholder, value.to_string())
                    .on_input(on_input)
                    .width(Length::Fixed(300.0)),
            )
        };
        let secret = |label: &'static str, value: String, on_input: fn(String) -> Message| {
            widget::settings::item(
                label,
                widget::secure_input(String::new(), value, None, true)
                    .on_input(on_input)
                    .width(Length::Fixed(300.0)),
            )
        };

        let mut col = widget::column::with_capacity(6)
            .spacing(8)
            .push(text_field("Full name", "Your name", &self.config.full_name, Message::SetFullName))
            .push(text_field("Username", "username", &self.config.username, Message::SetUsername))
            .push(text_field("Computer name", "draftos", &self.config.hostname, Message::SetHostname))
            .push(secret("Password", self.config.password.clone(), Message::SetPassword))
            .push(secret("Confirm password", self.password_confirm.clone(), Message::SetPasswordConfirm));

        if !self.config.username.is_empty()
            && !draftos_install::config::valid_username(&self.config.username)
        {
            col = col.push(widget::text::caption(
                "Username: lowercase letters, digits, '-' and '_' only; must start \
                 with a letter ('root' is reserved).",
            ));
        }
        if !self.config.hostname.trim().is_empty()
            && !draftos_install::config::valid_hostname(self.config.hostname.trim())
        {
            col = col.push(widget::text::caption(
                "Computer name: letters, digits and '-' only, no spaces.",
            ));
        }
        if self.config.full_name.contains(':') {
            col = col.push(widget::text::caption("The full name can't contain ':'."));
        }
        if !self.password_confirm.is_empty() && self.config.password != self.password_confirm {
            col = col.push(widget::text::caption("Passwords do not match."));
        }

        // Administrator (root) password: share the user's by default.
        col = col.push(widget::settings::item(
            "Use the same password for the administrator (root)",
            widget::toggler(!self.config.root_separate).on_toggle(Message::ToggleRootSame),
        ));
        if self.config.root_separate {
            let secret = |label: &'static str, value: String, on_input: fn(String) -> Message| {
                widget::settings::item(
                    label,
                    widget::secure_input(String::new(), value, None, true)
                        .on_input(on_input)
                        .width(Length::Fixed(300.0)),
                )
            };
            col = col
                .push(secret("Root password", self.config.root_password.clone(), Message::SetRootPassword))
                .push(secret("Confirm root password", self.root_confirm.clone(), Message::SetRootConfirm));
            if !self.root_confirm.is_empty() && self.config.root_password != self.root_confirm {
                col = col.push(widget::text::caption("Root passwords do not match."));
            }
        }

        widget::container(col)
            .padding(16)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card)
            .into()
    }

    /// Read-only review of the collected configuration.
    fn summary_view(&self) -> Element<'_, Message> {
        let lang = self
            .config
            .locale
            .as_deref()
            .map_or_else(|| "—".to_string(), locale_names::friendly);
        let kbd = if self.config.keyboard_layouts.is_empty() {
            "—".to_string()
        } else {
            self.config
                .keyboard_layouts
                .iter()
                .map(|c| self.layout_desc(c))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let install = self.config.install_type.map_or("—", InstallType::label);
        let admin = if self.config.root_separate {
            "Separate password"
        } else {
            "Same as user"
        };
        let user = if self.config.username.trim().is_empty() {
            "—"
        } else {
            self.config.username.as_str()
        };
        let host = if self.config.hostname.trim().is_empty() {
            "draftos"
        } else {
            self.config.hostname.as_str()
        };

        let row = |label: &'static str, value: &'static str| {
            widget::settings::item(label, widget::text::body(value.to_string()))
        };

        let disk = if self.config.install_type == Some(InstallType::Manual) {
            match (&self.config.root_partition, &self.config.efi_partition) {
                (Some(r), Some(e)) => format!("root {r}, EFI {e}"),
                (Some(r), None) => format!("root {r}"),
                _ => "—".to_string(),
            }
        } else {
            self.config.disk.clone().unwrap_or_else(|| "—".to_string())
        };
        let tz = self.config.timezone.as_deref().unwrap_or("—");
        let encryption = if self.config.encrypt { "On (LUKS)" } else { "Off" };

        let rows = widget::column::with_capacity(8)
            .spacing(8)
            .push(widget::settings::item("Language", widget::text::body(lang)))
            .push(widget::settings::item("Keyboard", widget::text::body(kbd)))
            .push(widget::settings::item("Time zone", widget::text::body(tz.to_string())))
            .push(row("Installation", install))
            .push(widget::settings::item("Disk", widget::text::body(disk)))
            .push(row("Encryption", encryption))
            .push(widget::settings::item("Username", widget::text::body(user.to_string())))
            .push(widget::settings::item("Computer name", widget::text::body(host.to_string())))
            .push(row("Administrator", admin));

        let card = widget::container(rows)
            .padding(16)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card);

        let mut col = widget::column::with_capacity(3).spacing(8).push(card);
        if self.bios_boot {
            col = col.push(
                widget::container(widget::text::body(
                    "This computer booted in BIOS/legacy mode, but DraftOS requires \
                     UEFI. Reboot, enable UEFI boot for the USB medium in the \
                     firmware settings, and start the installer again.",
                ))
                .padding(16)
                .width(Length::Fill)
                .class(cosmic::theme::Container::Card),
            );
        }
        if let Some(err) = &self.summary_error {
            col = col.push(widget::text::caption(format!("Cannot start install: {err}")));
        }
        col.into()
    }

    /// Bottom navigation: Back · step counter · Continue/Install/Restart.
    fn footer(&self) -> Element<'_, Message> {
        let step = Step::ALL[self.step];
        // No going back from the first step, or once installing / finished.
        let hide_back = self.step == 0 || matches!(step, Step::Install | Step::Done);

        let left: Element<'_, Message> = if hide_back {
            widget::Space::new().into()
        } else {
            widget::button::standard("Back").on_press(Message::Back).into()
        };
        let left = widget::container(left).width(Length::Fixed(140.0));

        let (label, message) = match step {
            Step::Summary => ("Install", Some(Message::Next)),
            Step::Install => ("Installing…", None),
            Step::Done => ("Restart", Some(Message::Reboot)),
            _ => ("Continue", self.can_advance().then_some(Message::Next)),
        };
        let mut next = widget::button::suggested(label);
        if let Some(m) = message {
            next = next.on_press(m);
        }
        let right = widget::container(
            widget::row::with_capacity(2)
                .push(widget::Space::new().width(Length::Fill))
                .push(next),
        )
        .width(Length::Fixed(140.0));

        let counter = widget::text::caption(format!("Step {} of {}", self.step + 1, Step::ALL.len()));

        let footer = widget::row::with_capacity(5)
            .align_y(Alignment::Center)
            .push(left)
            .push(widget::Space::new().width(Length::Fill))
            .push(counter)
            .push(widget::Space::new().width(Length::Fill))
            .push(right);

        widget::container(footer).padding(24).width(Length::Fill).into()
    }
}

/// Parameters describing a [`Installer::searchable_list`]: how to label items,
/// and which messages selection and search produce.
struct ListSpec {
    placeholder: &'static str,
    display: fn(&str) -> String,
    on_select: fn(usize) -> Message,
    on_search: fn(String) -> Message,
}

/// Run the install engine as root and stream its live progress to the UI.
///
/// Uses `pkexec` so it can partition and install; the live ISO's polkit rule lets
/// the live user do this without a prompt. The engine refuses to touch anything
/// outside a live environment, so this is safe to invoke anywhere.
///
/// The engine emits `PROGRESS <pct> <title>` lines on stdout as each step runs;
/// we parse those into [`Message::InstallProgress`] so the bar reflects the real
/// work. Human-readable logs and the failure reason go to stderr. When the
/// process exits we send a single [`Message::InstallFinished`].
fn engine_stream(request_json: String) -> impl cosmic::iced::futures::Stream<Item = Message> {
    use cosmic::iced::futures::SinkExt;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    cosmic::iced::stream::channel(32, move |mut output: cosmic::iced::futures::channel::mpsc::Sender<Message>| async move {
        let spawned = tokio::process::Command::new("pkexec")
            .args(["draftos-install", "--config", "-", "--commit"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();

        let mut child = match spawned {
            Ok(c) => c,
            Err(e) => {
                let _ = output
                    .send(Message::InstallFinished(Err(format!(
                        "could not launch the install engine: {e}"
                    ))))
                    .await;
                return;
            }
        };

        // Feed the request, then close stdin so the engine proceeds.
        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(request_json.as_bytes()).await {
                let _ = output
                    .send(Message::InstallFinished(Err(format!(
                        "could not send the request: {e}"
                    ))))
                    .await;
                return;
            }
            drop(stdin);
        }

        // Drain stderr in the background so the pipe never blocks the engine, and
        // keep the last non-empty line as the failure reason.
        let stderr = child.stderr.take();
        let stderr_task = tokio::spawn(async move {
            let mut last = String::new();
            if let Some(err) = stderr {
                let mut lines = BufReader::new(err).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if !line.trim().is_empty() {
                        last = line;
                    }
                }
            }
            last
        });

        // Parse PROGRESS lines from stdout into UI updates.
        if let Some(stdout) = child.stdout.take() {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(rest) = line.strip_prefix("PROGRESS ") {
                    let (pct, title) = rest.split_once(' ').unwrap_or((rest, ""));
                    if let Ok(pct) = pct.trim().parse::<u32>() {
                        let _ = output
                            .send(Message::InstallProgress(pct, title.trim().to_string()))
                            .await;
                    }
                }
            }
        }

        let status = child.wait().await;
        let reason = stderr_task.await.unwrap_or_default();

        let result = match status {
            Ok(s) if s.success() => Ok(()),
            Ok(_) => Err(if reason.is_empty() {
                "the installation did not complete".to_string()
            } else {
                reason
            }),
            Err(e) => Err(format!("the engine did not run: {e}")),
        };
        let _ = output.send(Message::InstallFinished(result)).await;
    })
}

/// A demo engine plan used only to preview the Install screen during development.
fn demo_plan() -> Vec<(String, String)> {
    use draftos_install::config as eng;
    let req = eng::InstallRequest {
        locale: "en_US.UTF-8".into(),
        keymap: "us".into(),
        x11_layouts: vec!["us".into()],
        timezone: "Europe/Moscow".into(),
        hostname: "draftos".into(),
        target: eng::Target::WholeDisk { device: "/dev/sda".into() },
        luks_passphrase: None,
        account: eng::Account {
            username: "user".into(),
            full_name: "User".into(),
            password: eng::Secret("x".into()),
        },
        root: eng::RootPolicy::SameAsUser,
        kernel: eng::Kernel::Standard,
    };
    draftos_install::plan(&req)
        .map(|steps| {
            steps
                .iter()
                .map(|s| (s.phase.label().to_string(), s.title.clone()))
                .collect()
        })
        .unwrap_or_default()
}

/// A neutral placeholder for steps whose screens are not built yet.
fn placeholder<'a>(step: Step) -> Element<'a, Message> {
    widget::container(
        widget::column::with_capacity(2)
            .spacing(6)
            .push(widget::text::heading("Coming next"))
            .push(widget::text::caption(format!(
                "The \"{}\" screen is not built yet.",
                step.title()
            ))),
    )
    .padding(16)
    .width(Length::Fill)
    .class(cosmic::theme::Container::Card)
    .into()
}
