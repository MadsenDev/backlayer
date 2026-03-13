use std::process::Command;

use backlayer_types::MonitorInfo;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HyprlandError {
    #[error("failed to execute hyprctl: {0}")]
    CommandFailed(#[from] std::io::Error),
    #[error("hyprctl exited unsuccessfully")]
    NonZeroExit,
    #[error("failed to parse hyprctl monitor output")]
    Parse(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize)]
struct RawMonitor {
    name: String,
    description: String,
    make: String,
    model: String,
    serial: String,
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    scale: f64,
    #[serde(rename = "refreshRate")]
    refresh_rate: f64,
    focused: bool,
    disabled: bool,
}

#[derive(Debug, Deserialize)]
struct RawActiveWindow {
    fullscreen: i32,
    #[serde(rename = "fullscreenClient")]
    fullscreen_client: i32,
}

#[derive(Debug, Default, Clone)]
pub struct HyprlandClient;

impl HyprlandClient {
    pub fn new() -> Self {
        Self
    }

    pub fn discover_monitors(&self) -> Result<Vec<MonitorInfo>, HyprlandError> {
        let output = Command::new("hyprctl").args(["monitors", "-j"]).output()?;

        if !output.status.success() {
            return Err(HyprlandError::NonZeroExit);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_monitors(&stdout)
    }

    pub fn fullscreen_active(&self) -> Result<bool, HyprlandError> {
        let output = Command::new("hyprctl")
            .args(["activewindow", "-j"])
            .output()?;

        if !output.status.success() {
            return Err(HyprlandError::NonZeroExit);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_fullscreen_active(&stdout)
    }

    pub fn parse_monitors(raw: &str) -> Result<Vec<MonitorInfo>, HyprlandError> {
        let parsed: Vec<RawMonitor> = serde_json::from_str(raw)?;

        Ok(parsed
            .into_iter()
            .map(|monitor| MonitorInfo {
                id: Self::monitor_id(
                    &monitor.make,
                    &monitor.model,
                    &monitor.serial,
                    &monitor.description,
                    &monitor.name,
                ),
                output_name: monitor.name,
                description: monitor.description,
                make: monitor.make,
                model: monitor.model,
                serial: normalized_component(&monitor.serial),
                width: monitor.width,
                height: monitor.height,
                x: monitor.x,
                y: monitor.y,
                scale: monitor.scale,
                refresh_rate: monitor.refresh_rate,
                focused: monitor.focused,
                disabled: monitor.disabled,
            })
            .collect())
    }

    pub fn parse_fullscreen_active(raw: &str) -> Result<bool, HyprlandError> {
        let parsed: RawActiveWindow = serde_json::from_str(raw)?;
        Ok(parsed.fullscreen > 0 || parsed.fullscreen_client > 0)
    }

    fn monitor_id(
        make: &str,
        model: &str,
        serial: &str,
        description: &str,
        output_name: &str,
    ) -> String {
        let fingerprint = normalized_component(serial)
            .or_else(|| normalized_component(description))
            .or_else(|| normalized_component(output_name))
            .unwrap_or_else(|| "unknown".to_string());

        format!(
            "hypr:{}:{}:{}",
            slugify(make).unwrap_or_else(|| "unknown".to_string()),
            slugify(model).unwrap_or_else(|| "unknown".to_string()),
            fingerprint
        )
    }
}

fn normalized_component(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
        return None;
    }

    slugify(trimmed)
}

fn slugify(value: &str) -> Option<String> {
    let slug = value
        .chars()
        .flat_map(char::to_lowercase)
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if slug.is_empty() { None } else { Some(slug) }
}

#[cfg(test)]
mod tests {
    use super::HyprlandClient;

    #[test]
    fn parses_hyprctl_monitors_json() {
        let raw = r#"
        [{
            "id": 0,
            "name": "eDP-1",
            "description": "Chimei Innolux Corporation 0x14C9",
            "make": "Chimei Innolux Corporation",
            "model": "0x14C9",
            "serial": "",
            "width": 1920,
            "height": 1080,
            "physicalWidth": 310,
            "physicalHeight": 170,
            "refreshRate": 60.00800,
            "x": 0,
            "y": 0,
            "scale": 1.00,
            "focused": true,
            "disabled": false
        }]
        "#;

        let monitors = HyprlandClient::parse_monitors(raw).expect("valid monitor fixture");
        assert_eq!(monitors.len(), 1);
        assert_eq!(monitors[0].output_name, "eDP-1");
        assert_eq!(monitors[0].width, 1920);
        assert!(monitors[0].focused);
        assert_eq!(
            monitors[0].id,
            "hypr:chimei-innolux-corporation:0x14c9:chimei-innolux-corporation-0x14c9"
        );
    }

    #[test]
    fn keeps_identity_stable_if_output_name_changes() {
        let first = r#"
        [{
            "name": "DP-1",
            "description": "Dell Inc. DELL U2720Q",
            "make": "Dell Inc.",
            "model": "DELL U2720Q",
            "serial": "CN0XYZ123456",
            "width": 3840,
            "height": 2160,
            "x": 0,
            "y": 0,
            "scale": 1.0,
            "refreshRate": 60.0,
            "focused": false,
            "disabled": false
        }]
        "#;
        let second = r#"
        [{
            "name": "DP-3",
            "description": "Dell Inc. DELL U2720Q",
            "make": "Dell Inc.",
            "model": "DELL U2720Q",
            "serial": "CN0XYZ123456",
            "width": 3840,
            "height": 2160,
            "x": 0,
            "y": 0,
            "scale": 1.0,
            "refreshRate": 60.0,
            "focused": false,
            "disabled": false
        }]
        "#;

        let first_monitor = HyprlandClient::parse_monitors(first)
            .expect("first fixture should parse")
            .remove(0);
        let second_monitor = HyprlandClient::parse_monitors(second)
            .expect("second fixture should parse")
            .remove(0);

        assert_eq!(first_monitor.id, second_monitor.id);
        assert_ne!(first_monitor.output_name, second_monitor.output_name);
    }

    #[test]
    fn parses_active_window_fullscreen_state() {
        let fullscreen = r#"
        {
            "fullscreen": 2,
            "fullscreenClient": 0
        }
        "#;
        let normal = r#"
        {
            "fullscreen": 0,
            "fullscreenClient": 0
        }
        "#;

        assert!(
            HyprlandClient::parse_fullscreen_active(fullscreen)
                .expect("fullscreen fixture should parse")
        );
        assert!(
            !HyprlandClient::parse_fullscreen_active(normal).expect("normal fixture should parse")
        );
    }
}
