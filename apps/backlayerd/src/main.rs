mod ipc;
mod runtime;

use std::sync::Arc;

use anyhow::Result;
use backlayer_config::ConfigStore;
use backlayer_hyprland::HyprlandClient;
use backlayer_kde::KdeClient;
use backlayer_renderer_image::ImageRenderer;
use backlayer_renderer_shader::ShaderRenderer;
use backlayer_renderer_video::VideoRenderer;
use backlayer_types::{CompositorClient, DaemonResponse, DaemonState, RuntimeDependencies};
use backlayer_wayland::LayerShellRuntime;
use runtime::RuntimeCoordinator;
use tracing::info;

fn detect_compositor() -> Arc<dyn CompositorClient> {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase();
    let is_hyprland =
        desktop.contains("hyprland") || std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok();
    let is_kde = desktop.contains("kde") || desktop.contains("plasma");

    if is_hyprland {
        info!("detected compositor: hyprland");
        Arc::new(HyprlandClient::new())
    } else if is_kde {
        info!("detected compositor: kde");
        Arc::new(KdeClient::new())
    } else {
        info!(desktop = %desktop, "compositor unknown, defaulting to hyprland client");
        Arc::new(HyprlandClient::new())
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("backlayer=info")
        .compact()
        .init();

    let compositor = detect_compositor();
    let config_store = ConfigStore::default();
    let config_path = config_store.default_config_path();
    let resolved_config_path = config_store.resolve_path(&config_path)?;
    let wayland = LayerShellRuntime::new();
    let run_mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "--once".to_string());

    let image = ImageRenderer::default();
    let shader = ShaderRenderer::default();
    let video = VideoRenderer::default();
    let runtime_dependencies = RuntimeDependencies {
        video: video.dependency_status(),
    };
    let loaded_config = config_store.load_or_default();
    let assets = config_store.discover_all_assets()?;
    let monitors = compositor
        .discover_monitors()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut daemon_state = DaemonState {
        monitors: monitors.clone(),
        assignments: loaded_config.assignments.clone(),
        pause: loaded_config.pause.clone(),
        runtime_dependencies: runtime_dependencies.clone(),
        runtime: Default::default(),
        recent_events: Vec::new(),
    };
    let coordinator = RuntimeCoordinator::new(
        wayland.clone(),
        image.clone(),
        shader.clone(),
        video.clone(),
    );

    info!(path = %config_path.display(), "backlayer daemon bootstrap");
    info!(ipc = %config_store.default_socket_path().display(), "ui/daemon ipc path");
    info!(config = ?loaded_config, "loaded config");
    info!(monitors = ?monitors, "monitor discovery bootstrap");
    info!(runtime = ?wayland.bootstrap_status(), "wayland bootstrap plan");
    info!(renderers = ?[image.name(), shader.name(), video.name()], "registered renderers");
    info!(runtime_dependencies = ?runtime_dependencies, "runtime dependency status");
    info!(assets = ?assets, "discovered wallpaper assets");

    if run_mode == "--serve" {
        // In serve mode, build the plan without creating any Wayland sessions.
        // The runtime manager's apply() call inside serve_forever will immediately
        // spawn the actual runner subprocesses. Creating probe sessions here would
        // leave ghost surfaces in the compositor that overlap with the runners.
        let runtime_plan = coordinator.build_plan(&daemon_state);
        daemon_state.runtime = runtime_plan.clone();
        info!(runtime_plan = ?runtime_plan, "planned renderer sessions");
        let socket_path = config_store.resolve_path(config_store.default_socket_path())?;
        ipc::serve_forever(
            &socket_path,
            &resolved_config_path,
            daemon_state,
            assets,
            compositor,
        )?;
    } else {
        // Probe-only mode: run full diagnostics and render preview wallpapers.
        match wayland.probe() {
            Ok(status) => info!(runtime = ?status, "wayland runtime probe"),
            Err(error) => info!(%error, "wayland runtime probe failed"),
        }
        if let Some(primary_monitor) = monitors.first() {
            match wayland.probe_on_output(Some(&primary_monitor.output_name)) {
                Ok(status) => info!(
                    output = %primary_monitor.output_name,
                    runtime = ?status,
                    "wayland output-bound probe"
                ),
                Err(error) => info!(
                    output = %primary_monitor.output_name,
                    %error,
                    "wayland output-bound probe failed"
                ),
            }
            match wayland.start_session_on_output(Some(&primary_monitor.output_name)) {
                Ok(session) => info!(
                    output = %primary_monitor.output_name,
                    runtime = ?session.status(),
                    "wayland persistent session bootstrap"
                ),
                Err(error) => info!(
                    output = %primary_monitor.output_name,
                    %error,
                    "wayland persistent session bootstrap failed"
                ),
            }
        }
        let runtime_plan = coordinator.start(&daemon_state);
        daemon_state.runtime = runtime_plan.clone();
        info!(runtime_plan = ?runtime_plan, "planned renderer sessions");
        info!(
            response = ?DaemonResponse::State {
                state: daemon_state.clone()
            },
            "sample daemon state payload"
        );
    }

    Ok(())
}
