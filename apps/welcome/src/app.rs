//! Application state and the onboarding flow for DraftOS Hello.

use cosmic::iced::{Alignment, Length};
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
        (Welcome { core, page: 0 }, cosmic::app::Task::none())
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
            // TODO: swap for a proper window-close task once verified live.
            Message::Close => std::process::exit(0),
        }
        cosmic::app::Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let content = match Page::ALL[self.page] {
            Page::Welcome => pages::welcome(),
            Page::Highlights => pages::highlights(),
            Page::Personalize => pages::personalize(),
            Page::Finish => pages::finish(),
        };

        // Constrain content to a comfortable reading width, centered in the window.
        let body = widget::container(widget::container(content).width(Length::Fixed(560.0)))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding(32);

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

        let back = {
            let b = widget::button::standard("Back");
            if is_first {
                b
            } else {
                b.on_press(Message::Back)
            }
        };

        let next = if is_last {
            widget::button::suggested("Finish").on_press(Message::Close)
        } else {
            widget::button::suggested("Continue").on_press(Message::Next)
        };

        let footer = widget::row::with_capacity(5)
            .align_y(Alignment::Center)
            .push(back)
            .push(widget::Space::new().width(Length::Fill))
            .push(self.dots())
            .push(widget::Space::new().width(Length::Fill))
            .push(next);

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
