use std::process::Command;

pub(crate) struct SysInfo {
    pub ipv4: Option<String>,
    pub ipv6: Option<String>,
    pub battery_level: Option<u8>,
}

impl SysInfo {
    pub(crate) fn new() -> Self {
        Self {
            ipv4: SysInfo::get_ipv4_address(),
            ipv6: SysInfo::get_ipv6_address(),
            battery_level: SysInfo::get_battery_level(),
        }
    }

    fn get_ipv4_address() -> Option<String> {
        let output = Command::new("ipconfig")
            .arg("getifaddr")
            .arg("en0")
            .output()
            .ok()?;

        if output.status.success() {
            String::from_utf8(output.stdout)
                .ok()
                .map(|s| s.trim().to_string())
        } else {
            None
        }
    }

    fn get_ipv6_address() -> Option<String> {
        let output = Command::new("networksetup")
            .arg("-getinfo")
            .arg("Wi-Fi")
            .output()
            .ok()?;
        if output.status.success() {
            String::from_utf8(output.stdout).ok().map(|s| {
                let ipv6_line = s.lines().nth(6).expect("no ipv6 addr list");
                let addr = ipv6_line
                    .split_whitespace()
                    .last()
                    .expect("no ipv6 address");
                if addr == "none" {
                    None
                } else {
                    Some(addr.to_string())
                }
            })?
        } else {
            None
        }
    }

    fn get_battery_level() -> Option<u8> {
        let output = Command::new("pmset").arg("-g").arg("batt").output().ok()?;

        let text = String::from_utf8(output.stdout).ok()?;

        for line in text.lines() {
            if line.contains("InternalBattery") {
                if let Some(percent_pos) = line.find('%') {
                    let start = line[..percent_pos].rfind(|c: char| !c.is_numeric())?;
                    let percent_str = &line[start + 1..percent_pos];
                    return percent_str.parse().ok();
                }
            }
        }

        None
    }
}
