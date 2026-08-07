//! `draftos` — the DraftOS command-line control tool.
//!
//! One entry point for keeping a DraftOS system healthy and yours: snapshotted
//! system updates, one-command rollback, and a discoverable library of task
//! recipes (`draftos do`). System-changing commands are snapshot-guarded and
//! run through `sudo` when they need root; `--dry-run` prints the plan instead.

mod recipes;

use std::process::Command;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use draftos_common::{brand, Edition};

#[derive(Parser)]
#[command(name = "draftos", version, about = "DraftOS system control")]
struct Cli {
    /// Print the commands that would run instead of running them.
    #[arg(long, global = true)]
    dry_run: bool,

    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Update the whole system, wrapped in before/after Btrfs snapshots.
    Update,
    /// Roll the system back to an earlier snapshot.
    Rollback {
        /// The snapshot number to roll back to. Omit to list snapshots.
        number: Option<u32>,
        /// List available snapshots and exit.
        #[arg(long)]
        list: bool,
    },
    /// Run a task recipe, or list them with no argument.
    Do {
        /// The recipe name. Omit to list all available recipes.
        recipe: Option<String>,
    },
    /// Show what this DraftOS system is.
    Info,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let ctx = Ctx { dry_run: cli.dry_run, root: is_root() };
    match cli.command {
        Cmd::Update => update(&ctx),
        Cmd::Rollback { number, list } => rollback(&ctx, number, list),
        Cmd::Do { recipe } => run_do(&ctx, recipe),
        Cmd::Info => info(),
    }
}

/// Shared run context.
struct Ctx {
    dry_run: bool,
    root: bool,
}

impl Ctx {
    /// Run a shell command, escalating with `sudo` when `need_root` and we are
    /// not already root. In dry-run mode the command is printed, not executed.
    fn run(&self, need_root: bool, cmd: &str) -> Result<()> {
        let sudo = need_root && !self.root;
        let shown = if sudo { format!("sudo {cmd}") } else { cmd.to_string() };
        if self.dry_run {
            println!("  → {shown}");
            return Ok(());
        }
        println!("\x1b[2m$ {shown}\x1b[0m");
        let status = if sudo {
            Command::new("sudo").args(["sh", "-c", cmd]).status()
        } else {
            Command::new("sh").args(["-c", cmd]).status()
        }
        .with_context(|| format!("failed to launch: {shown}"))?;
        if !status.success() {
            bail!("command failed ({status}): {shown}");
        }
        Ok(())
    }
}

/// `draftos update` — pre-snapshot, upgrade, post-snapshot.
fn update(ctx: &Ctx) -> Result<()> {
    guard_draftos(ctx)?;
    let has_snapper = which("snapper");
    if !has_snapper {
        eprintln!("draftos: snapper not found — updating without snapshots");
    }
    println!("Updating DraftOS…");
    if has_snapper {
        ctx.run(true, "snapper -c root create -t single -c number -d 'draftos update: before'")?;
    }
    ctx.run(true, "pacman -Syu")?;
    if has_snapper {
        ctx.run(true, "snapper -c root create -t single -c number -d 'draftos update: after'")?;
    }
    println!("Done. Roll back with: draftos rollback --list");
    Ok(())
}

/// `draftos rollback` — list snapshots, or roll back to a chosen one.
fn rollback(ctx: &Ctx, number: Option<u32>, list: bool) -> Result<()> {
    guard_draftos(ctx)?;
    if !which("snapper") {
        bail!("snapper is not installed — cannot manage snapshots");
    }
    match (list, number) {
        (true, _) | (false, None) => ctx.run(true, "snapper -c root list"),
        (false, Some(n)) => {
            ctx.run(true, &format!("snapper -c root rollback {n}"))?;
            println!("Rolled back to snapshot {n}. Reboot to boot into it.");
            Ok(())
        }
    }
}

/// `draftos do` — list recipes, or run one by name.
fn run_do(ctx: &Ctx, recipe: Option<String>) -> Result<()> {
    match recipe {
        None => {
            let all = recipes::load_all();
            if all.is_empty() {
                println!("No recipes found. (Looked in {})", recipes::search_dirs()
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", "));
                return Ok(());
            }
            println!("Available recipes ({} found):\n", all.len());
            let width = all.iter().map(|r| r.name.len()).max().unwrap_or(0);
            for r in all {
                println!("  {:width$}  {}", r.name, r.description, width = width);
            }
            println!("\nRun one with:  draftos do <name>");
            Ok(())
        }
        Some(name) => {
            let r = recipes::find(&name)
                .with_context(|| format!("no recipe named '{name}' (try: draftos do)"))?;
            println!("{}  —  {}", r.name, r.description);
            for cmd in &r.commands {
                ctx.run(r.root, cmd)?;
            }
            println!("Recipe '{}' complete.", r.name);
            Ok(())
        }
    }
}

/// `draftos info` — identity and edition.
fn info() -> Result<()> {
    let edition = Edition::detect();
    println!("{}  {}", brand::PRETTY_NAME, brand::VERSION);
    println!("  edition:  {}", edition.id());
    println!("  home:     {}", brand::HOME_URL);
    Ok(())
}

/// Refuse system-changing commands off a real DraftOS install, unless dry-run.
fn guard_draftos(ctx: &Ctx) -> Result<()> {
    if ctx.dry_run {
        return Ok(());
    }
    let id = os_release_id();
    if id.as_deref() != Some("draftos") {
        bail!(
            "this doesn't look like a DraftOS system (os-release ID = {}). \
             Re-run with --dry-run to preview the commands.",
            id.as_deref().unwrap_or("unknown")
        );
    }
    Ok(())
}

fn os_release_id() -> Option<String> {
    for path in ["/etc/os-release", "/usr/lib/os-release"] {
        if let Ok(text) = std::fs::read_to_string(path) {
            for line in text.lines() {
                if let Some(v) = line.strip_prefix("ID=") {
                    return Some(v.trim_matches('"').to_string());
                }
            }
        }
    }
    None
}

/// Is a program on PATH?
fn which(prog: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {prog} >/dev/null 2>&1")])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Are we running as root (euid 0)?
fn is_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .map(|o| o.stdout.starts_with(b"0"))
        .unwrap_or(false)
}
