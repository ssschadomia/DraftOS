//! The install wizard: state, flow control, and per-step views.

use cosmic::iced::{Alignment, Length, Padding};
use cosmic::prelude::*;
use cosmic::widget;
use cosmic::{executor, Core};

use crate::config::{InstallConfig, InstallType, KEYBOARDS, LANGUAGES};
use crate::steps::Step;

/// Wizard events.
#[derive(Clone, Debug)]
pub enum Message {
    Next,
    Back,
    Close,
    SelectLanguage(usize),
    SelectKeyboard(usize),
    SelectInstallType(InstallType),
    SetFullName(String),
    SetUsername(String),
    SetHostname(String),
    SetPassword(String),
    SetPasswordConfirm(String),
}

/// The DraftOS installer.
pub struct Installer {
    core: Core,
    /// Index into [`Step::ALL`].
    step: usize,
    /// Choices collected so far.
    config: InstallConfig,
    /// Password re-entry, kept in the UI only to check it matches.
    password_confirm: String,
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
        let installer = Installer {
            core,
            step,
            config: InstallConfig::default(),
            password_confirm: String::new(),
        };
        (installer, cosmic::app::Task::none())
    }

    fn update(&mut self, message: Message) -> cosmic::app::Task<Message> {
        let last = Step::ALL.len() - 1;
        match message {
            Message::Next => {
                if self.can_advance() {
                    self.step = (self.step + 1).min(last);
                }
            }
            Message::Back => self.step = self.step.saturating_sub(1),
            Message::SelectLanguage(i) => self.config.language = Some(i),
            Message::SelectKeyboard(i) => self.config.keyboard = Some(i),
            Message::SelectInstallType(t) => self.config.install_type = Some(t),
            Message::SetFullName(v) => self.config.full_name = v,
            Message::SetUsername(v) => self.config.username = v,
            Message::SetHostname(v) => self.config.hostname = v,
            Message::SetPassword(v) => self.config.password = v,
            Message::SetPasswordConfirm(v) => self.password_confirm = v,
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

        let header = widget::column::with_capacity(2)
            .spacing(4)
            .push(widget::text::title2(step.title()))
            .push(widget::text::body(step.subtitle()));

        let content = match step {
            Step::Language => self.radio_list(LANGUAGES, self.config.language, Message::SelectLanguage),
            Step::Keyboard => self.radio_list(KEYBOARDS, self.config.keyboard, Message::SelectKeyboard),
            Step::InstallType => self.install_type_view(),
            Step::Account => self.account_view(),
            Step::Summary => self.summary_view(),
            other => placeholder(other),
        };

        // Constrain the whole thing to a comfortable column, centered horizontally.
        let page = widget::column::with_capacity(2)
            .spacing(24)
            .push(header)
            .push(content)
            .width(Length::Fixed(620.0));

        let body = widget::container(widget::container(page).center_x(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(Padding {
                top: 40.0,
                right: 32.0,
                bottom: 8.0,
                left: 32.0,
            });

        widget::column::with_capacity(2)
            .push(body)
            .push(self.footer())
            .into()
    }
}

impl Installer {
    /// Whether the current step's requirements are met (gates the Continue button).
    fn can_advance(&self) -> bool {
        match Step::ALL[self.step] {
            Step::Language => self.config.language.is_some(),
            Step::Keyboard => self.config.keyboard.is_some(),
            Step::InstallType => self.config.install_type.is_some(),
            Step::Account => self.account_ok(),
            _ => true,
        }
    }

    /// The account step is satisfiable when a username and a matching, non-empty
    /// password are present.
    fn account_ok(&self) -> bool {
        !self.config.username.trim().is_empty()
            && !self.config.password.is_empty()
            && self.config.password == self.password_confirm
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

        widget::column::with_capacity(2)
            .spacing(12)
            .push(option(
                "Clean install",
                "Erase a disk and install DraftOS on it. Recommended.",
                InstallType::Clean,
            ))
            .push(option(
                "Custom (advanced)",
                "Assign existing partitions yourself.",
                InstallType::Custom,
            ))
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

        widget::container(col)
            .padding(16)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card)
            .into()
    }

    /// Read-only review of the collected configuration.
    fn summary_view(&self) -> Element<'_, Message> {
        let lang = self.config.language.map_or("—", |i| LANGUAGES[i].0);
        let kbd = self.config.keyboard.map_or("—", |i| KEYBOARDS[i].0);
        let install = match self.config.install_type {
            Some(InstallType::Clean) => "Clean install",
            Some(InstallType::Custom) => "Custom (advanced)",
            None => "—",
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

        let rows = widget::column::with_capacity(5)
            .spacing(8)
            .push(row("Language", lang))
            .push(row("Keyboard", kbd))
            .push(row("Installation", install))
            .push(widget::settings::item("Username", widget::text::body(user.to_string())))
            .push(widget::settings::item("Computer name", widget::text::body(host.to_string())));

        widget::container(rows)
            .padding(16)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card)
            .into()
    }

    /// Bottom navigation: Back · step counter · Continue/Install/Restart.
    fn footer(&self) -> Element<'_, Message> {
        let step = Step::ALL[self.step];
        let is_first = self.step == 0;

        let left: Element<'_, Message> = if is_first {
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
