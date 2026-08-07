//! DraftOS Media Writer — download an edition (or use a local ISO) and write it
//! to a USB drive.

use std::time::Duration;

use cosmic::iced::{Alignment, Length, Padding, Subscription};
use cosmic::prelude::*;
use cosmic::widget;
use cosmic::{executor, Core};
use serde::Deserialize;

use crate::system::{self, DriveInfo};

/// Where the editions list is fetched from (repo-controlled, always available).
const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/ssschadomia/DraftOS/main/editions/editions.json";

/// One downloadable DraftOS edition, from the manifest.
#[derive(Debug, Clone, Deserialize)]
pub struct Edition {
    id: String,
    name: String,
    description: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    available: bool,
    #[serde(default)]
    url: String,
    #[serde(default)]
    size: u64,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    editions: Vec<Edition>,
}

/// How the user is providing the image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceMode {
    Download,
    Local,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Source,
    Edition,
    Download,
    LocalIso,
    Drive,
    Writing,
    Done,
}

impl Step {
    fn title(self) -> &'static str {
        match self {
            Step::Source => "Create install media",
            Step::Edition => "Choose an edition",
            Step::Download => "Downloading DraftOS",
            Step::LocalIso => "Choose an ISO",
            Step::Drive => "Select a USB drive",
            Step::Writing => "Writing DraftOS",
            Step::Done => "Ready",
        }
    }
    fn subtitle(self) -> &'static str {
        match self {
            Step::Source => "Download DraftOS, or use an ISO you already have.",
            Step::Edition => "Pick the edition to download.",
            Step::Download => "Fetching the image — this can take a while.",
            Step::LocalIso => "Point to the DraftOS ISO on this computer.",
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
    SetMode(SourceMode),
    ManifestLoaded(Result<Vec<Edition>, String>),
    RetryManifest,
    SelectEdition(usize),
    DownloadFinished(Result<String, String>),
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
    step: Step,
    mode: SourceMode,
    editions: Vec<Edition>,
    editions_loading: bool,
    selected_edition: Option<usize>,
    iso_path: String,
    download_path: String,
    download_total: u64,
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
        let mut app = MediaWriter {
            core,
            step: Step::Source,
            mode: SourceMode::Download,
            editions: Vec::new(),
            editions_loading: false,
            selected_edition: None,
            iso_path: String::new(),
            download_path: String::new(),
            download_total: 0,
            drives: Vec::new(),
            selected_drive: None,
            progress: 0.0,
            error: None,
        };
        // Dev hook: preview the edition picker without navigating/fetching.
        if std::env::var("DRAFTOS_WRITER_PREVIEW").as_deref() == Ok("edition") {
            app.step = Step::Edition;
            app.editions = vec![
                Edition {
                    id: "desktop".into(),
                    name: "Desktop".into(),
                    description: "Classic desktops and laptops. CachyOS-tuned COSMIC".into(),
                    version: "2026.08.06".into(),
                    available: true,
                    url: "x".into(),
                    size: 1_990_000_000,
                },
                Edition {
                    id: "immutable".into(),
                    name: "Immutable".into(),
                    description: "Atomic A/B system (Bazzite / SteamOS style)".into(),
                    version: String::new(),
                    available: false,
                    url: String::new(),
                    size: 0,
                },
            ];
        }
        (app, cosmic::app::Task::none())
    }

    fn update(&mut self, message: Message) -> cosmic::app::Task<Message> {
        match message {
            Message::Next => return self.advance(),
            Message::Back => self.go_back(),
            Message::SetMode(m) => self.mode = m,
            Message::RetryManifest => {
                self.editions_loading = true;
                self.error = None;
                return cosmic::task::future(async {
                    Message::ManifestLoaded(load_manifest(MANIFEST_URL.into()).await)
                });
            }
            Message::ManifestLoaded(result) => {
                self.editions_loading = false;
                match result {
                    Ok(list) => self.editions = list,
                    Err(e) => self.error = Some(e),
                }
            }
            Message::SelectEdition(i) => self.selected_edition = Some(i),
            Message::DownloadFinished(result) => match result {
                Ok(path) => {
                    self.iso_path = path;
                    self.progress = 1.0;
                    self.step = Step::Drive;
                    self.drives = system::detect_removable_drives();
                    // The list was just rebuilt — a stale index could point at a
                    // different drive than the user chose.
                    self.selected_drive = None;
                }
                Err(e) => {
                    self.error = Some(e);
                    self.step = Step::Edition;
                }
            },
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
            Message::Tick => match self.step {
                Step::Download => {
                    if self.download_total > 0 {
                        let got = std::fs::metadata(&self.download_path)
                            .map(|m| m.len())
                            .unwrap_or(0);
                        self.progress = (got as f32 / self.download_total as f32).min(0.99);
                    }
                }
                Step::Writing => self.progress = (self.progress + 0.02).min(0.95),
                _ => {}
            },
            Message::WriteFinished(result) => match result {
                Ok(()) => {
                    self.progress = 1.0;
                    self.step = Step::Done;
                }
                Err(e) => {
                    self.error = Some(e);
                    self.step = Step::Drive;
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
        let centered = matches!(self.step, Step::Download | Step::Writing | Step::Done);
        let inner = match self.step {
            Step::Source => self.source_view(),
            Step::Edition => self.edition_view(),
            Step::Download => self.progress_view("Downloading DraftOS", "downloading the image"),
            Step::LocalIso => self.local_iso_view(),
            Step::Drive => self.drive_view(),
            Step::Writing => self.progress_view("Writing DraftOS", "writing to the drive"),
            Step::Done => self.done_view(),
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
        let active = (self.step == Step::Download && self.progress < 0.99)
            || (self.step == Step::Writing && self.progress < 0.95);
        if active {
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

    fn selected_edition_ok(&self) -> bool {
        self.selected_edition
            .and_then(|i| self.editions.get(i))
            .is_some_and(|e| e.available && !e.url.is_empty())
    }

    fn can_advance(&self) -> bool {
        match self.step {
            Step::Source => true,
            Step::Edition => self.selected_edition_ok(),
            Step::LocalIso => self.iso_ok(),
            Step::Drive => self.selected_drive.is_some(),
            _ => false,
        }
    }

    /// Advance to the next step, kicking off any async work (fetch/download/write).
    fn advance(&mut self) -> cosmic::app::Task<Message> {
        if !self.can_advance() {
            return cosmic::app::Task::none();
        }
        match self.step {
            Step::Source => match self.mode {
                SourceMode::Local => self.step = Step::LocalIso,
                SourceMode::Download => {
                    self.step = Step::Edition;
                    if self.editions.is_empty() && !self.editions_loading {
                        self.editions_loading = true;
                        self.error = None;
                        return cosmic::task::future(async {
                            Message::ManifestLoaded(load_manifest(MANIFEST_URL.into()).await)
                        });
                    }
                }
            },
            Step::Edition => {
                if let Some(ed) =
                    self.selected_edition.and_then(|i| self.editions.get(i)).cloned()
                {
                    self.step = Step::Download;
                    self.progress = 0.0;
                    self.error = None;
                    self.download_total = ed.size;
                    // NOT temp_dir(): /tmp is RAM-backed tmpfs on Arch/Fedora —
                    // a ~2 GB ISO there evicts memory or fails outright. Use the
                    // on-disk cache dir, and check free space up front.
                    let cache = dirs_cache().join("draftos-media-writer");
                    let _ = std::fs::create_dir_all(&cache);
                    if let Some(free) = free_space_bytes(&cache) {
                        if ed.size > 0 && free < ed.size + 512 * 1024 * 1024 {
                            self.error = Some(format!(
                                "Not enough disk space: the image needs {} and only {} is free.",
                                human(ed.size),
                                human(free)
                            ));
                            return cosmic::app::Task::none();
                        }
                    }
                    let dest = cache
                        .join(format!("draftos-{}-{}.iso", ed.id, ed.version))
                        .display()
                        .to_string();
                    self.download_path = dest.clone();
                    return cosmic::task::future(async move {
                        Message::DownloadFinished(download_iso(ed.url, dest).await)
                    });
                }
            }
            Step::LocalIso => {
                self.step = Step::Drive;
                self.drives = system::detect_removable_drives();
                // Fresh list — never carry a selection index across a re-detect.
                self.selected_drive = None;
            }
            Step::Drive => {
                // The drive list may have changed since selection (drive pulled,
                // rescan); never index blindly.
                let Some(drive) = self.selected_drive.and_then(|i| self.drives.get(i)) else {
                    self.error = Some("The selected drive is no longer available — pick it again.".into());
                    self.selected_drive = None;
                    self.drives = system::detect_removable_drives();
                    return cosmic::app::Task::none();
                };
                let dev = drive.device();
                self.step = Step::Writing;
                self.progress = 0.0;
                self.error = None;
                let iso = self.iso_path.clone();
                return cosmic::task::future(async move {
                    Message::WriteFinished(write_iso(iso, dev).await)
                });
            }
            _ => {}
        }
        cosmic::app::Task::none()
    }

    fn go_back(&mut self) {
        self.step = match self.step {
            Step::Edition | Step::LocalIso => Step::Source,
            Step::Drive => {
                if self.mode == SourceMode::Local {
                    Step::LocalIso
                } else {
                    Step::Edition
                }
            }
            other => other,
        };
    }

    fn header(&self) -> Element<'_, Message> {
        widget::column::with_capacity(2)
            .spacing(4)
            .push(widget::text::title2(self.step.title()))
            .push(widget::text::body(self.step.subtitle()))
            .into()
    }

    fn source_view(&self) -> Element<'_, Message> {
        let option = |title: &'static str, detail: &'static str, value: SourceMode, selected| {
            let label = widget::column::with_capacity(2)
                .spacing(2)
                .push(widget::text::heading(title))
                .push(widget::text::caption(detail));
            widget::container(
                widget::radio(label, value, selected, Message::SetMode).width(Length::Fill),
            )
            .padding(16)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card)
        };
        let sel = Some(self.mode);
        widget::column::with_capacity(3)
            .spacing(12)
            .push(self.header())
            .push(option(
                "Download DraftOS",
                "Fetch the latest edition and write it — no ISO needed.",
                SourceMode::Download,
                sel,
            ))
            .push(option(
                "Use a local ISO",
                "Write a DraftOS .iso you already have.",
                SourceMode::Local,
                sel,
            ))
            .into()
    }

    fn edition_view(&self) -> Element<'_, Message> {
        let content: Element<'_, Message> = if self.editions_loading {
            card(widget::text::body("Loading editions…").into())
        } else if self.editions.is_empty() {
            let msg = self.error.clone().unwrap_or_else(|| "No editions available.".into());
            widget::column::with_capacity(2)
                .spacing(8)
                .push(card(widget::text::caption(msg).into()))
                .push(widget::button::standard("Retry").on_press(Message::RetryManifest))
                .into()
        } else {
            let mut col = widget::column::with_capacity(self.editions.len()).spacing(4);
            for (i, ed) in self.editions.iter().enumerate() {
                let title = if ed.version.is_empty() {
                    ed.name.clone()
                } else {
                    format!("{} {}", ed.name, ed.version)
                };
                let detail = if ed.available && ed.size > 0 {
                    format!("{}  ·  {}", ed.description, human(ed.size))
                } else {
                    format!("{}  ·  coming soon", ed.description)
                };
                let lines = widget::column::with_capacity(2)
                    .spacing(1)
                    .push(widget::text::body(title))
                    .push(widget::text::caption(detail));
                // Only available editions are selectable; others are shown greyed out.
                let row: Element<'_, Message> = if ed.available {
                    widget::radio(lines, i, self.selected_edition, Message::SelectEdition)
                        .width(Length::Fill)
                        .into()
                } else {
                    widget::container(lines).padding([2, 8]).width(Length::Fill).into()
                };
                col = col.push(row);
            }
            card(col.into())
        };
        widget::column::with_capacity(2)
            .spacing(12)
            .push(self.header())
            .push(content)
            .into()
    }

    fn local_iso_view(&self) -> Element<'_, Message> {
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
        widget::column::with_capacity(2)
            .spacing(12)
            .push(self.header())
            .push(card(widget::column::with_capacity(2)
                .spacing(8)
                .push(picker)
                .push(note)
                .into()))
            .into()
    }

    fn drive_view(&self) -> Element<'_, Message> {
        let content: Element<'_, Message> = if self.drives.is_empty() {
            card(widget::column::with_capacity(2)
                .spacing(6)
                .push(widget::text::heading("No removable drives found"))
                .push(widget::text::caption("Insert a USB drive, then press Rescan."))
                .into())
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
            card(col.into())
        };
        let mut column = widget::column::with_capacity(4)
            .spacing(12)
            .push(self.header())
            .push(content)
            .push(
                widget::row::with_capacity(2)
                    .push(widget::button::standard("Rescan").on_press(Message::RescanDrives))
                    .push(widget::Space::new().width(Length::Fill)),
            );
        if let Some(err) = &self.error {
            column = column.push(widget::text::caption(format!("Failed: {err}")));
        }
        if !self.drives.is_empty() {
            column = column
                .push(widget::text::caption("The selected drive will be completely erased."));
        }
        column.into()
    }

    fn progress_view(&self, title: &'static str, what: &'static str) -> Element<'_, Message> {
        let pct = (self.progress * 100.0).round() as u32;
        widget::column::with_capacity(4)
            .spacing(16)
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .push(widget::text::title1(title))
            .push(widget::Space::new().height(Length::Fixed(8.0)))
            .push(
                widget::container(widget::determinate_linear(self.progress).width(Length::Fixed(360.0)))
                    .center_x(Length::Fill),
            )
            .push(widget::text::caption(format!("{pct}%  ·  {what}")))
            .into()
    }

    fn done_view(&self) -> Element<'_, Message> {
        widget::column::with_capacity(3)
            .spacing(20)
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .push(widget::icon::from_name("emblem-default-symbolic").size(96).icon())
            .push(widget::text::title1("Your drive is ready"))
            .push(widget::text::body("Remove it and boot from it to install DraftOS.").center())
            .into()
    }

    fn footer(&self) -> Element<'_, Message> {
        let hide_back =
            matches!(self.step, Step::Source | Step::Download | Step::Writing | Step::Done);
        let left: Element<'_, Message> = if hide_back {
            widget::Space::new().into()
        } else {
            widget::button::standard("Back").on_press(Message::Back).into()
        };
        let left = widget::container(left).width(Length::Fixed(140.0));

        let (label, msg) = match self.step {
            Step::Edition => ("Download", self.can_advance().then_some(Message::Next)),
            Step::Download => ("Downloading…", None),
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

/// Wrap content in a standard card container.
fn card(content: Element<'_, Message>) -> Element<'_, Message> {
    widget::container(content)
        .padding(16)
        .width(Length::Fill)
        .class(cosmic::theme::Container::Card)
        .into()
}

/// Fetch and parse the editions manifest with `curl`.
async fn load_manifest(url: String) -> Result<Vec<Edition>, String> {
    let output = tokio::process::Command::new("curl")
        .args(["-fsSL", "--retry", "3", &url])
        .output()
        .await
        .map_err(|e| format!("could not fetch the edition list: {e}"))?;
    if !output.status.success() {
        return Err("could not reach the edition list".into());
    }
    let manifest: Manifest =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("bad edition list: {e}"))?;
    Ok(manifest.editions)
}

/// `~/.cache` (or `$XDG_CACHE_HOME`) — real disk, unlike tmpfs `/tmp`.
fn dirs_cache() -> std::path::PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            std::path::PathBuf::from(home).join(".cache")
        })
}

/// Free bytes on the filesystem holding `path` (via `df`, no extra deps).
fn free_space_bytes(path: &std::path::Path) -> Option<u64> {
    let out = std::process::Command::new("df")
        .args(["--output=avail", "-B1"])
        .arg(path)
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .nth(1)?
        .trim()
        .parse()
        .ok()
}

/// Download `url` to `dest` with `curl` (progress is tracked by polling the file).
async fn download_iso(url: String, dest: String) -> Result<String, String> {
    if url.is_empty() {
        return Err("this edition has no download yet".into());
    }
    let status = tokio::process::Command::new("curl")
        .args(["-fL", "--retry", "3", "-o", &dest, &url])
        .status()
        .await
        .map_err(|e| format!("could not start the download: {e}"))?;
    if status.success() {
        Ok(dest)
    } else {
        Err("the download failed (is the release published yet?)".into())
    }
}

/// Write the ISO to the device as root (pkexec prompts for authentication).
///
/// The desktop may have automounted the stick's partitions; writing through a
/// mounted device corrupts the image, so they are unmounted first in the same
/// privileged call. `$0`/`$1` are passed as arguments — no shell interpolation
/// of the paths.
async fn write_iso(iso: String, device: String) -> Result<(), String> {
    let script = r#"for p in "$1"?*; do umount "$p" 2>/dev/null || true; done
exec dd if="$0" of="$1" bs=4M conv=fsync"#;
    let output = tokio::process::Command::new("pkexec")
        .args(["bash", "-c", script, &iso, &device])
        .output()
        .await
        .map_err(|e| format!("could not start the writer: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        // dd ends its stderr with throughput statistics; the actual error is the
        // last line that is NOT one of those.
        let stderr = String::from_utf8_lossy(&output.stderr);
        let last = stderr
            .lines()
            .rev()
            .map(str::trim)
            .find(|l| {
                !l.is_empty()
                    && !l.contains("records in")
                    && !l.contains("records out")
                    && !l.contains("copied,")
            })
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
