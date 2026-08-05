//! Execute a plan — or, by default, dry-run it.
//!
//! Safety: destructive execution is refused unless we're clearly in a DraftOS
//! live environment (`/run/archiso` present) or explicitly forced with
//! `DRAFTOS_INSTALL_FORCE=1` (intended only for a VM). This makes it impossible
//! to wipe a normal system by accident.

use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::step::{Op, Step};

/// Guard: only allow real execution in a live installer environment.
pub fn ensure_commit_allowed() -> anyhow::Result<()> {
    if Path::new("/run/archiso").exists() {
        return Ok(());
    }
    if std::env::var("DRAFTOS_INSTALL_FORCE").as_deref() == Ok("1") {
        return Ok(());
    }
    anyhow::bail!(
        "refusing to execute: not a DraftOS live environment (no /run/archiso). \
         Set DRAFTOS_INSTALL_FORCE=1 only inside a VM/live ISO to override."
    );
}

/// Run (or dry-run) every step. In dry-run nothing touches the system; each step
/// is printed with a secret-free summary. Progress is emitted on stdout as
/// `PROGRESS <pct> <phase>` for a front-end to parse; human logs go to stderr.
pub fn execute(steps: &[Step], commit: bool, target_root: &str) -> anyhow::Result<()> {
    if commit {
        ensure_commit_allowed()?;
    }
    let total = steps.len().max(1);
    let mut current_phase = None;

    for (i, step) in steps.iter().enumerate() {
        if current_phase != Some(step.phase) {
            current_phase = Some(step.phase);
            eprintln!("\n== {} ==", step.phase.label());
        }
        let pct = i * 100 / total;
        println!("PROGRESS {pct} {}", step.phase.label());
        eprintln!("[{}/{}] {}", i + 1, total, step.title);

        if !commit {
            eprintln!("   DRY-RUN  {}", step.summary());
            continue;
        }
        run_step(step, target_root)
            .map_err(|e| anyhow::anyhow!("step '{}' failed: {e}", step.title))?;
    }
    println!("PROGRESS 100 done");
    eprintln!("\n{}", if commit { "Installation complete." } else { "Dry-run complete — no changes made." });
    Ok(())
}

fn run_step(step: &Step, target_root: &str) -> anyhow::Result<()> {
    match &step.op {
        Op::Run { argv, stdin } => {
            let mut cmd = Command::new(&argv[0]);
            cmd.args(&argv[1..]);
            if stdin.is_some() {
                cmd.stdin(Stdio::piped());
            }
            let mut child = cmd.spawn()?;
            if let Some(secret) = stdin {
                let mut sink = child.stdin.take().expect("stdin piped");
                // A trailing newline terminates the line for chpasswd; cryptsetup
                // reads the first line and strips it, so this is safe for both.
                writeln!(sink, "{}", secret.expose())?;
            }
            let status = child.wait()?;
            if !status.success() {
                anyhow::bail!("command exited with {status}: {}", argv.join(" "));
            }
            Ok(())
        }
        Op::WriteFile { path, contents, mode } => {
            let full = format!("{target_root}{path}");
            if let Some(parent) = Path::new(&full).parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&full, contents)?;
            std::fs::set_permissions(&full, std::fs::Permissions::from_mode(*mode))?;
            Ok(())
        }
    }
}
