//! Read-only detection of removable drives (never the system disk).

use std::process::Command;

/// A removable/USB block device the ISO can be written to.
#[derive(Debug, Clone)]
pub struct DriveInfo {
    pub name: String,
    pub size: String,
    pub model: String,
}

impl DriveInfo {
    pub fn device(&self) -> String {
        format!("/dev/{}", self.name)
    }

    /// e.g. "SanDisk Ultra — 32G" or "sdb — 16G".
    pub fn label(&self) -> String {
        if self.model.is_empty() {
            format!("{} — {}", self.name, self.size)
        } else {
            format!("{} — {}", self.model, self.size)
        }
    }
}

/// Enumerate **removable** whole disks (USB sticks, SD cards). Fixed/internal
/// disks are deliberately excluded so we can't offer to erase the system drive.
pub fn detect_removable_drives() -> Vec<DriveInfo> {
    let Ok(output) = Command::new("lsblk")
        .args(["-dn", "-P", "-o", "NAME,SIZE,TYPE,RM,HOTPLUG,TRAN,MODEL"])
        .output()
    else {
        return Vec::new();
    };

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse)
        .filter(|d| {
            d.type_val == "disk"
                && (d.rm || d.hotplug || d.tran == "usb")
                && !is_virtual(&d.name)
        })
        .map(|d| DriveInfo { name: d.name, size: d.size, model: d.model })
        .collect()
}

fn is_virtual(name: &str) -> bool {
    ["zram", "loop", "ram", "sr", "fd"].iter().any(|p| name.starts_with(p))
}

struct Row {
    name: String,
    size: String,
    model: String,
    type_val: String,
    rm: bool,
    hotplug: bool,
    tran: String,
}

/// Parse one `lsblk -P` line into a [`Row`].
fn parse(line: &str) -> Option<Row> {
    let mut name = None;
    let (mut size, mut model, mut type_val, mut tran) =
        (String::new(), String::new(), String::new(), String::new());
    let (mut rm, mut hotplug) = (false, false);

    for token in split_pairs(line) {
        let (key, value) = token.split_once('=')?;
        let value = value.trim_matches('"').to_string();
        match key {
            "NAME" => name = Some(value),
            "SIZE" => size = value,
            "TYPE" => type_val = value,
            "MODEL" => model = value.trim().to_string(),
            "TRAN" => tran = value,
            "RM" => rm = value == "1",
            "HOTPLUG" => hotplug = value == "1",
            _ => {}
        }
    }
    Some(Row { name: name?, size, model, type_val, rm, hotplug, tran })
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
    fn removable_flags_and_virtual_filter() {
        let usb = parse(r#"NAME="sdb" SIZE="32G" TYPE="disk" RM="1" HOTPLUG="1" TRAN="usb" MODEL="Ultra""#).unwrap();
        assert!(usb.rm && usb.type_val == "disk");
        assert!(is_virtual("zram0"));
        assert!(!is_virtual("sdb"));
    }

    #[test]
    fn label_falls_back_to_name() {
        let d = DriveInfo { name: "sdb".into(), size: "32G".into(), model: String::new() };
        assert_eq!(d.label(), "sdb — 32G");
        assert_eq!(d.device(), "/dev/sdb");
    }
}
