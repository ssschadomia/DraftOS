//! `draftos-install` — the DraftOS install engine CLI.
//!
//! Reads an install request as JSON, plans it, and dry-runs it by default.
//! Pass `--commit` (only in a live environment) to actually install.
//!
//! Examples:
//!   draftos-install --config req.json            # dry-run: print the plan
//!   draftos-install --config req.json --commit   # execute (live ISO only)
//!   cat req.json | draftos-install               # config from stdin

use std::io::Read;

use clap::Parser;

#[derive(Parser)]
#[command(name = "draftos-install", about = "DraftOS install engine")]
struct Cli {
    /// Path to the install request JSON, or "-" for stdin.
    #[arg(long, default_value = "-")]
    config: String,
    /// Actually perform the installation (default is a safe dry-run).
    #[arg(long)]
    commit: bool,
    /// Root of the target mount.
    #[arg(long, default_value = "/mnt")]
    target_root: String,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let json = if cli.config == "-" {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s)?;
        s
    } else {
        std::fs::read_to_string(&cli.config)?
    };

    let request: draftos_install::InstallRequest = serde_json::from_str(&json)?;
    let steps = draftos_install::plan(&request)?;

    let destructive = steps.iter().filter(|s| s.destructive).count();
    eprintln!(
        "Planned {} steps ({} destructive).{}",
        steps.len(),
        destructive,
        if cli.commit { "" } else { " DRY-RUN — no changes will be made." }
    );

    draftos_install::exec::execute(&steps, cli.commit, &cli.target_root)
}
