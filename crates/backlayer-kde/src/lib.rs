use backlayer_types::{CompositorClient, MonitorInfo};
use smithay_client_toolkit::{
    delegate_output, delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
};
use thiserror::Error;
use wayland_client::{Connection, QueueHandle, globals::registry_queue_init, protocol::wl_output};

#[derive(Debug, Error)]
pub enum KdeError {
    #[error("failed to connect to wayland compositor: {0}")]
    Connect(#[from] wayland_client::ConnectError),
    #[error("failed to initialize wayland registry: {0}")]
    Global(#[from] wayland_client::globals::GlobalError),
    #[error("failed to dispatch wayland event queue: {0}")]
    Dispatch(#[from] wayland_client::DispatchError),
}

/// Wayland-native monitor discovery via `wl_output`. Works on any Wayland
/// compositor; the name and monitor-id prefix identify which desktop the
/// daemon believes it is running on so stored assignments stay stable.
#[derive(Debug, Clone)]
pub struct WaylandOutputClient {
    name: &'static str,
    id_prefix: &'static str,
}

impl WaylandOutputClient {
    pub fn new(name: &'static str, id_prefix: &'static str) -> Self {
        Self { name, id_prefix }
    }

    /// KDE Plasma session.
    pub fn kde() -> Self {
        Self::new("kde", "kde")
    }

    /// Generic layer-shell compositor fallback (Niri, Sway, river, ...).
    pub fn generic() -> Self {
        Self::new("wayland", "wl")
    }
}

/// Backwards-compatible KDE constructor.
#[derive(Debug, Default, Clone)]
pub struct KdeClient;

impl KdeClient {
    pub fn new() -> WaylandOutputClient {
        WaylandOutputClient::kde()
    }
}

impl CompositorClient for WaylandOutputClient {
    fn compositor_name(&self) -> &'static str {
        self.name
    }

    fn discover_monitors(
        &self,
    ) -> Result<Vec<MonitorInfo>, Box<dyn std::error::Error + Send + Sync>> {
        let conn = Connection::connect_to_env()?;
        let (globals, mut event_queue) = registry_queue_init(&conn)?;
        let qh = event_queue.handle();

        let output_state = OutputState::new(&globals, &qh);
        let registry_state = RegistryState::new(&globals);

        let mut probe = OutputDiscoveryProbe {
            registry_state,
            output_state,
        };

        event_queue.roundtrip(&mut probe)?;

        let monitors = probe
            .output_state
            .outputs()
            .filter_map(|output| {
                probe
                    .output_state
                    .info(&output)
                    .map(|info| monitor_info_from_output(self.id_prefix, &info))
            })
            .collect();

        Ok(monitors)
    }

    fn fullscreen_active(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // Fullscreen detection for non-Hyprland compositors (zwlr foreign
        // toplevel, KDE D-Bus, or niri IPC) is not yet implemented.
        Ok(false)
    }
}

fn monitor_info_from_output(
    id_prefix: &str,
    info: &smithay_client_toolkit::output::OutputInfo,
) -> MonitorInfo {
    let current_mode = info
        .modes
        .iter()
        .find(|m| m.current)
        .or_else(|| info.modes.iter().find(|m| m.preferred))
        .or_else(|| info.modes.first());

    let (width, height) = current_mode
        .map(|m| (m.dimensions.0.max(0) as u32, m.dimensions.1.max(0) as u32))
        .unwrap_or((0, 0));
    let refresh_rate = current_mode
        .map(|m| m.refresh_rate as f64 / 1000.0)
        .unwrap_or(0.0);

    let (x, y) = info.logical_position.unwrap_or(info.location);
    let scale = info.scale_factor as f64;
    let output_name = info.name.clone().unwrap_or_else(|| "unknown".into());
    let description = info.description.clone().unwrap_or_default();
    let make = info.make.clone();
    let model = info.model.clone();
    let disabled = current_mode.is_none();

    let id = build_monitor_id(id_prefix, &make, &model, &description, &output_name);

    MonitorInfo {
        id,
        output_name,
        description,
        make,
        model,
        serial: None,
        width,
        height,
        x,
        y,
        scale,
        refresh_rate,
        focused: false,
        disabled,
    }
}

fn build_monitor_id(
    id_prefix: &str,
    make: &str,
    model: &str,
    description: &str,
    output_name: &str,
) -> String {
    let fingerprint = normalized_component(description)
        .or_else(|| normalized_component(output_name))
        .unwrap_or_else(|| "unknown".to_string());

    format!(
        "{}:{}:{}:{}",
        id_prefix,
        slugify(make).unwrap_or_else(|| "unknown".to_string()),
        slugify(model).unwrap_or_else(|| "unknown".to_string()),
        fingerprint
    )
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
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if slug.is_empty() { None } else { Some(slug) }
}

struct OutputDiscoveryProbe {
    registry_state: RegistryState,
    output_state: OutputState,
}

impl OutputHandler for OutputDiscoveryProbe {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl ProvidesRegistryState for OutputDiscoveryProbe {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

delegate_output!(OutputDiscoveryProbe);
delegate_registry!(OutputDiscoveryProbe);

#[cfg(test)]
mod tests {
    use super::{build_monitor_id, slugify};

    #[test]
    fn monitor_id_uses_kde_prefix() {
        let id = build_monitor_id("kde", "Samsung", "ATNA33AA04", "Samsung ATNA33AA04", "eDP-1");
        assert!(id.starts_with("kde:"));
        assert!(id.contains("samsung"));
        assert!(id.contains("atna33aa04"));
    }

    #[test]
    fn monitor_id_uses_generic_wayland_prefix() {
        let id = build_monitor_id("wl", "Dell Inc.", "U2720Q", "Dell U2720Q", "DP-3");
        assert!(id.starts_with("wl:"));
        assert!(id.contains("dell"));
        assert!(id.contains("u2720q"));
    }

    #[test]
    fn slugify_lowercases_and_replaces_separators() {
        assert_eq!(slugify("Dell Inc."), Some("dell-inc".into()));
        assert_eq!(slugify("DELL U2720Q"), Some("dell-u2720q".into()));
        assert_eq!(slugify(""), None);
    }

    #[test]
    fn build_monitor_id_falls_back_to_output_name() {
        let id = build_monitor_id("kde", "", "", "", "eDP-1");
        assert_eq!(id, "kde:unknown:unknown:edp-1");
    }
}
