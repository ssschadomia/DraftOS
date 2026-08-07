//! Read-only probes of the host system.
//!
//! This module only *reads* system state (e.g. block devices); it never modifies
//! anything. Destructive operations belong to the future install engine.

use std::process::Command;

/// A whole-disk block device the installer may target.
#[derive(Debug, Clone)]
pub struct DiskInfo {
    /// Kernel name, e.g. `nvme0n1` or `sda`.
    pub name: String,
    /// Human-readable size, e.g. `476.9G`.
    pub size: String,
    /// Model string, empty if the kernel reports none.
    pub model: String,
}

impl DiskInfo {
    /// Device path, e.g. `/dev/nvme0n1`.
    pub fn device(&self) -> String {
        format!("/dev/{}", self.name)
    }

    /// One-line label for the picker: "Samsung SSD — 476.9G" or just the size.
    pub fn label(&self) -> String {
        if self.model.is_empty() {
            format!("{} — {}", self.name, self.size)
        } else {
            format!("{} — {}", self.model, self.size)
        }
    }
}

/// A partition on a disk, offered on the manual-partitioning step.
#[derive(Debug, Clone)]
pub struct PartInfo {
    pub name: String,
    pub size: String,
    /// Filesystem type, empty if unformatted.
    pub fstype: String,
}

impl PartInfo {
    pub fn device(&self) -> String {
        format!("/dev/{}", self.name)
    }

    /// e.g. "nvme0n1p1 — 512M (vfat)" or "sda2 — 40G (unformatted)".
    pub fn label(&self) -> String {
        let fs = if self.fstype.is_empty() { "unformatted".to_string() } else { self.fstype.clone() };
        format!("{} — {} ({})", self.name, self.size, fs)
    }
}

/// Enumerate real partitions via `lsblk`, excluding virtual devices. Empty if
/// `lsblk` is unavailable.
pub fn detect_partitions() -> Vec<PartInfo> {
    let output = Command::new("lsblk")
        .args(["-n", "-P", "-o", "NAME,SIZE,TYPE,FSTYPE"])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_pairs)
        .filter(|d| d.type_val == "part" && !is_virtual(&d.name))
        .map(|d| PartInfo { name: d.name, size: d.size, fstype: d.fstype })
        .collect()
}

/// Enumerate installable whole disks via `lsblk`, excluding virtual devices
/// (zram, loop, ram, optical). Returns an empty vec if `lsblk` is unavailable.
pub fn detect_disks() -> Vec<DiskInfo> {
    let output = Command::new("lsblk")
        .args(["-dn", "-P", "-o", "NAME,SIZE,TYPE,MODEL"])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let boot = live_boot_disk();

    stdout
        .lines()
        .filter_map(parse_pairs)
        .filter(|d| d.type_val == "disk" && !is_virtual(&d.name))
        .filter(|d| Some(&d.name) != boot.as_ref())
        .map(|d| DiskInfo { name: d.name, size: d.size, model: d.model })
        .collect()
}

/// The disk the live medium itself is running from (e.g. the installer USB) —
/// it must never be offered as an install target: `sgdisk --zap-all` on it would
/// destroy the running system mid-install. Resolves the parent disk of
/// `/run/archiso/bootmnt`; `None` outside a live environment.
fn live_boot_disk() -> Option<String> {
    let src = Command::new("findmnt")
        .args(["-no", "SOURCE", "/run/archiso/bootmnt"])
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let src = String::from_utf8_lossy(&src.stdout).trim().to_string();
    if src.is_empty() {
        return None;
    }
    let pk = Command::new("lsblk")
        .args(["-no", "PKNAME", &src])
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let parent = String::from_utf8_lossy(&pk.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    if parent.is_empty() {
        // `src` may already be the whole disk (rare) — fall back to its basename.
        src.rsplit('/').next().map(str::to_string)
    } else {
        Some(parent)
    }
}

/// All installable UTF-8 locales.
///
/// Prefers glibc's canonical `/usr/share/i18n/SUPPORTED` — the full list of
/// locales you can *install* (~340). `localectl list-locales` only reports
/// already-generated locales, which on a fresh live system is often just
/// `C.UTF-8`, so it's used only as a fallback.
pub fn detect_locales() -> Vec<String> {
    if let Ok(content) = std::fs::read_to_string("/usr/share/i18n/SUPPORTED") {
        let list: Vec<String> = content
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    return None;
                }
                // Lines are "<locale> <charmap>", e.g. "en_US.UTF-8 UTF-8".
                let mut parts = line.split_whitespace();
                let name = parts.next()?;
                let charmap = parts.next().unwrap_or("");
                (charmap == "UTF-8" && !name.starts_with("C.") && !name.starts_with("POSIX"))
                    .then(|| name.to_string())
            })
            .collect();
        if !list.is_empty() {
            return list;
        }
    }
    Command::new("localectl")
        .arg("list-locales")
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(str::trim)
                .filter(|l| l.contains("UTF-8") && !l.starts_with("C.") && !l.starts_with("POSIX"))
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

/// The system's current locale, from `$LANG` (e.g. `en_US.UTF-8`).
pub fn current_locale() -> Option<String> {
    std::env::var("LANG").ok().filter(|l| l.contains("UTF-8"))
}

/// All IANA time zones known to the system, via `timedatectl list-timezones`.
/// Empty if the command is unavailable.
pub fn detect_timezones() -> Vec<String> {
    Command::new("timedatectl")
        .arg("list-timezones")
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// The system's current time zone, e.g. `Europe/Moscow`, if it can be read.
pub fn current_timezone() -> Option<String> {
    let out = Command::new("timedatectl")
        .args(["show", "-p", "Timezone", "--value"])
        .output()
        .ok()?;
    let tz = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!tz.is_empty()).then_some(tz)
}

/// All XKB keyboard layouts as `(code, description)`, parsed from the X11 rules
/// list (`evdev.lst`); falls back to bare codes from `localectl`. Empty if neither
/// source is available.
pub fn detect_keyboard_layouts() -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Ok(content) = std::fs::read_to_string("/usr/share/X11/xkb/rules/evdev.lst") {
        let mut in_section = false;
        for line in content.lines() {
            if line.starts_with("! layout") {
                in_section = true;
                continue;
            }
            if line.starts_with('!') {
                in_section = false;
                continue;
            }
            if !in_section {
                continue;
            }
            let line = line.trim();
            if let Some((code, desc)) = line.split_once(char::is_whitespace) {
                out.push((code.trim().to_string(), desc.trim().to_string()));
            }
        }
    }
    if out.is_empty() {
        if let Ok(o) = Command::new("localectl").arg("list-x11-keymap-layouts").output() {
            for l in String::from_utf8_lossy(&o.stdout).lines() {
                let l = l.trim();
                if !l.is_empty() {
                    out.push((l.to_string(), l.to_string()));
                }
            }
        }
    }
    out
}

/// The system's current keyboard layouts, from `localectl status`
/// (`X11 Layout: us,ru` → `["us", "ru"]`).
pub fn current_layouts() -> Vec<String> {
    let Some(o) = Command::new("localectl").arg("status").output().ok() else {
        return Vec::new();
    };
    for line in String::from_utf8_lossy(&o.stdout).lines() {
        if let Some(v) = line.trim().strip_prefix("X11 Layout:") {
            return v
                .trim()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }
    Vec::new()
}

fn is_virtual(name: &str) -> bool {
    ["zram", "loop", "ram", "sr", "fd"]
        .iter()
        .any(|p| name.starts_with(p))
}

struct Parsed {
    name: String,
    size: String,
    model: String,
    fstype: String,
    type_val: String,
}

/// Parse one `lsblk -P` line, e.g. `NAME="x" SIZE="y" TYPE="disk" MODEL="z"`.
fn parse_pairs(line: &str) -> Option<Parsed> {
    let mut name = None;
    let mut size = String::new();
    let mut model = String::new();
    let mut fstype = String::new();
    let mut type_val = String::new();

    for token in split_pairs(line) {
        let (key, value) = token.split_once('=')?;
        let value = value.trim_matches('"').to_string();
        match key {
            "NAME" => name = Some(value),
            "SIZE" => size = value,
            "TYPE" => type_val = value,
            "MODEL" => model = value.trim().to_string(),
            "FSTYPE" => fstype = value.trim().to_string(),
            _ => {}
        }
    }
    Some(Parsed { name: name?, size, model, fstype, type_val })
}

/// Split a `-P` line into `KEY="value"` tokens, respecting quoted spaces.
fn split_pairs(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for c in line.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                current.push(c);
            }
            ' ' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_disk_line_with_spaced_model() {
        let p = parse_pairs(r#"NAME="sda" SIZE="500G" TYPE="disk" MODEL="Samsung SSD 860""#).unwrap();
        assert_eq!(p.name, "sda");
        assert_eq!(p.size, "500G");
        assert_eq!(p.model, "Samsung SSD 860");
        assert_eq!(p.type_val, "disk");
    }

    #[test]
    fn virtual_devices_are_flagged() {
        assert!(is_virtual("zram0"));
        assert!(is_virtual("loop3"));
        assert!(!is_virtual("nvme0n1"));
        assert!(!is_virtual("sda"));
    }

    #[test]
    fn empty_model_falls_back_to_name_in_label() {
        let d = DiskInfo { name: "sda".into(), size: "500G".into(), model: String::new() };
        assert_eq!(d.label(), "sda — 500G");
        assert_eq!(d.device(), "/dev/sda");
    }
}
