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

    stdout
        .lines()
        .filter_map(parse_pairs)
        .filter(|d| d.type_is_disk && !is_virtual(&d.name))
        .map(|d| DiskInfo { name: d.name, size: d.size, model: d.model })
        .collect()
}

/// All UTF-8 locales the system knows, via `localectl list-locales` (excluding
/// the `C`/`POSIX` pseudo-locales). Empty if the command is unavailable.
pub fn detect_locales() -> Vec<String> {
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

fn is_virtual(name: &str) -> bool {
    ["zram", "loop", "ram", "sr", "fd"]
        .iter()
        .any(|p| name.starts_with(p))
}

struct Parsed {
    name: String,
    size: String,
    model: String,
    type_is_disk: bool,
}

/// Parse one `lsblk -P` line: `NAME="x" SIZE="y" TYPE="disk" MODEL="z"`.
fn parse_pairs(line: &str) -> Option<Parsed> {
    let mut name = None;
    let mut size = String::new();
    let mut model = String::new();
    let mut is_disk = false;

    for token in split_pairs(line) {
        let (key, value) = token.split_once('=')?;
        let value = value.trim_matches('"').to_string();
        match key {
            "NAME" => name = Some(value),
            "SIZE" => size = value,
            "TYPE" => is_disk = value == "disk",
            "MODEL" => model = value.trim().to_string(),
            _ => {}
        }
    }
    Some(Parsed { name: name?, size, model, type_is_disk: is_disk })
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
        assert!(p.type_is_disk);
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
