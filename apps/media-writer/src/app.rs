//! DraftOS Media Writer — write a DraftOS ISO to a USB drive.

use std::time::Duration;

use cosmic::iced::{Alignment, Length, Padding, Subscription};
use cosmic::prelude::*;
use cosmic::widget;
use cosmic::{executor, Core};

use crate::system::{self, DriveInfo};

/// Steps of the writer flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Source,
    Drive,
    Writing,
    Done,
}

impl Step {
    const ALL: [Step; 4] = [Step::Source, Step::Drive, Step::Writing, Step::Done];
    fn title(self) -> &'static str {
        match self {
            Step::Source => "Create install media",
            Step::Drive => "Select a USB drive",
            Step::Writing => "Writing DraftOS",
            Step::Done => "Ready",
        }
    }
    fn subtitle(self) -> &'static str {
        match self {
            Step::Source => "Choose the DraftOS ISO to write.",
            Step::Drive => "Pick the drive to write it to. It will be erased.",
            Step::Writing => "Writing the image — don't remove the drive.",
            Step::Done => "The drive is ready to boot from.",
        }
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    Next,
    Back,
    Close,
    ChooseIso,
    IsoChosen(Option<String>),
    SetIsoPath(String),
    RescanDrives,
    SelectDrive(usize),
    Tick,
    WriteFinished(Result<(), String>),
}

pub struct MediaWriter {
    core: Core,
    step: usize,
    iso_path: String,
    drives: Vec<DriveInfo>,
    selected_drive: Option<usize>,
    progress: f32,
    error: Option<String>,
}

impl cosmic::Application for MediaWriter {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "org.draftos.MediaWriter";

    fn core(&self) -> &Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: ()) -> (Self, cosmic::app::Task<Message>) {
        // Dev hook: DRAFTOS_WRITER_STEP=<n> opens on a given step for previews.
        let step = std::env::var("DRAFTOS_WRITER_STEP")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|&i| i < Step::ALL.len())
            .unwrap_or(0);
        let app = MediaWriter {
            core,
            step,
            iso_path: String::new(),
            drives: system::detect_removable_drives(),
            selected_drive: None,
            progress: 0.0,
            error: None,
        };
        (app, cosmic::app::Task::none())
    }

    fn update(&mut self, message: Message) -> cosmic::app::Task<Message> {
        let last = Step::ALL.len() - 1;
        match message {
            Message::Next => {
                if self.can_advance() {
                    let from = Step::ALL[self.step];
                    self.step = (self.step + 1).min(last);
                    match from {
                        // Refresh the drive list when arriving at the Drive step.
                        Step::Source => self.drives = system::detect_removable_drives(),
                        // Kick off the real write when leaving the Drive step.
                        Step::Drive => {
                            self.progress = 0.0;
                            self.error = None;
                            let iso = self.iso_path.clone();
                            let dev = self.drives[self.selected_drive.unwrap()].device();
                            return cosmic::task::future(async move {
                                Message::WriteFinished(write_iso(iso, dev).await)
                            });
                        }
                        _ => {}
                    }
                }
            }
            Message::Back => self.step = self.step.saturating_sub(1),
            Message::ChooseIso => {
                return cosmic::task::future(async move {
                    use cosmic::dialog::file_chooser::{self, FileFilter};
                    let dialog = file_chooser::open::Dialog::new()
                        .title("Choose a DraftOS ISO")
                        .filter(FileFilter::new("Disk images").glob("*.iso"));
                    match dialog.open_file().await {
                        Ok(resp) => Message::IsoChosen(
                            resp.url().to_file_path().ok().map(|p| p.display().to_string()),
                        ),
                        Err(_) => Message::IsoChosen(None),
                    }
                });
            }
            Message::IsoChosen(path) => {
                if let Some(p) = path {
                    self.iso_path = p;
                }
            }
            Message::SetIsoPath(p) => self.iso_path = p,
            Message::RescanDrives => {
                self.drives = system::detect_removable_drives();
                self.selected_drive = None;
            }
            Message::SelectDrive(i) => self.selected_drive = Some(i),
            Message::Tick => {
                if Step::ALL[self.step] == Step::Writing {
                    self.progress = (self.progress + 0.02).min(0.95);
                }
            }
            Message::WriteFinished(result) => match result {
                Ok(()) => {
                    self.progress = 1.0;
                    self.step = (self.step + 1).min(last); // Writing → Done
                }
                Err(e) => {
                    self.error = Some(e);
                    // Back to the Drive step so the user can retry.
                    self.step = Step::ALL.iter().position(|s| *s == Step::Drive).unwrap_or(0);
                }
            },
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
        let (inner, centered): (Element<'_, Message>, bool) = match step {
            Step::Source => (self.source_view(), false),
            Step::Drive => (self.drive_view(), false),
            Step::Writing => (self.writing_view(), true),
            Step::Done => (self.done_view(), true),
        };

        let framed = widget::container(widget::container(inner).width(Length::Fixed(560.0)))
            .center_x(Length::Fill);

        let body: Element<'_, Message> = if centered {
            let col = widget::column::with_capacity(3)
                .push(widget::Space::new().height(Length::Fill))
                .push(framed)
                .push(widget::Space::new().height(Length::Fill));
            widget::container(col)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(24)
                .into()
        } else {
            let padded = widget::container(framed).width(Length::Fill).padding(Padding {
                top: 40.0,
                right: 32.0,
                bottom: 24.0,
                left: 32.0,
            });
            widget::scrollable(padded).width(Length::Fill).height(Length::Fill).into()
        };

        widget::column::with_capacity(2).push(body).push(self.footer()).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        if Step::ALL[self.step] == Step::Writing && self.progress < 0.95 {
            cosmic::iced::time::every(Duration::from_millis(300)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        }
    }
}

impl MediaWriter {
    fn iso_ok(&self) -> bool {
        let p = self.iso_path.trim();
        !p.is_empty() && p.ends_with(".iso") && std::path::Path::new(p).is_file()
    }

    fn can_advance(&self) -> bool {
        match Step::ALL[self.step] {
            Step::Source => self.iso_ok(),
            Step::Drive => self.selected_drive.is_some(),
            _ => true,
        }
    }

    fn header(&self, step: Step) -> Element<'_, Message> {
        widget::column::with_capacity(2)
            .spacing(4)
            .push(widget::text::title2(step.title()))
            .push(widget::text::body(step.subtitle()))
            .into()
    }

    fn source_view(&self) -> Element<'_, Message> {
        let picker = widget::row::with_capacity(2)
            .spacing(8)
            .align_y(Alignment::Center)
            .push(
                widget::text_input("Path to the DraftOS .iso", self.iso_path.as_str())
                    .on_input(Message::SetIsoPath)
                    .width(Length::Fill),
            )
            .push(widget::button::standard("Choose…").on_press(Message::ChooseIso));

        let note = if self.iso_path.trim().is_empty() {
            widget::text::caption("Pick the DraftOS .iso you downloaded or built.")
        } else if self.iso_ok() {
            let size = std::fs::metadata(self.iso_path.trim())
                .map(|m| human(m.len()))
                .unwrap_or_default();
            widget::text::caption(format!("Ready — {size}"))
        } else {
            widget::text::caption("File not found, or it isn't an .iso.")
        };

        widget::column::with_capacity(3)
            .spacing(24)
            .push(self.header(Step::Source))
            .push(
                widget::container(widget::column::with_capacity(2).spacing(8).push(picker).push(note))
                    .padding(16)
                    .width(Length::Fill)
                    .class(cosmic::theme::Container::Card),
            )
            .into()
    }

    fn drive_view(&self) -> Element<'_, Message> {
        let content: Element<'_, Message> = if self.drives.is_empty() {
            widget::container(
                widget::column::with_capacity(2)
                    .spacing(6)
                    .push(widget::text::heading("No removable drives found"))
                    .push(widget::text::caption("Insert a USB drive, then press Rescan.")),
            )
            .padding(16)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card)
            .into()
        } else {
            let mut col = widget::column::with_capacity(self.drives.len()).spacing(4);
            for (i, drive) in self.drives.iter().enumerate() {
                let label = widget::column::with_capacity(2)
                    .spacing(1)
                    .push(widget::text::body(drive.label()))
                    .push(widget::text::caption(drive.device()));
                col = col.push(
                    widget::radio(label, i, self.selected_drive, Message::SelectDrive)
                        .width(Length::Fill),
                );
            }
            widget::container(col)
                .padding(12)
                .width(Length::Fill)
                .class(cosmic::theme::Container::Card)
                .into()
        };

        let mut column = widget::column::with_capacity(4)
            .spacing(12)
            .push(self.header(Step::Drive))
            .push(content)
            .push(
                widget::row::with_capacity(2)
                    .push(widget::button::standard("Rescan").on_press(Message::RescanDrives))
                    .push(widget::Space::new().width(Length::Fill)),
            );
        if let Some(err) = &self.error {
            column = column.push(widget::text::caption(format!("Write failed: {err}")));
        }
        if !self.drives.is_empty() {
            column = column.push(widget::text::caption(
                "The selected drive will be completely erased.",
            ));
        }
        column.into()
    }

    fn writing_view(&self) -> Element<'_, Message> {
        let pct = (self.progress * 100.0).round() as u32;
        let target = self
            .selected_drive
            .and_then(|i| self.drives.get(i))
            .map_or_else(|| "the drive".to_string(), DriveInfo::label);

        widget::column::with_capacity(5)
            .spacing(16)
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .push(widget::text::title1("Writing DraftOS"))
            .push(widget::text::body(format!("to {target}")).center())
            .push(widget::Space::new().height(Length::Fixed(8.0)))
            .push(
                widget::container(widget::determinate_linear(self.progress).width(Length::Fixed(360.0)))
                    .center_x(Length::Fill),
            )
            .push(widget::text::caption(format!("{pct}%  ·  don't remove the drive")))
            .into()
    }

    fn done_view(&self) -> Element<'_, Message> {
        widget::column::with_capacity(3)
            .spacing(20)
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .push(widget::icon::from_name("emblem-default-symbolic").size(96).icon())
            .push(widget::text::title1("Your drive is ready"))
            .push(
                widget::text::body("Remove it and boot from it to install DraftOS.").center(),
            )
            .into()
    }

    fn footer(&self) -> Element<'_, Message> {
        let step = Step::ALL[self.step];
        let hide_back = self.step == 0 || matches!(step, Step::Writing | Step::Done);

        let left: Element<'_, Message> = if hide_back {
            widget::Space::new().into()
        } else {
            widget::button::standard("Back").on_press(Message::Back).into()
        };
        let left = widget::container(left).width(Length::Fixed(140.0));

        let (label, msg) = match step {
            Step::Drive => ("Write", self.can_advance().then_some(Message::Next)),
            Step::Writing => ("Writing…", None),
            Step::Done => ("Finish", Some(Message::Close)),
            _ => ("Continue", self.can_advance().then_some(Message::Next)),
        };
        let mut next = widget::button::suggested(label);
        if let Some(m) = msg {
            next = next.on_press(m);
        }
        let right = widget::container(
            widget::row::with_capacity(2)
                .push(widget::Space::new().width(Length::Fill))
                .push(next),
        )
        .width(Length::Fixed(140.0));

        let footer = widget::row::with_capacity(3)
            .align_y(Alignment::Center)
            .push(left)
            .push(widget::Space::new().width(Length::Fill))
            .push(right);

        widget::container(footer).padding(24).width(Length::Fill).into()
    }
}

/// Write the ISO to the device as root (pkexec prompts for authentication).
async fn write_iso(iso: String, device: String) -> Result<(), String> {
    let output = tokio::process::Command::new("pkexec")
        .args([
            "dd",
            &format!("if={iso}"),
            &format!("of={device}"),
            "bs=4M",
            "conv=fsync",
        ])
        .output()
        .await
        .map_err(|e| format!("could not start the writer: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let last = stderr
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("the write did not complete (check permissions and the drive)");
        Err(last.to_string())
    }
}

/// Human-readable byte size.
fn human(bytes: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut n = bytes as f64;
    let mut i = 0;
    while n >= 1024.0 && i < units.len() - 1 {
        n /= 1024.0;
        i += 1;
    }
    format!("{n:.1} {}", units[i])
}
