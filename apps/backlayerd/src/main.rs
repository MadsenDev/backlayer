mod ipc;
mod runtime;

use anyhow::Result;
use backlayer_config::ConfigStore;
use backlayer_hyprland::HyprlandClient;
use backlayer_renderer_image::ImageRenderer;
use backlayer_renderer_shader::ShaderRenderer;
use backlayer_renderer_video::VideoRenderer;
use backlayer_types::{DaemonResponse, DaemonState, RuntimeDependencies};
use backlayer_wayland::LayerShellRuntime;
use runtime::RuntimeCoordinator;
use tracing::info;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("backlayer=info")
        .compact()
        .init();

    let config_store = ConfigStore::default();
    let config_path = config_store.default_config_path();
    let resolved_config_path = config_store.resolve_path(&config_path)?;
    let hyprland = HyprlandClient::new();
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
    let monitors = hyprland.discover_monitors()?;
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
    let runtime_plan = coordinator.start(&daemon_state);
    daemon_state.runtime = runtime_plan.clone();

    info!(path = %config_path.display(), "backlayer daemon bootstrap");
    info!(ipc = %config_store.default_socket_path().display(), "ui/daemon ipc path");
    info!(config = ?loaded_config, "loaded config");
    info!(monitors = ?monitors, "monitor discovery bootstrap");
    info!(runtime = ?wayland.bootstrap_status(), "wayland bootstrap plan");
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
    info!(renderers = ?[image.name(), shader.name(), video.name()], "registered renderers");
    info!(runtime_dependencies = ?runtime_dependencies, "runtime dependency status");
    info!(assets = ?assets, "discovered wallpaper assets");
    info!(runtime_plan = ?runtime_plan, "planned renderer sessions");
    info!(
        response = ?DaemonResponse::State {
            state: daemon_state.clone()
        },
        "sample daemon state payload"
    );

    if run_mode == "--serve" {
        let socket_path = config_store.resolve_path(config_store.default_socket_path())?;
        ipc::serve_forever(&socket_path, &resolved_config_path, daemon_state, assets)?;
    }

    Ok(())
}
