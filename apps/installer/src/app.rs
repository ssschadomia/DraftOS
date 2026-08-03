//! The install wizard: state, flow control, and per-step views.

use std::time::Duration;

use cosmic::iced::{Alignment, Length, Padding, Subscription};
use cosmic::prelude::*;
use cosmic::widget;
use cosmic::{executor, Core};

use crate::config::{InstallConfig, InstallType, KEYBOARDS};
use crate::locale_names;
use crate::steps::Step;
use crate::system::{self, DiskInfo};

/// Wizard events.
#[derive(Clone, Debug)]
pub enum Message {
    Next,
    Back,
    Close,
    /// Timer tick that drives the install progress animation.
    Tick,
    SelectLocale(usize),
    SetLocaleSearch(String),
    SelectKeyboard(usize),
    SelectTimezone(usize),
    SetTimezoneSearch(String),
    SelectInstallType(InstallType),
    SelectDisk(usize),
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
    /// LUKS passphrase re-entry, UI-only, to check it matches.
    luks_confirm: String,
    /// Password re-entry, kept in the UI only to check it matches.
    password_confirm: String,
    /// Root password re-entry, UI-only, used when root has a separate password.
    root_confirm: String,
    /// Install progress, 0.0..=1.0. Simulated until the engine reports real work.
    install_progress: f32,
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
            .and_then(|l| locales.iter().position(|x| *x == l));
        if let Some(i) = selected_locale {
            config.locale = Some(locales[i].clone());
        }

        // Detect time zones and preselect the system's current one.
        let timezones = system::detect_timezones();
        let selected_tz = system::current_timezone()
            .and_then(|tz| timezones.iter().position(|z| *z == tz));
        if let Some(i) = selected_tz {
            config.timezone = Some(timezones[i].clone());
        }

        let installer = Installer {
            core,
            step,
            config,
            locales,
            selected_locale,
            locale_search: String::new(),
            timezones,
            selected_tz,
            tz_search: String::new(),
            disks: system::detect_disks(),
            selected_disk: None,
            luks_confirm: String::new(),
            password_confirm: String::new(),
            root_confirm: String::new(),
            install_progress: 0.0,
        };
        (installer, cosmic::app::Task::none())
    }

    fn update(&mut self, message: Message) -> cosmic::app::Task<Message> {
        let last = Step::ALL.len() - 1;
        match message {
            Message::Next => {
                if self.can_advance() {
                    self.step = (self.step + 1).min(last);
                    if Step::ALL[self.step] == Step::Install {
                        self.install_progress = 0.0;
                    }
                }
            }
            Message::Back => self.step = self.step.saturating_sub(1),
            Message::Tick => {
                if Step::ALL[self.step] == Step::Install {
                    self.install_progress = (self.install_progress + 0.012).min(1.0);
                    if self.install_progress >= 1.0 {
                        self.step = (self.step + 1).min(last);
                    }
                }
            }
            Message::SelectLocale(i) => {
                self.selected_locale = Some(i);
                self.config.locale = self.locales.get(i).cloned();
            }
            Message::SetLocaleSearch(v) => self.locale_search = v,
            Message::SelectKeyboard(i) => self.config.keyboard = Some(i),
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

        let content_area = if centered {
            widget::column::with_capacity(3)
                .push(widget::Space::new().height(Length::Fill))
                .push(framed)
                .push(widget::Space::new().height(Length::Fill))
        } else {
            widget::column::with_capacity(2)
                .push(framed)
                .push(widget::Space::new().height(Length::Fill))
        };

        let padding = if centered {
            Padding { top: 24.0, right: 32.0, bottom: 24.0, left: 32.0 }
        } else {
            Padding { top: 40.0, right: 32.0, bottom: 8.0, left: 32.0 }
        };

        let body = widget::container(content_area)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(padding);

        widget::column::with_capacity(2)
            .push(body)
            .push(self.footer())
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        // Only tick while the install screen is animating.
        if Step::ALL[self.step] == Step::Install && self.install_progress < 1.0 {
            cosmic::iced::time::every(Duration::from_millis(90)).map(|_| Message::Tick)
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
            Step::Keyboard => self.config.keyboard.is_some(),
            Step::Timezone => self.config.timezone.is_some(),
            Step::InstallType => self.config.install_type.is_some(),
            Step::Disk => self.selected_disk.is_some(),
            Step::Encryption => {
                !self.config.encrypt
                    || (!self.config.luks_password.is_empty()
                        && self.config.luks_password == self.luks_confirm)
            }
            Step::Account => self.account_ok(),
            _ => true,
        }
    }

    /// The account step is satisfiable when a username and a matching, non-empty
    /// password are present — and, if root has its own password, that it matches too.
    fn account_ok(&self) -> bool {
        let user_ok = !self.config.username.trim().is_empty()
            && !self.config.password.is_empty()
            && self.config.password == self.password_confirm;
        let root_ok = !self.config.root_separate
            || (!self.config.root_password.is_empty()
                && self.config.root_password == self.root_confirm);
        user_ok && root_ok
    }

    /// Titled form layout: heading + subtitle above the step's content.
    fn form_page(&self, step: Step) -> Element<'_, Message> {
        let header = widget::column::with_capacity(2)
            .spacing(4)
            .push(widget::text::title2(step.title()))
            .push(widget::text::body(step.subtitle()));

        let content = match step {
            Step::Language => self.language_view(),
            Step::Keyboard => self.radio_list(KEYBOARDS, self.config.keyboard, Message::SelectKeyboard),
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

    /// Install progress: a rotating highlight, the current phase, and a bar.
    fn install_view(&self) -> Element<'_, Message> {
        const TIPS: [(&str, &str, &str); 4] = [
            ("video-display-symbolic", "A desktop with depth", "COSMIC with real glass, tuned to stay out of your way."),
            ("system-software-install-symbolic", "Curated apps", "A store, control center and companion tools that feel like one system."),
            ("utilities-system-monitor-symbolic", "Tuned for speed", "A performance-first Arch base with the CachyOS kernel."),
            ("security-high-symbolic", "Open and yours", "The polish of a curated OS with the freedom of Linux underneath."),
        ];
        let idx = ((self.install_progress * TIPS.len() as f32) as usize).min(TIPS.len() - 1);
        let (icon, title, detail) = TIPS[idx];

        let highlight = widget::column::with_capacity(3)
            .spacing(12)
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .push(widget::icon::from_name(icon).size(72).icon())
            .push(widget::text::title3(title))
            .push(widget::text::body(detail).center());

        let pct = (self.install_progress * 100.0).round() as u32;

        widget::column::with_capacity(6)
            .spacing(20)
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .push(widget::text::title1("Installing DraftOS"))
            .push(highlight)
            .push(widget::Space::new().height(Length::Fixed(8.0)))
            .push(widget::text::body(self.install_phase()))
            .push(
                widget::container(
                    widget::determinate_linear(self.install_progress).width(Length::Fixed(360.0)),
                )
                .center_x(Length::Fill),
            )
            .push(widget::text::caption(format!("{pct}%")))
            .into()
    }

    /// Human label for the current install phase, derived from progress.
    fn install_phase(&self) -> &'static str {
        match (self.install_progress * 6.0) as usize {
            0 => "Preparing the disk…",
            1 => "Creating partitions…",
            2 => "Installing the base system…",
            3 => "Installing the COSMIC desktop…",
            4 => "Configuring your system…",
            _ => "Finishing up…",
        }
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

    /// A selectable list of `(display, _)` options rendered as radios in a card.
    fn radio_list(
        &self,
        options: &'static [(&'static str, &'static str)],
        selected: Option<usize>,
        on_select: fn(usize) -> Message,
    ) -> Element<'_, Message> {
        let mut col = widget::column::with_capacity(options.len()).spacing(2);
        for (i, (label, _)) in options.iter().enumerate() {
            col = col.push(
                widget::radio(widget::text::body(*label), i, selected, on_select)
                    .width(Length::Fill),
            );
        }
        widget::scrollable(
            widget::container(col)
                .padding(12)
                .width(Length::Fill)
                .class(cosmic::theme::Container::Card),
        )
        .into()
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

        widget::column::with_capacity(4)
            .spacing(12)
            .push(option(
                "Clean install",
                "Erase a whole disk and install DraftOS on it. Recommended.",
                InstallType::Clean,
            ))
            .push(option(
                "Install alongside",
                "Keep your existing operating system and install DraftOS beside it.",
                InstallType::Alongside,
            ))
            .push(option(
                "Reinstall",
                "Replace an existing DraftOS installation, keeping the disk layout.",
                InstallType::Reinstall,
            ))
            .push(option(
                "Manual partitioning",
                "Assign existing partitions yourself (advanced).",
                InstallType::Manual,
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

    /// Target-disk picker, populated from `lsblk`.
    fn disk_view(&self) -> Element<'_, Message> {
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

        widget::column::with_capacity(2)
            .spacing(8)
            .push(list)
            .push(widget::text::caption(
                "The selected disk will be erased during a clean install.",
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
        let kbd = self.config.keyboard.map_or("—", |i| KEYBOARDS[i].0);
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

        let disk = self.config.disk.as_deref().unwrap_or("—");
        let tz = self.config.timezone.as_deref().unwrap_or("—");
        let encryption = if self.config.encrypt { "On (LUKS)" } else { "Off" };

        let rows = widget::column::with_capacity(8)
            .spacing(8)
            .push(widget::settings::item("Language", widget::text::body(lang)))
            .push(row("Keyboard", kbd))
            .push(widget::settings::item("Time zone", widget::text::body(tz.to_string())))
            .push(row("Installation", install))
            .push(widget::settings::item("Disk", widget::text::body(disk.to_string())))
            .push(row("Encryption", encryption))
            .push(widget::settings::item("Username", widget::text::body(user.to_string())))
            .push(widget::settings::item("Computer name", widget::text::body(host.to_string())))
            .push(row("Administrator", admin));

        widget::container(rows)
            .padding(16)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card)
            .into()
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
            Step::Done => ("Restart", Some(Message::Close)),
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
