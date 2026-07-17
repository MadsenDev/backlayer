use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
    time::Duration,
};

use anyhow::Result;
use backlayer_config::ConfigStore;
use backlayer_renderer_video::VideoRenderer;
use backlayer_types::{CompositorKind, DaemonRequest, DaemonResponse, DaemonState};
use serde::Serialize;

const USAGE: &str = "\
Usage: backlayerctl <COMMAND>

Commands:
  doctor [--json]    check the local Backlayer installation and daemon health

Options:
  -h, --help       print this help and exit
  -V, --version    print the version and exit";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum CheckStatus {
    Ok,
    Warn,
    Fail,
}

#[derive(Debug, Serialize)]
struct CheckResult {
    name: &'static str,
    status: CheckStatus,
    detail: String,
}

impl CheckResult {
    fn ok(name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            name,
            status: CheckStatus::Ok,
            detail: detail.into(),
        }
    }

    fn warn(name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            name,
            status: CheckStatus::Warn,
            detail: detail.into(),
        }
    }

    fn fail(name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            name,
            status: CheckStatus::Fail,
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Serialize)]
struct DoctorReport {
    healthy: bool,
    checks: Vec<CheckResult>,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("doctor") => {
            let json = match args[1..] {
                [] => false,
                [ref flag] if flag == "--json" => true,
                _ => {
                    eprintln!(
                        "backlayerctl: unrecognized doctor arguments: {}\n\n{USAGE}",
                        args[1..].join(" ")
                    );
                    std::process::exit(2);
                }
            };
            run_doctor(json)
        }
        Some("-h") | Some("--help") => {
            println!("{USAGE}");
            Ok(())
        }
        Some("-V") | Some("--version") => {
            println!("backlayerctl {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some(other) => {
            eprintln!("backlayerctl: unrecognized command: {other}\n\n{USAGE}");
            std::process::exit(2);
        }
        None => {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    }
}

fn run_doctor(json: bool) -> Result<()> {
    let mut checks = Vec::new();

    checks.push(check_session());
    let config_store = ConfigStore::default();
    checks.push(check_config(&config_store));

    let daemon_state = query_daemon_state(&config_store, &mut checks);
    checks.push(check_video_dependencies(daemon_state.as_ref()));
    checks.push(check_assets(&config_store));

    let healthy = !checks.iter().any(|check| check.status == CheckStatus::Fail);
    let report = DoctorReport { healthy, checks };

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        for check in &report.checks {
            let tag = match check.status {
                CheckStatus::Ok => " ok ",
                CheckStatus::Warn => "warn",
                CheckStatus::Fail => "FAIL",
            };
            println!("[{tag}] {}: {}", check.name, check.detail);
        }
        println!();
        if report.healthy {
            println!("No blocking problems found.");
        } else {
            println!("Problems found. Fix the FAIL items above and re-run `backlayerctl doctor`.");
        }
    }

    if !report.healthy {
        std::process::exit(1);
    }
    Ok(())
}

fn check_session() -> CheckResult {
    let wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let kind = CompositorKind::detect_from_env();

    let session_detail = format!(
        "WAYLAND_DISPLAY={}, XDG_CURRENT_DESKTOP={}",
        wayland_display.as_deref().unwrap_or("<unset>"),
        if desktop.is_empty() {
            "<unset>"
        } else {
            &desktop
        },
    );

    match kind {
        CompositorKind::Unsupported => CheckResult::fail(
            "session",
            format!(
                "no supported compositor detected ({session_detail}); Backlayer requires a \
                 Wayland session"
            ),
        ),
        kind => CheckResult::ok(
            "session",
            format!(
                "daemon would use the {} integration ({session_detail})",
                kind.label()
            ),
        ),
    }
}

fn check_config(config_store: &ConfigStore) -> CheckResult {
    let config_path = config_store.default_config_path();
    let resolved = match config_store.resolve_path(&config_path) {
        Ok(path) => path,
        Err(error) => {
            return CheckResult::fail(
                "config",
                format!("cannot resolve {}: {error}", config_path.display()),
            );
        }
    };

    if !resolved.exists() {
        return CheckResult::warn(
            "config",
            format!(
                "{} does not exist yet; the daemon will start with defaults",
                resolved.display()
            ),
        );
    }

    match config_store.load_from_path(&config_path) {
        Ok(config) => CheckResult::ok(
            "config",
            format!(
                "{} loads cleanly ({} assignment(s))",
                resolved.display(),
                config.assignments.len()
            ),
        ),
        Err(error) => CheckResult::fail(
            "config",
            format!("{} failed to load: {error}", resolved.display()),
        ),
    }
}

/// Contacts the daemon over the IPC socket. Pushes socket/daemon checks and
/// returns the daemon state when it responded.
fn query_daemon_state(
    config_store: &ConfigStore,
    checks: &mut Vec<CheckResult>,
) -> Option<DaemonState> {
    let socket_path = match config_store.resolve_path(config_store.default_socket_path()) {
        Ok(path) => path,
        Err(error) => {
            checks.push(CheckResult::fail(
                "daemon",
                format!("cannot resolve socket path: {error}"),
            ));
            return None;
        }
    };

    if !socket_path.exists() {
        checks.push(CheckResult::warn(
            "daemon",
            format!(
                "not running (no socket at {}); start it with `backlayerd --serve`",
                socket_path.display()
            ),
        ));
        return None;
    }

    let state = match request_state(&socket_path) {
        Ok(state) => state,
        Err(error) => {
            checks.push(CheckResult::fail(
                "daemon",
                format!(
                    "socket {} exists but the daemon did not respond: {error}",
                    socket_path.display()
                ),
            ));
            return None;
        }
    };

    checks.push(CheckResult::ok(
        "daemon",
        format!("responding on {}", socket_path.display()),
    ));

    if state.monitors.is_empty() {
        checks.push(CheckResult::warn("monitors", "daemon reports no monitors"));
    } else {
        let summary = state
            .monitors
            .iter()
            .map(|monitor| {
                format!(
                    "{} {}x{}@{:.0}",
                    monitor.output_name, monitor.width, monitor.height, monitor.refresh_rate
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        checks.push(CheckResult::ok(
            "monitors",
            format!("{} monitor(s): {summary}", state.monitors.len()),
        ));
    }

    let session_count = state.runtime.sessions.len();
    if state.runtime.unresolved_assignments.is_empty() {
        checks.push(CheckResult::ok(
            "runtime",
            format!(
                "{} renderer session(s), {} assignment(s)",
                session_count,
                state.assignments.len()
            ),
        ));
    } else {
        checks.push(CheckResult::warn(
            "runtime",
            format!(
                "{} renderer session(s); unresolved assignments: {}",
                session_count,
                state.runtime.unresolved_assignments.join(", ")
            ),
        ));
    }

    let problem_events: Vec<String> = state
        .recent_events
        .iter()
        .filter(|event| event.level != "info")
        .map(|event| format!("[{}] {}: {}", event.level, event.runtime_key, event.message))
        .collect();
    if !problem_events.is_empty() {
        checks.push(CheckResult::warn(
            "recent events",
            problem_events.join("; "),
        ));
    }

    Some(state)
}

fn request_state(socket_path: &std::path::Path) -> Result<DaemonState, String> {
    let mut stream = UnixStream::connect(socket_path).map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| error.to_string())?;

    let payload =
        serde_json::to_vec(&DaemonRequest::GetState).map_err(|error| error.to_string())?;
    stream
        .write_all(&payload)
        .map_err(|error| error.to_string())?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|error| error.to_string())?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| error.to_string())?;

    match serde_json::from_str::<DaemonResponse>(&response).map_err(|error| error.to_string())? {
        DaemonResponse::State { state } => Ok(state),
        DaemonResponse::Error { message } => Err(format!("daemon returned an error: {message}")),
        other => Err(format!("unexpected daemon response: {other:?}")),
    }
}

fn check_video_dependencies(daemon_state: Option<&DaemonState>) -> CheckResult {
    // Prefer the daemon's own view when it is running; fall back to probing
    // this machine directly so doctor still works without a daemon.
    let status = match daemon_state {
        Some(state) => state.runtime_dependencies.video.clone(),
        None => VideoRenderer::default().dependency_status(),
    };

    let detail = status
        .detail
        .clone()
        .unwrap_or_else(|| "no detail reported".to_string());
    if status.available {
        CheckResult::ok("video dependencies", detail)
    } else {
        CheckResult::warn(
            "video dependencies",
            format!("video wallpapers will not play: {detail}"),
        )
    }
}

fn check_assets(config_store: &ConfigStore) -> CheckResult {
    match config_store.discover_all_assets() {
        Ok(assets) => CheckResult::ok(
            "assets",
            format!(
                "{} wallpaper asset(s) discovered (workshop imports {})",
                assets.len(),
                if config_store.workshop_enabled() {
                    "enabled"
                } else {
                    "disabled"
                }
            ),
        ),
        Err(error) => CheckResult::fail("assets", format!("asset discovery failed: {error}")),
    }
}
