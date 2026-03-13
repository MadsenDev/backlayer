use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow};
use backlayer_hyprland::HyprlandClient;
use backlayer_renderer_shader::ShaderRenderer;
use backlayer_types::{AssetMetadata, AssetSourceKind, CompatibilityInfo, WallpaperKind};
use backlayer_wayland::LayerShellRuntime;
use tracing::info;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("shader_runner=info,backlayer=info")
        .compact()
        .init();

    let output_name = std::env::args()
        .nth(1)
        .context("missing output name argument")?;
    let fps = std::env::args()
        .nth(2)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(30)
        .max(1);
    let pause_on_fullscreen = std::env::args().nth(3).as_deref() == Some("1");
    let pause_on_battery = std::env::args().nth(4).as_deref() == Some("1");
    let asset_id = std::env::args()
        .nth(5)
        .unwrap_or_else(|| "shader-runner".to_string());
    let asset_entrypoint = std::env::args()
        .nth(6)
        .map(PathBuf::from)
        .context("missing asset entrypoint")?;
    let animated = std::env::args().nth(7).as_deref() == Some("1");

    let runtime = LayerShellRuntime::new();
    let mut session = runtime
        .start_session_on_output(Some(&output_name))
        .with_context(|| format!("failed to start layer-shell session for {output_name}"))?;
    let renderer = ShaderRenderer::default();
    let asset = AssetMetadata {
        id: asset_id.clone(),
        name: asset_id.clone(),
        kind: WallpaperKind::Shader,
        animated,
        image_fit: None,
        source_kind: AssetSourceKind::Native,
        preview_image: None,
        compatibility: CompatibilityInfo::default(),
        import_metadata: None,
        entrypoint: asset_entrypoint,
    };
    let mut shader_runtime = renderer
        .create_runtime(&asset, &session)
        .map_err(|error| anyhow!(error.to_string()))?;

    info!(
        output = %output_name,
        fps,
        animated = shader_runtime.animated(),
        pause_on_fullscreen,
        pause_on_battery,
        detail = %shader_runtime.detail(),
        "shader runner started"
    );

    let frame_interval = Duration::from_millis((1000 / fps).max(1));
    let hyprland = HyprlandClient::new();
    let power = PowerStateProbe::default();
    let mut next_frame_at = Instant::now();
    let mut rendered_first_frame = true;

    shader_runtime
        .render_frame()
        .map_err(|error| anyhow!(error.to_string()))?;
    if shader_runtime.animated() {
        next_frame_at = Instant::now() + frame_interval;
    }

    loop {
        let paused_for_fullscreen =
            pause_on_fullscreen && hyprland.fullscreen_active().unwrap_or(false);
        let paused_for_battery = pause_on_battery && power.on_battery().unwrap_or(false);
        let paused = paused_for_fullscreen || paused_for_battery;

        if !paused {
            if shader_runtime.animated() {
                if Instant::now() >= next_frame_at {
                    shader_runtime
                        .render_frame()
                        .map_err(|error| anyhow!(error.to_string()))?;
                    next_frame_at = Instant::now() + frame_interval;
                }
            } else if !rendered_first_frame {
                shader_runtime
                    .render_frame()
                    .map_err(|error| anyhow!(error.to_string()))?;
                rendered_first_frame = true;
            }
        }

        session
            .dispatch_pending()
            .map_err(|error| anyhow!("wayland dispatch failed: {error}"))?;
        thread::sleep(Duration::from_millis(16));
    }
}

#[derive(Debug, Clone)]
struct PowerStateProbe {
    power_supply_root: PathBuf,
}

impl Default for PowerStateProbe {
    fn default() -> Self {
        Self {
            power_supply_root: PathBuf::from("/sys/class/power_supply"),
        }
    }
}

impl PowerStateProbe {
    fn on_battery(&self) -> std::io::Result<bool> {
        self.on_battery_at(&self.power_supply_root)
    }

    fn on_battery_at(&self, root: &Path) -> std::io::Result<bool> {
        let mut saw_battery = false;
        let mut saw_online_external_power = false;

        for entry in fs::read_dir(root)? {
            let path = entry?.path();
            let kind = fs::read_to_string(path.join("type")).unwrap_or_default();
            match kind.trim() {
                "Battery" => {
                    saw_battery = true;
                    let status = fs::read_to_string(path.join("status")).unwrap_or_default();
                    if status.trim() == "Discharging" {
                        return Ok(true);
                    }
                }
                "Mains" | "USB" | "USB_C" => {
                    let online = fs::read_to_string(path.join("online")).unwrap_or_default();
                    if online.trim() == "1" {
                        saw_online_external_power = true;
                    }
                }
                _ => {}
            }
        }

        Ok(saw_battery && !saw_online_external_power)
    }
}
