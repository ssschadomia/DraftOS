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

/// Guard: only allow real execution in a live installer environment, and only
/// when booted in UEFI mode — the plan installs systemd-boot (UEFI-only), and
/// on a BIOS boot `bootctl install` merely warns, so without this gate a BIOS
/// machine would be wiped, report success, and be unbootable after reboot.
pub fn ensure_commit_allowed() -> anyhow::Result<()> {
    if std::env::var("DRAFTOS_INSTALL_FORCE").as_deref() == Ok("1") {
        return Ok(());
    }
    if !Path::new("/run/archiso").exists() {
        anyhow::bail!(
            "refusing to execute: not a DraftOS live environment (no /run/archiso). \
             Set DRAFTOS_INSTALL_FORCE=1 only inside a VM/live ISO to override."
        );
    }
    if !Path::new("/sys/firmware/efi").exists() {
        anyhow::bail!(
            "this computer booted in BIOS/legacy mode, but DraftOS requires UEFI. \
             Reboot, enter the firmware settings, enable UEFI boot for the USB \
             medium, and run the installer again. No changes were made."
        );
    }
    Ok(())
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
        // Machine-readable progress for a front-end: "PROGRESS <pct> <step title>".
        println!("PROGRESS {pct} {}", step.title);
        eprintln!("[{}/{}] {}", i + 1, total, step.title);

        if !commit {
            eprintln!("   DRY-RUN  {}", step.summary());
            continue;
        }
        if let Err(e) = run_step(step, target_root) {
            // Best-effort teardown so a retry starts from a clean slate instead
            // of failing on busy mounts / an open LUKS mapper.
            teardown();
            anyhow::bail!("step '{}' failed: {e}", step.title);
        }
    }
    println!("PROGRESS 100 done");
    eprintln!("\n{}", if commit { "Installation complete." } else { "Dry-run complete — no changes made." });
    Ok(())
}

/// Release everything a partial install may hold. Errors are ignored — this is
/// cleanup after a failure, not a step of its own.
fn teardown() {
    eprintln!("Cleaning up after the failure...");
    for argv in [
        &["umount", "-R", "/mnt"][..],
        &["cryptsetup", "close", "cryptroot"][..],
    ] {
        let _ = Command::new(argv[0])
            .args(&argv[1..])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn run_step(step: &Step, target_root: &str) -> anyhow::Result<()> {
    match &step.op {
        Op::Run { argv, stdin } => {
            let mut cmd = Command::new(&argv[0]);
            cmd.args(&argv[1..]);
            if stdin.is_some() {
                cmd.stdin(Stdio::piped());
            }
            // Our stdout is the machine-readable PROGRESS channel a front-end
            // parses; pacstrap/pacman/bootctl chatter must not interleave into
            // it. Pipe child stdout and forward it to stderr with the logs.
            cmd.stdout(Stdio::piped());
            let mut child = cmd.spawn()?;
            if let Some(secret) = stdin {
                let mut sink = child.stdin.take().expect("stdin piped");
                // A trailing newline terminates the line for chpasswd; cryptsetup
                // reads the first line and strips it, so this is safe for both.
                writeln!(sink, "{}", secret.expose())?;
            }
            let forwarder = child.stdout.take().map(|mut out| {
                std::thread::spawn(move || {
                    let _ = std::io::copy(&mut out, &mut std::io::stderr());
                })
            });
            let status = child.wait()?;
            if let Some(t) = forwarder {
                let _ = t.join();
            }
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
