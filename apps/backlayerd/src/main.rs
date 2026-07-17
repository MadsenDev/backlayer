mod ipc;
mod runtime;

use std::sync::Arc;

use anyhow::Result;
use backlayer_config::ConfigStore;
use backlayer_hyprland::HyprlandClient;
use backlayer_kde::{KdeClient, WaylandOutputClient};
use backlayer_renderer_image::ImageRenderer;
use backlayer_renderer_shader::ShaderRenderer;
use backlayer_renderer_video::VideoRenderer;
use backlayer_types::{
    CompositorClient, CompositorKind, DaemonResponse, DaemonState, RuntimeDependencies,
};
use backlayer_wayland::LayerShellRuntime;
use runtime::RuntimeCoordinator;
use tracing::info;

fn detect_compositor() -> Result<Arc<dyn CompositorClient>, String> {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();

    match CompositorKind::detect_from_env() {
        CompositorKind::Hyprland => {
            info!("detected compositor: hyprland");
            Ok(Arc::new(HyprlandClient::new()))
        }
        CompositorKind::Kde => {
            info!("detected compositor: kde");
            Ok(Arc::new(KdeClient::new()))
        }
        CompositorKind::GenericWayland => {
            // Any other Wayland session (Niri, Sway, river, ...) gets
            // Wayland-native monitor discovery; rendering works wherever the
            // compositor implements wlr-layer-shell.
            info!(desktop = %desktop, "detected compositor: generic wayland layer-shell fallback");
            Ok(Arc::new(WaylandOutputClient::generic()))
        }
        CompositorKind::Unsupported => Err(format!(
            "unsupported session: Backlayer requires a Wayland session, but WAYLAND_DISPLAY is \
             not set (XDG_CURRENT_DESKTOP={desktop:?}). X11 and non-graphical sessions are not \
             supported; start Backlayer from inside a Wayland compositor such as Hyprland."
        )),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunMode {
    Once,
    Serve,
}

const USAGE: &str = "\
Usage: backlayerd [MODE]

Modes:
  --once       run a one-shot probe and exit (default)
  --serve      run as a persistent daemon with the IPC server

Options:
  -h, --help       print this help and exit
  -V, --version    print the version and exit";

fn parse_run_mode() -> Result<RunMode, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => Ok(RunMode::Once),
        [arg] => match arg.as_str() {
            "--once" => Ok(RunMode::Once),
            "--serve" => Ok(RunMode::Serve),
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("backlayerd {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            other => Err(format!("unrecognized argument: {other}")),
        },
        _ => Err(format!("too many arguments: {}", args.join(" "))),
    }
}

fn main() -> Result<()> {
    let run_mode = parse_run_mode().unwrap_or_else(|error| {
        eprintln!("backlayerd: {error}\n\n{USAGE}");
        std::process::exit(2);
    });

    tracing_subscriber::fmt()
        .with_env_filter("backlayer=info")
        .compact()
        .init();

    let compositor = detect_compositor().unwrap_or_else(|error| {
        eprintln!("backlayerd: {error}");
        std::process::exit(1);
    });
    let config_store = ConfigStore::default();
    let config_path = config_store.default_config_path();
    let resolved_config_path = config_store.resolve_path(&config_path)?;
    let wayland = LayerShellRuntime::new();

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

    if run_mode == RunMode::Serve {
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
