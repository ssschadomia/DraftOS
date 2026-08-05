//! The step model the planner produces and the executor runs.
//!
//! A [`Step`] is pure data — a described operation — so a full install can be
//! planned, inspected and unit-tested without touching a disk.

use crate::config::Secret;

/// Progress phases, mirrored by the installer UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Partition,
    Format,
    BaseSystem,
    Configure,
    Bootloader,
    Users,
    Finish,
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Phase::Partition => "Preparing the disk",
            Phase::Format => "Creating filesystems",
            Phase::BaseSystem => "Installing the base system",
            Phase::Configure => "Configuring the system",
            Phase::Bootloader => "Installing the bootloader",
            Phase::Users => "Creating your account",
            Phase::Finish => "Finishing up",
        }
    }
}

/// A single operation.
#[derive(Debug, Clone)]
pub enum Op {
    /// Run a command in the live environment (as root). `stdin`, if present, is
    /// fed to the process; secret stdin (passwords) is redacted in output.
    Run { argv: Vec<String>, stdin: Option<Secret> },
    /// Write a file into the new system (`path` is relative to the install root,
    /// e.g. `/etc/locale.conf`).
    WriteFile { path: String, contents: String, mode: u32 },
}

/// A described, ordered step of the install.
#[derive(Debug, Clone)]
pub struct Step {
    pub phase: Phase,
    pub title: String,
    pub op: Op,
    /// True for irreversible operations (partitioning, mkfs, luksFormat).
    pub destructive: bool,
}

impl Step {
    /// A command in the live environment.
    pub fn run(phase: Phase, title: impl Into<String>, argv: &[&str]) -> Step {
        Step {
            phase,
            title: title.into(),
            op: Op::Run { argv: argv.iter().map(|s| s.to_string()).collect(), stdin: None },
            destructive: false,
        }
    }

    /// A `bash -c` script in the live environment.
    pub fn sh(phase: Phase, title: impl Into<String>, script: impl Into<String>) -> Step {
        Step {
            phase,
            title: title.into(),
            op: Op::Run {
                argv: vec!["bash".into(), "-c".into(), script.into()],
                stdin: None,
            },
            destructive: false,
        }
    }

    /// A command run inside the target via `arch-chroot`.
    pub fn chroot(phase: Phase, title: impl Into<String>, argv: &[&str]) -> Step {
        let mut full = vec!["arch-chroot".to_string(), "/mnt".to_string()];
        full.extend(argv.iter().map(|s| s.to_string()));
        Step {
            phase,
            title: title.into(),
            op: Op::Run { argv: full, stdin: None },
            destructive: false,
        }
    }

    /// A chroot command fed secret stdin (e.g. `chpasswd`).
    pub fn chroot_secret(phase: Phase, title: impl Into<String>, argv: &[&str], stdin: Secret) -> Step {
        let mut full = vec!["arch-chroot".to_string(), "/mnt".to_string()];
        full.extend(argv.iter().map(|s| s.to_string()));
        Step {
            phase,
            title: title.into(),
            op: Op::Run { argv: full, stdin: Some(stdin) },
            destructive: false,
        }
    }

    /// A command fed secret stdin in the live environment (e.g. `cryptsetup`).
    pub fn run_secret(phase: Phase, title: impl Into<String>, argv: &[&str], stdin: Secret) -> Step {
        Step {
            phase,
            title: title.into(),
            op: Op::Run { argv: argv.iter().map(|s| s.to_string()).collect(), stdin: Some(stdin) },
            destructive: false,
        }
    }

    /// Write a file into the target.
    pub fn write(phase: Phase, title: impl Into<String>, path: impl Into<String>, contents: impl Into<String>, mode: u32) -> Step {
        Step {
            phase,
            title: title.into(),
            op: Op::WriteFile { path: path.into(), contents: contents.into(), mode },
            destructive: false,
        }
    }

    /// Mark this step as destructive (irreversible).
    pub fn danger(mut self) -> Step {
        self.destructive = true;
        self
    }

    /// A one-line, secret-free summary for logs / dry-run.
    pub fn summary(&self) -> String {
        match &self.op {
            Op::Run { argv, stdin } => {
                let s = if stdin.is_some() { "  <stdin: redacted>" } else { "" };
                format!("run: {}{s}", argv.join(" "))
            }
            Op::WriteFile { path, contents, mode } => {
                format!("write: {path} (mode {mode:o}, {} bytes)", contents.len())
            }
        }
    }
}
