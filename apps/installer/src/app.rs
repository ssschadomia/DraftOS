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
}

/// The DraftOS installer.
pub struct Installer {
    core: Core,
    /// Index into [`Step::ALL`].
    step: usize,
    /// Choices collected so far.
    config: InstallConfig,
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
        (Installer { core, step, config: InstallConfig::default() }, cosmic::app::Task::none())
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
            _ => true,
        }
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
