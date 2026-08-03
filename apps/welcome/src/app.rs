//! Application state and the onboarding flow for DraftOS Hello.

use cosmic::iced::{Alignment, Length, Padding};
use cosmic::prelude::*;
use cosmic::widget;
use cosmic::{executor, Core};

use crate::pages::{self, Page};

/// User-driven events in the welcome flow.
#[derive(Clone, Debug)]
pub enum Message {
    /// Advance to the next page.
    Next,
    /// Go back one page.
    Back,
    /// Jump directly to a page (used by the step indicator).
    GoTo(usize),
    /// Finish onboarding and close the window.
    Close,
}

/// The DraftOS Hello application.
pub struct Welcome {
    core: Core,
    /// Index into [`Page::ALL`] of the current step.
    page: usize,
}

impl cosmic::Application for Welcome {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "org.draftos.Welcome";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: ()) -> (Self, cosmic::app::Task<Message>) {
        // Dev hook: DRAFTOS_WELCOME_PAGE=<n> opens directly on a given step, so
        // each page can be previewed/screenshot without clicking through.
        let page = std::env::var("DRAFTOS_WELCOME_PAGE")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|&i| i < Page::ALL.len())
            .unwrap_or(0);
        (Welcome { core, page }, cosmic::app::Task::none())
    }

    fn update(&mut self, message: Message) -> cosmic::app::Task<Message> {
        let last = Page::ALL.len() - 1;
        match message {
            Message::Next => self.page = (self.page + 1).min(last),
            Message::Back => self.page = self.page.saturating_sub(1),
            Message::GoTo(i) => {
                if i <= last {
                    self.page = i;
                }
            }
            Message::Close => {
                if let Some(id) = self.core.main_window_id() {
                    return cosmic::iced::window::close(id);
                }
            }
        }
        cosmic::app::Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let page = Page::ALL[self.page];
        let content = match page {
            Page::Welcome => pages::welcome(),
            Page::Highlights => pages::highlights(),
            Page::Personalize => pages::personalize(),
            Page::Finish => pages::finish(),
        };

        // Constrain content to a comfortable reading width, horizontally centered.
        let framed = widget::container(widget::container(content).width(Length::Fixed(560.0)))
            .center_x(Length::Fill);

        // Equal flexible spacers put hero content in the exact center of the area
        // between the header bar and the footer; content pages sit at the top.
        let content_area = if page.is_centered() {
            widget::column::with_capacity(3)
                .push(widget::Space::new().height(Length::Fill))
                .push(framed)
                .push(widget::Space::new().height(Length::Fill))
        } else {
            widget::column::with_capacity(2)
                .push(framed)
                .push(widget::Space::new().height(Length::Fill))
        };

        let body = widget::container(content_area)
            .width(Length::Fill)
            .height(Length::Fill)
            // Vertically symmetric so the flexible spacers center hero content
            // exactly between the header bar and the footer.
            .padding(Padding {
                top: 24.0,
                right: 32.0,
                bottom: 24.0,
                left: 32.0,
            });

        widget::column::with_capacity(2)
            .push(body)
            .push(self.footer())
            .into()
    }
}

impl Welcome {
    /// Bottom navigation bar: Back · step dots · Continue/Finish.
    fn footer(&self) -> Element<'_, Message> {
        let last = Page::ALL.len() - 1;
        let is_first = self.page == 0;
        let is_last = self.page == last;

        // Left slot: Back — hidden entirely on the first page (macOS-style). A
        // fixed-width slot on each side keeps the step dots centered.
        let left: Element<'_, Message> = if is_first {
            widget::Space::new().into()
        } else {
            widget::button::standard("Back").on_press(Message::Back).into()
        };
        let left = widget::container(left).width(Length::Fixed(120.0));

        let next = if is_last {
            widget::button::suggested("Finish").on_press(Message::Close)
        } else {
            widget::button::suggested("Continue").on_press(Message::Next)
        };
        // Right slot: next button pushed flush-right within the same fixed width.
        let right = widget::container(
            widget::row::with_capacity(2)
                .push(widget::Space::new().width(Length::Fill))
                .push(next),
        )
        .width(Length::Fixed(120.0));

        let footer = widget::row::with_capacity(5)
            .align_y(Alignment::Center)
            .push(left)
            .push(widget::Space::new().width(Length::Fill))
            .push(self.dots())
            .push(widget::Space::new().width(Length::Fill))
            .push(right);

        widget::container(footer)
            .padding(24)
            .width(Length::Fill)
            .into()
    }

    /// Step indicator: a clickable dot per page.
    fn dots(&self) -> Element<'_, Message> {
        let mut row = widget::row::with_capacity(Page::ALL.len())
            .spacing(10)
            .align_y(Alignment::Center);
        for i in 0..Page::ALL.len() {
            let glyph = if i == self.page { "●" } else { "○" };
            row = row.push(
                widget::button::text(glyph.to_string())
                    .padding(0)
                    .on_press(Message::GoTo(i)),
            );
        }
        row.into()
    }
}
