use std::{
    collections::HashMap,
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use backlayer_hyprland::HyprlandClient;
use backlayer_renderer_image::ImageRenderer;
use backlayer_renderer_shader::ShaderRenderer;
use backlayer_renderer_video::VideoRenderer;
use backlayer_types::{
    DaemonState, PausePolicy, RendererBackend, RendererLaunchSpec, RendererSession,
    RendererSessionStatus, RuntimeEvent, RuntimePlan, WallpaperKind,
};
use backlayer_wayland::{BootstrapStatus, LayerShellRuntime};
use tracing::info;

const MAX_RUNTIME_EVENTS: usize = 32;
const MAX_RESTART_ATTEMPTS: usize = 3;
const RESTART_BACKOFF: Duration = Duration::from_secs(1);

pub struct RuntimeCoordinator {
    wayland: LayerShellRuntime,
    image: ImageRenderer,
    shader: ShaderRenderer,
    video: VideoRenderer,
}

pub struct RuntimeManager {
    coordinator: RuntimeCoordinator,
    workers: Vec<RuntimeWorker>,
    status_map: Arc<Mutex<HashMap<String, RendererSessionStatus>>>,
    event_log: Arc<Mutex<VecDeque<RuntimeEvent>>>,
    crash_flags: Arc<Mutex<HashMap<String, bool>>>,
    specs: Vec<RendererLaunchSpec>,
    unresolved_assignments: Vec<String>,
}

struct RuntimeWorker {
    stop: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<()>>,
}

impl RuntimeCoordinator {
    pub fn new(
        wayland: LayerShellRuntime,
        image: ImageRenderer,
        shader: ShaderRenderer,
        video: VideoRenderer,
    ) -> Self {
        Self {
            wayland,
            image,
            shader,
            video,
        }
    }

    pub fn build_plan(&self, state: &DaemonState) -> RuntimePlan {
        let mut sessions = Vec::new();
        let mut unresolved_assignments = Vec::new();

        for assignment in &state.assignments {
            match state
                .monitors
                .iter()
                .find(|monitor| monitor.matches_assignment(assignment))
            {
                Some(monitor) => {
                    let mut asset = assignment.wallpaper.clone();
                    if let Some(image_fit) = assignment.settings.image_fit.clone() {
                        asset.image_fit = Some(image_fit);
                    }

                    sessions.push(RendererSession {
                        spec: RendererLaunchSpec {
                            monitor_id: monitor.id.clone(),
                            output_name: monitor.output_name.clone(),
                            asset: asset.clone(),
                            backend: backend_for(&asset.kind),
                        },
                        status: RendererSessionStatus::Ready {
                            output_name: monitor.output_name.clone(),
                            configured: false,
                            persistent: false,
                            paused_reason: None,
                            detail: None,
                        },
                    })
                }
                None => unresolved_assignments.push(assignment.monitor_id.clone()),
            }
        }

        RuntimePlan {
            sessions,
            unresolved_assignments,
        }
    }

    pub fn start(&self, state: &DaemonState) -> RuntimePlan {
        let mut plan = self.build_plan(state);

        for session in &mut plan.sessions {
            session.status = self.start_session(&session.spec);
        }

        plan
    }

    fn start_session(&self, spec: &RendererLaunchSpec) -> RendererSessionStatus {
        match spec.backend {
            RendererBackend::Image => self.image.validate_asset(&spec.asset).map_or_else(
                |error| RendererSessionStatus::Failed {
                    reason: error.to_string(),
                },
                |_| match self
                    .wayland
                    .start_session_on_output(Some(&spec.output_name))
                {
                    Ok(mut session) => {
                        match self.image.render_asset_once(&spec.asset, &mut session) {
                            Ok(detail) => match session.dispatch_pending() {
                                Ok(BootstrapStatus::Ready {
                                    bound_output,
                                    configured,
                                    ..
                                }) => RendererSessionStatus::Ready {
                                    output_name: bound_output
                                        .unwrap_or_else(|| spec.output_name.clone()),
                                    configured,
                                    persistent: false,
                                    paused_reason: None,
                                    detail: Some(detail),
                                },
                                Ok(status) => RendererSessionStatus::Failed {
                                    reason: format!(
                                        "unexpected wayland session status: {status:?}"
                                    ),
                                },
                                Err(error) => RendererSessionStatus::Failed {
                                    reason: error.to_string(),
                                },
                            },
                            Err(error) => RendererSessionStatus::Failed {
                                reason: error.to_string(),
                            },
                        }
                    }
                    Err(error) => RendererSessionStatus::Failed {
                        reason: error.to_string(),
                    },
                },
            ),
            RendererBackend::Shader => self.shader.validate_asset(&spec.asset).map_or_else(
                |error| RendererSessionStatus::Failed {
                    reason: error.to_string(),
                },
                |_| match self
                    .wayland
                    .start_session_on_output(Some(&spec.output_name))
                {
                    Ok(mut session) => match self
                        .shader
                        .render_asset_once(&spec.asset, &mut session)
                    {
                        Ok(detail) => match session.dispatch_pending() {
                            Ok(BootstrapStatus::Ready {
                                bound_output,
                                configured,
                                ..
                            }) => RendererSessionStatus::Ready {
                                output_name: bound_output
                                    .unwrap_or_else(|| spec.output_name.clone()),
                                configured,
                                persistent: false,
                                paused_reason: None,
                                detail: Some(detail),
                            },
                            Ok(status) => RendererSessionStatus::Failed {
                                reason: format!("unexpected wayland session status: {status:?}"),
                            },
                            Err(error) => RendererSessionStatus::Failed {
                                reason: error.to_string(),
                            },
                        },
                        Err(error) => RendererSessionStatus::Failed {
                            reason: error.to_string(),
                        },
                    },
                    Err(error) => RendererSessionStatus::Failed {
                        reason: error.to_string(),
                    },
                },
            ),
            RendererBackend::Video => self.video.validate_asset(&spec.asset).map_or_else(
                |error| RendererSessionStatus::Failed {
                    reason: error.to_string(),
                },
                |_| match try_preview_fallback_session(&self.wayland, &self.image, spec) {
                    Ok((mut session, detail)) => ready_status(&mut session, spec, detail),
                    Err(status) => status,
                },
            ),
            RendererBackend::Scene | RendererBackend::Web => {
                match try_preview_fallback_session(&self.wayland, &self.image, spec) {
                    Ok((mut session, detail)) => ready_status(&mut session, spec, detail),
                    Err(status) => status,
                }
            }
        }
    }
}

impl RuntimeManager {
    pub fn new(
        wayland: LayerShellRuntime,
        image: ImageRenderer,
        shader: ShaderRenderer,
        video: VideoRenderer,
    ) -> Self {
        Self {
            coordinator: RuntimeCoordinator::new(wayland, image, shader, video),
            workers: Vec::new(),
            status_map: Arc::new(Mutex::new(HashMap::new())),
            event_log: Arc::new(Mutex::new(VecDeque::new())),
            crash_flags: Arc::new(Mutex::new(HashMap::new())),
            specs: Vec::new(),
            unresolved_assignments: Vec::new(),
        }
    }

    pub fn apply(&mut self, state: &DaemonState) -> RuntimePlan {
        self.stop_all();

        let plan = self.coordinator.build_plan(state);
        self.specs = plan
            .sessions
            .iter()
            .map(|session| session.spec.clone())
            .collect();
        self.unresolved_assignments = plan.unresolved_assignments.clone();
        self.status_map.lock().expect("status map lock").clear();
        self.event_log.lock().expect("event log lock").clear();
        self.crash_flags.lock().expect("crash flags lock").clear();
        let mut sessions = Vec::with_capacity(plan.sessions.len());

        for session in plan.sessions {
            let (status, worker) = self.spawn_worker(session.spec.clone(), state.pause.clone());
            self.status_map
                .lock()
                .expect("status map lock")
                .insert(runtime_key(&session.spec), status.clone());
            if let Some(worker) = worker {
                self.workers.push(worker);
            }
            sessions.push(RendererSession {
                spec: session.spec,
                status,
            });
        }

        RuntimePlan {
            sessions,
            unresolved_assignments: self.unresolved_assignments.clone(),
        }
    }

    pub fn snapshot(&self) -> RuntimePlan {
        let statuses = self.status_map.lock().expect("status map lock");
        RuntimePlan {
            sessions: self
                .specs
                .iter()
                .cloned()
                .map(|spec| RendererSession {
                    status: statuses.get(&runtime_key(&spec)).cloned().unwrap_or(
                        RendererSessionStatus::Failed {
                            reason: "runtime status unavailable".into(),
                        },
                    ),
                    spec,
                })
                .collect(),
            unresolved_assignments: self.unresolved_assignments.clone(),
        }
    }

    pub fn recent_events(&self) -> Vec<RuntimeEvent> {
        self.event_log
            .lock()
            .expect("event log lock")
            .iter()
            .cloned()
            .collect()
    }

    pub fn simulate_crash(&self, monitor_id: &str, asset_id: &str) -> Result<(), String> {
        let runtime_key = format!("{monitor_id}:{asset_id}");
        if !self
            .specs
            .iter()
            .any(|spec| runtime_key == runtime_key_for(spec))
        {
            return Err(format!("unknown runtime session: {runtime_key}"));
        }

        self.crash_flags
            .lock()
            .expect("crash flags lock")
            .insert(runtime_key.clone(), true);
        push_runtime_event(
            &self.event_log,
            &runtime_key,
            "info",
            "simulated crash requested".into(),
        );
        Ok(())
    }

    fn stop_all(&mut self) {
        for worker in &self.workers {
            worker.stop.store(true, Ordering::Relaxed);
        }

        for worker in &mut self.workers {
            if let Some(join) = worker.join.take() {
                let _ = join.join();
            }
        }

        self.workers.clear();
    }

    fn spawn_worker(
        &self,
        spec: RendererLaunchSpec,
        pause: PausePolicy,
    ) -> (RendererSessionStatus, Option<RuntimeWorker>) {
        match spec.backend {
            RendererBackend::Image | RendererBackend::Shader => {
                let stop = Arc::new(AtomicBool::new(false));
                let (tx, rx) = mpsc::channel();
                let stop_thread = stop.clone();
                let wayland = self.coordinator.wayland.clone();
                let image = self.coordinator.image.clone();
                let shader = self.coordinator.shader.clone();
                let status_map = self.status_map.clone();
                let event_log = self.event_log.clone();
                let crash_flags = self.crash_flags.clone();
                let runtime_key = runtime_key_for(&spec);
                let join = thread::spawn(move || {
                    match bootstrap_worker_session(
                        &spec,
                        &pause,
                        &wayland,
                        &image,
                        &shader,
                        &status_map,
                        &event_log,
                        &runtime_key,
                    ) {
                        Ok((status, live_session)) => {
                            let _ = tx.send(status.clone());
                            supervise_session_restarts(
                                &spec,
                                &pause,
                                &wayland,
                                &image,
                                &shader,
                                &stop_thread,
                                &status_map,
                                &event_log,
                                &crash_flags,
                                &runtime_key,
                                live_session,
                            );
                        }
                        Err(status) => {
                            push_runtime_event(
                                &event_log,
                                &runtime_key,
                                "error",
                                failure_message(&status),
                            );
                            let _ = tx.send(status);
                        }
                    }
                });

                match rx.recv_timeout(Duration::from_secs(5)) {
                    Ok(status) => (
                        status,
                        Some(RuntimeWorker {
                            stop,
                            join: Some(join),
                        }),
                    ),
                    Err(error) => (
                        RendererSessionStatus::Failed {
                            reason: format!("renderer worker failed to report readiness: {error}"),
                        },
                        Some(RuntimeWorker {
                            stop,
                            join: Some(join),
                        }),
                    ),
                }
            }
            RendererBackend::Video | RendererBackend::Scene | RendererBackend::Web => {
                let stop = Arc::new(AtomicBool::new(false));
                let (tx, rx) = mpsc::channel();
                let stop_thread = stop.clone();
                let wayland = self.coordinator.wayland.clone();
                let image = self.coordinator.image.clone();
                let status_map = self.status_map.clone();
                let event_log = self.event_log.clone();
                let crash_flags = self.crash_flags.clone();
                let runtime_key = runtime_key_for(&spec);
                let join = thread::spawn(move || {
                    match bootstrap_worker_session(
                        &spec,
                        &pause,
                        &wayland,
                        &image,
                        &ShaderRenderer::default(),
                        &status_map,
                        &event_log,
                        &runtime_key,
                    ) {
                        Ok((status, live_session)) => {
                            let _ = tx.send(status.clone());
                            supervise_session_restarts(
                                &spec,
                                &pause,
                                &wayland,
                                &image,
                                &ShaderRenderer::default(),
                                &stop_thread,
                                &status_map,
                                &event_log,
                                &crash_flags,
                                &runtime_key,
                                live_session,
                            );
                        }
                        Err(status) => {
                            push_runtime_event(
                                &event_log,
                                &runtime_key,
                                "error",
                                failure_message(&status),
                            );
                            let _ = tx.send(status);
                        }
                    }
                });

                match rx.recv_timeout(Duration::from_secs(5)) {
                    Ok(status) => (
                        status,
                        Some(RuntimeWorker {
                            stop,
                            join: Some(join),
                        }),
                    ),
                    Err(error) => (
                        RendererSessionStatus::Failed {
                            reason: format!("renderer worker failed to report readiness: {error}"),
                        },
                        Some(RuntimeWorker {
                            stop,
                            join: Some(join),
                        }),
                    ),
                }
            }
        }
    }
}

fn bootstrap_worker_session(
    spec: &RendererLaunchSpec,
    pause: &PausePolicy,
    wayland: &LayerShellRuntime,
    image: &ImageRenderer,
    _shader: &ShaderRenderer,
    status_map: &Arc<Mutex<HashMap<String, RendererSessionStatus>>>,
    event_log: &Arc<Mutex<VecDeque<RuntimeEvent>>>,
    runtime_key: &str,
) -> Result<(RendererSessionStatus, LiveSession), RendererSessionStatus> {
    match spec.backend {
        RendererBackend::Image => match start_image_session(wayland, image, spec) {
            Ok((mut session, detail)) => {
                let status = ready_status(&mut session, spec, detail);
                set_runtime_status(status_map, runtime_key, status.clone());
                push_runtime_event(event_log, runtime_key, "info", ready_message(spec));
                if matches!(status, RendererSessionStatus::Ready { .. }) {
                    Ok((status, LiveSession::Image(session)))
                } else {
                    Err(status)
                }
            }
            Err(status) => Err(status),
        },
        RendererBackend::Shader => match start_shader_process_session(spec, pause) {
            Ok(child) => {
                let status = RendererSessionStatus::Ready {
                    output_name: spec.output_name.clone(),
                    configured: true,
                    persistent: true,
                    paused_reason: None,
                    detail: Some(shader_process_detail(spec)),
                };
                set_runtime_status(status_map, runtime_key, status.clone());
                push_runtime_event(event_log, runtime_key, "info", ready_message(spec));
                Ok((status, LiveSession::ShaderProcess(child)))
            }
            Err(status) => Err(status),
        },
        RendererBackend::Video => match start_video_process_session(spec, pause) {
            Ok(child) => {
                let status = RendererSessionStatus::Ready {
                    output_name: spec.output_name.clone(),
                    configured: true,
                    persistent: true,
                    paused_reason: None,
                    detail: Some(video_process_detail(spec)),
                };
                set_runtime_status(status_map, runtime_key, status.clone());
                push_runtime_event(event_log, runtime_key, "info", ready_message(spec));
                Ok((status, LiveSession::VideoProcess(child)))
            }
            Err(status) => Err(status),
        },
        RendererBackend::Web => match start_web_process_session(spec) {
            Ok(child) => {
                let status = RendererSessionStatus::Ready {
                    output_name: spec.output_name.clone(),
                    configured: true,
                    persistent: true,
                    paused_reason: None,
                    detail: Some(web_process_detail(spec)),
                };
                set_runtime_status(status_map, runtime_key, status.clone());
                push_runtime_event(event_log, runtime_key, "info", ready_message(spec));
                Ok((status, LiveSession::WebProcess(child)))
            }
            Err(status) => Err(status),
        },
        RendererBackend::Scene => match start_scene_process_session(spec, pause) {
            Ok(child) => {
                let status = RendererSessionStatus::Ready {
                    output_name: spec.output_name.clone(),
                    configured: true,
                    persistent: true,
                    paused_reason: None,
                    detail: Some(scene_process_detail(spec)),
                };
                set_runtime_status(status_map, runtime_key, status.clone());
                push_runtime_event(event_log, runtime_key, "info", ready_message(spec));
                Ok((status, LiveSession::SceneProcess(child)))
            }
            Err(status) => Err(status),
        },
    }
}

impl Drop for RuntimeManager {
    fn drop(&mut self) {
        self.stop_all();
    }
}

fn backend_for(kind: &WallpaperKind) -> RendererBackend {
    match kind {
        WallpaperKind::Image => RendererBackend::Image,
        WallpaperKind::Shader => RendererBackend::Shader,
        WallpaperKind::Video => RendererBackend::Video,
        WallpaperKind::Scene => RendererBackend::Scene,
        WallpaperKind::Web => RendererBackend::Web,
    }
}

fn start_image_session(
    wayland: &LayerShellRuntime,
    image: &ImageRenderer,
    spec: &RendererLaunchSpec,
) -> Result<(backlayer_wayland::LayerSurfaceSession, String), RendererSessionStatus> {
    match wayland.start_session_on_output(Some(&spec.output_name)) {
        Ok(mut session) => match image.render_asset_once(&spec.asset, &mut session) {
            Ok(detail) => Ok((session, detail)),
            Err(error) => Err(RendererSessionStatus::Failed {
                reason: error.to_string(),
            }),
        },
        Err(error) => Err(RendererSessionStatus::Failed {
            reason: error.to_string(),
        }),
    }
}

fn preview_fallback_spec(spec: &RendererLaunchSpec) -> Option<RendererLaunchSpec> {
    let preview_image = spec.asset.preview_image.clone()?;
    if !preview_image.is_file() {
        return None;
    }

    let mut asset = spec.asset.clone();
    asset.kind = WallpaperKind::Image;
    asset.animated = false;
    asset.entrypoint = preview_image;

    Some(RendererLaunchSpec {
        monitor_id: spec.monitor_id.clone(),
        output_name: spec.output_name.clone(),
        asset,
        backend: RendererBackend::Image,
    })
}

fn try_preview_fallback_session(
    wayland: &LayerShellRuntime,
    image: &ImageRenderer,
    spec: &RendererLaunchSpec,
) -> Result<(backlayer_wayland::LayerSurfaceSession, String), RendererSessionStatus> {
    let fallback_spec =
        preview_fallback_spec(spec).ok_or_else(|| RendererSessionStatus::Unsupported {
            reason: format!(
                "{} runtime is not implemented yet and no preview image fallback is available",
                backend_label(spec.backend.clone())
            ),
        })?;

    let (session, detail) = start_image_session(wayland, image, &fallback_spec)?;
    Ok((
        session,
        format!(
            "{} runtime is not implemented yet; preview fallback rendered from {} ({detail})",
            backend_label(spec.backend.clone()),
            fallback_spec.asset.entrypoint.display(),
        ),
    ))
}

fn start_shader_process_session(
    spec: &RendererLaunchSpec,
    pause: &PausePolicy,
) -> Result<Child, RendererSessionStatus> {
    let args = vec![
        spec.output_name.clone(),
        pause.fps_limit.max(1).to_string(),
        if pause.pause_on_fullscreen { "1" } else { "0" }.to_string(),
        if pause.pause_on_battery { "1" } else { "0" }.to_string(),
        spec.asset.id.clone(),
        spec.asset.entrypoint.display().to_string(),
        if spec.asset.animated { "1" } else { "0" }.to_string(),
    ];

    start_runner_process("shader-runner", &args)
}

fn shader_process_detail(spec: &RendererLaunchSpec) -> String {
    format!(
        "shader runner process started ({} asset mode) for {}",
        if spec.asset.animated {
            "animated"
        } else {
            "static"
        },
        spec.asset.id
    )
}

fn start_video_process_session(
    spec: &RendererLaunchSpec,
    pause: &PausePolicy,
) -> Result<Child, RendererSessionStatus> {
    let args = vec![
        spec.output_name.clone(),
        pause.fps_limit.max(1).to_string(),
        if pause.pause_on_fullscreen { "1" } else { "0" }.to_string(),
        if pause.pause_on_battery { "1" } else { "0" }.to_string(),
        spec.asset.id.clone(),
        spec.asset.entrypoint.display().to_string(),
    ];

    start_runner_process("video-runner", &args)
}

fn start_web_process_session(spec: &RendererLaunchSpec) -> Result<Child, RendererSessionStatus> {
    let args = vec![
        spec.output_name.clone(),
        spec.asset.id.clone(),
        spec.asset
            .preview_image
            .as_ref()
            .filter(|path| path.is_file())
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        spec.asset.entrypoint.display().to_string(),
    ];

    start_runner_process("web-runner", &args)
}

fn start_scene_process_session(
    spec: &RendererLaunchSpec,
    pause: &PausePolicy,
) -> Result<Child, RendererSessionStatus> {
    let args = vec![
        spec.output_name.clone(),
        pause.fps_limit.max(1).to_string(),
        if pause.pause_on_fullscreen { "1" } else { "0" }.to_string(),
        if pause.pause_on_battery { "1" } else { "0" }.to_string(),
        spec.asset.id.clone(),
        spec.asset
            .preview_image
            .as_ref()
            .filter(|path| path.is_file())
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        spec.asset.entrypoint.display().to_string(),
    ];

    start_runner_process("scene-runner", &args)
}

fn start_runner_process(
    runner_name: &str,
    args: &[String],
) -> Result<Child, RendererSessionStatus> {
    let current_exe = std::env::current_exe().map_err(|error| RendererSessionStatus::Failed {
        reason: format!("failed to resolve current executable: {error}"),
    })?;
    let runner_path = current_exe
        .parent()
        .map(|dir| dir.join(runner_name))
        .ok_or_else(|| RendererSessionStatus::Failed {
            reason: format!("failed to resolve {runner_name} path"),
        })?;

    if runner_path.exists() && !cfg!(debug_assertions) {
        let mut command = Command::new(&runner_path);
        for arg in args {
            command.arg(arg);
        }
        command
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| RendererSessionStatus::Failed {
                reason: format!(
                    "failed to launch {runner_name} at {}: {error}",
                    runner_path.display()
                ),
            })
    } else {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|dir| dir.parent())
            .map(Path::to_path_buf)
            .ok_or_else(|| RendererSessionStatus::Failed {
                reason: format!(
                    "failed to resolve workspace root for cargo {runner_name} fallback"
                ),
            })?;
        let mut command = Command::new("cargo");
        command.arg("run").arg("-p").arg(runner_name).arg("--");
        for arg in args {
            command.arg(arg);
        }
        command
            .current_dir(&workspace_root)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| RendererSessionStatus::Failed {
                reason: format!(
                    "failed to launch {runner_name} via cargo from {}: {error}",
                    workspace_root.display()
                ),
            })
    }
}

fn video_process_detail(spec: &RendererLaunchSpec) -> String {
    format!(
        "video runner process started with ffmpeg decode playback for {}",
        spec.asset.id
    )
}

fn web_process_detail(spec: &RendererLaunchSpec) -> String {
    match spec.asset.preview_image.as_ref() {
        Some(preview) => format!(
            "web runner process started for {} (preview available at {})",
            spec.asset.id,
            preview.display()
        ),
        None => format!("web runner process started for {}", spec.asset.id),
    }
}

fn scene_process_detail(spec: &RendererLaunchSpec) -> String {
    match spec.asset.preview_image.as_ref() {
        Some(preview) => format!(
            "scene runner process started for {} (preview available at {})",
            spec.asset.id,
            preview.display()
        ),
        None => format!("scene runner process started for {}", spec.asset.id),
    }
}

fn ready_status(
    session: &mut backlayer_wayland::LayerSurfaceSession,
    spec: &RendererLaunchSpec,
    detail: String,
) -> RendererSessionStatus {
    match session.dispatch_pending() {
        Ok(BootstrapStatus::Ready {
            bound_output,
            configured,
            ..
        }) => RendererSessionStatus::Ready {
            output_name: bound_output.unwrap_or_else(|| spec.output_name.clone()),
            configured,
            persistent: true,
            paused_reason: None,
            detail: Some(detail),
        },
        Ok(status) => RendererSessionStatus::Failed {
            reason: format!("unexpected wayland session status: {status:?}"),
        },
        Err(error) => RendererSessionStatus::Failed {
            reason: error.to_string(),
        },
    }
}

fn idle_session(
    session: &mut backlayer_wayland::LayerSurfaceSession,
    stop: &Arc<AtomicBool>,
    mut shader_runtime: Option<(&mut backlayer_renderer_shader::ShaderRuntime, Duration)>,
    pause: Option<&PausePolicy>,
    status_map: &Arc<Mutex<HashMap<String, RendererSessionStatus>>>,
    event_log: &Arc<Mutex<VecDeque<RuntimeEvent>>>,
    crash_flags: &Arc<Mutex<HashMap<String, bool>>>,
    runtime_key: &str,
) -> Result<(), String> {
    let mut next_frame_at = Instant::now();
    let hyprland = pause.map(|_| HyprlandClient::new());
    let power = pause.map(|_| PowerStateProbe::default());
    let mut paused_reason = None;

    while !stop.load(Ordering::Relaxed) {
        if take_crash_flag(crash_flags, runtime_key) {
            return Err("simulated renderer crash".into());
        }
        if let Some((runtime, interval)) = shader_runtime.as_mut() {
            let paused_for_fullscreen = pause
                .filter(|policy| policy.pause_on_fullscreen)
                .is_some_and(|_| {
                    hyprland
                        .as_ref()
                        .and_then(|client| client.fullscreen_active().ok())
                        .unwrap_or(false)
                });
            let paused_for_battery =
                pause
                    .filter(|policy| policy.pause_on_battery)
                    .is_some_and(|_| {
                        power
                            .as_ref()
                            .and_then(|probe| probe.on_battery().ok())
                            .unwrap_or(false)
                    });
            let next_paused_reason = if paused_for_fullscreen {
                Some("fullscreen active".to_string())
            } else if paused_for_battery {
                Some("battery power".to_string())
            } else {
                None
            };

            if paused_reason != next_paused_reason {
                paused_reason = next_paused_reason.clone();
                update_paused_reason(status_map, event_log, runtime_key, next_paused_reason);
            }

            if paused_reason.is_none() && Instant::now() >= next_frame_at {
                if let Err(error) = runtime.render_frame() {
                    return Err(error.to_string());
                }
                next_frame_at = Instant::now() + *interval;
            }
        }
        if let Err(error) = session.dispatch_pending() {
            return Err(error.to_string());
        }
        thread::sleep(Duration::from_millis(16));
    }

    Ok(())
}

fn idle_process(child: &mut Child, stop: &Arc<AtomicBool>, label: &str) -> Result<(), String> {
    while !stop.load(Ordering::Relaxed) {
        match child.try_wait() {
            Ok(Some(status)) => return Err(format!("{label} exited with status {status}")),
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(error) => return Err(format!("failed to query {label} status: {error}")),
        }
    }

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

fn runtime_key(spec: &RendererLaunchSpec) -> String {
    runtime_key_for(spec)
}

fn runtime_key_for(spec: &RendererLaunchSpec) -> String {
    format!("{}:{}", spec.monitor_id, spec.asset.id)
}

fn set_runtime_status(
    status_map: &Arc<Mutex<HashMap<String, RendererSessionStatus>>>,
    runtime_key: &str,
    status: RendererSessionStatus,
) {
    status_map
        .lock()
        .expect("status map lock")
        .insert(runtime_key.to_string(), status);
}

fn update_paused_reason(
    status_map: &Arc<Mutex<HashMap<String, RendererSessionStatus>>>,
    event_log: &Arc<Mutex<VecDeque<RuntimeEvent>>>,
    runtime_key: &str,
    paused_reason: Option<String>,
) {
    if let Some(RendererSessionStatus::Ready {
        paused_reason: current,
        ..
    }) = status_map
        .lock()
        .expect("status map lock")
        .get_mut(runtime_key)
    {
        if *current != paused_reason {
            match &paused_reason {
                Some(reason) => {
                    info!(runtime_key, %reason, "renderer session paused");
                    push_runtime_event(
                        event_log,
                        runtime_key,
                        "info",
                        format!("renderer paused: {reason}"),
                    );
                }
                None => {
                    info!(runtime_key, "renderer session resumed");
                    push_runtime_event(event_log, runtime_key, "info", "renderer resumed".into());
                }
            }
        }
        *current = paused_reason;
    }
}

fn ready_message(spec: &RendererLaunchSpec) -> String {
    format!(
        "{} renderer ready on {} for {}",
        match spec.backend {
            RendererBackend::Image => "image",
            RendererBackend::Shader => "shader",
            RendererBackend::Video => "video",
            RendererBackend::Scene => "scene",
            RendererBackend::Web => "web",
        },
        spec.output_name,
        spec.asset.id
    )
}

fn failure_message(status: &RendererSessionStatus) -> String {
    match status {
        RendererSessionStatus::Failed { reason } => format!("renderer failed: {reason}"),
        RendererSessionStatus::Unsupported { reason } => format!("renderer unsupported: {reason}"),
        RendererSessionStatus::Ready { .. } => "renderer ready".into(),
    }
}

fn push_runtime_event(
    event_log: &Arc<Mutex<VecDeque<RuntimeEvent>>>,
    runtime_key: &str,
    level: &str,
    message: String,
) {
    let mut events = event_log.lock().expect("event log lock");
    if events.len() >= MAX_RUNTIME_EVENTS {
        events.pop_front();
    }
    events.push_back(RuntimeEvent {
        runtime_key: runtime_key.to_string(),
        level: level.to_string(),
        message,
    });
}

enum LiveSession {
    Image(backlayer_wayland::LayerSurfaceSession),
    SceneProcess(Child),
    ShaderProcess(Child),
    VideoProcess(Child),
    WebProcess(Child),
}

fn supervise_session_restarts(
    spec: &RendererLaunchSpec,
    pause: &PausePolicy,
    wayland: &LayerShellRuntime,
    image: &ImageRenderer,
    shader: &ShaderRenderer,
    stop: &Arc<AtomicBool>,
    status_map: &Arc<Mutex<HashMap<String, RendererSessionStatus>>>,
    event_log: &Arc<Mutex<VecDeque<RuntimeEvent>>>,
    crash_flags: &Arc<Mutex<HashMap<String, bool>>>,
    runtime_key: &str,
    mut live_session: LiveSession,
) {
    let mut restart_attempts = 0usize;

    loop {
        let run_result = match &mut live_session {
            LiveSession::Image(session) => idle_session(
                session,
                stop,
                None,
                Some(pause),
                status_map,
                event_log,
                crash_flags,
                runtime_key,
            ),
            LiveSession::SceneProcess(child) => idle_process(child, stop, "scene runner"),
            LiveSession::ShaderProcess(child) => idle_process(child, stop, "shader runner"),
            LiveSession::VideoProcess(child) => idle_process(child, stop, "video runner"),
            LiveSession::WebProcess(child) => idle_process(child, stop, "web runner"),
        };

        if stop.load(Ordering::Relaxed) {
            return;
        }

        let error = match run_result {
            Ok(()) => return,
            Err(error) => error,
        };

        restart_attempts += 1;
        let failed_status = RendererSessionStatus::Failed {
            reason: error.clone(),
        };
        set_runtime_status(status_map, runtime_key, failed_status);
        push_runtime_event(
            event_log,
            runtime_key,
            "error",
            format!(
                "renderer crashed: {error} (restart attempt {restart_attempts}/{MAX_RESTART_ATTEMPTS})"
            ),
        );

        if restart_attempts > MAX_RESTART_ATTEMPTS {
            push_runtime_event(
                event_log,
                runtime_key,
                "error",
                format!(
                    "renderer stopped after {} failed restart attempts",
                    MAX_RESTART_ATTEMPTS
                ),
            );
            return;
        }

        thread::sleep(RESTART_BACKOFF);
        if stop.load(Ordering::Relaxed) {
            return;
        }

        push_runtime_event(
            event_log,
            runtime_key,
            "info",
            format!(
                "restarting {} renderer on {}",
                backend_label(spec.backend.clone()),
                spec.output_name
            ),
        );

        live_session = match restart_live_session(
            spec,
            pause,
            wayland,
            image,
            shader,
            status_map,
            event_log,
            runtime_key,
        ) {
            Ok(session) => {
                restart_attempts = 0;
                session
            }
            Err(error_status) => {
                set_runtime_status(status_map, runtime_key, error_status.clone());
                push_runtime_event(
                    event_log,
                    runtime_key,
                    "error",
                    failure_message(&error_status),
                );
                if restart_attempts >= MAX_RESTART_ATTEMPTS {
                    push_runtime_event(
                        event_log,
                        runtime_key,
                        "error",
                        format!(
                            "renderer stopped after {} failed restart attempts",
                            MAX_RESTART_ATTEMPTS
                        ),
                    );
                    return;
                }
                continue;
            }
        };
    }
}

fn restart_live_session(
    spec: &RendererLaunchSpec,
    pause: &PausePolicy,
    wayland: &LayerShellRuntime,
    image: &ImageRenderer,
    _shader: &ShaderRenderer,
    status_map: &Arc<Mutex<HashMap<String, RendererSessionStatus>>>,
    event_log: &Arc<Mutex<VecDeque<RuntimeEvent>>>,
    runtime_key: &str,
) -> Result<LiveSession, RendererSessionStatus> {
    match spec.backend {
        RendererBackend::Image => {
            let (mut session, detail) = start_image_session(wayland, image, spec)?;
            let status = ready_status(&mut session, spec, detail);
            set_runtime_status(status_map, runtime_key, status.clone());
            push_runtime_event(event_log, runtime_key, "info", ready_message(spec));
            if matches!(status, RendererSessionStatus::Ready { .. }) {
                Ok(LiveSession::Image(session))
            } else {
                Err(status)
            }
        }
        RendererBackend::Shader => {
            let child = start_shader_process_session(spec, pause)?;
            let status = RendererSessionStatus::Ready {
                output_name: spec.output_name.clone(),
                configured: true,
                persistent: true,
                paused_reason: None,
                detail: Some(shader_process_detail(spec)),
            };
            set_runtime_status(status_map, runtime_key, status.clone());
            push_runtime_event(event_log, runtime_key, "info", ready_message(spec));
            if matches!(status, RendererSessionStatus::Ready { .. }) {
                Ok(LiveSession::ShaderProcess(child))
            } else {
                Err(status)
            }
        }
        RendererBackend::Video => {
            let child = start_video_process_session(spec, pause)?;
            let status = RendererSessionStatus::Ready {
                output_name: spec.output_name.clone(),
                configured: true,
                persistent: true,
                paused_reason: None,
                detail: Some(video_process_detail(spec)),
            };
            set_runtime_status(status_map, runtime_key, status.clone());
            push_runtime_event(event_log, runtime_key, "info", ready_message(spec));
            if matches!(status, RendererSessionStatus::Ready { .. }) {
                Ok(LiveSession::VideoProcess(child))
            } else {
                Err(status)
            }
        }
        RendererBackend::Web => {
            let child = start_web_process_session(spec)?;
            let status = RendererSessionStatus::Ready {
                output_name: spec.output_name.clone(),
                configured: true,
                persistent: true,
                paused_reason: None,
                detail: Some(web_process_detail(spec)),
            };
            set_runtime_status(status_map, runtime_key, status.clone());
            push_runtime_event(event_log, runtime_key, "info", ready_message(spec));
            if matches!(status, RendererSessionStatus::Ready { .. }) {
                Ok(LiveSession::WebProcess(child))
            } else {
                Err(status)
            }
        }
        RendererBackend::Scene => {
            let child = start_scene_process_session(spec, pause)?;
            let status = RendererSessionStatus::Ready {
                output_name: spec.output_name.clone(),
                configured: true,
                persistent: true,
                paused_reason: None,
                detail: Some(scene_process_detail(spec)),
            };
            set_runtime_status(status_map, runtime_key, status.clone());
            push_runtime_event(event_log, runtime_key, "info", ready_message(spec));
            if matches!(status, RendererSessionStatus::Ready { .. }) {
                Ok(LiveSession::SceneProcess(child))
            } else {
                Err(status)
            }
        }
    }
}

fn backend_label(backend: RendererBackend) -> &'static str {
    match backend {
        RendererBackend::Image => "image",
        RendererBackend::Shader => "shader",
        RendererBackend::Video => "video",
        RendererBackend::Scene => "scene",
        RendererBackend::Web => "web",
    }
}

fn take_crash_flag(crash_flags: &Arc<Mutex<HashMap<String, bool>>>, runtime_key: &str) -> bool {
    crash_flags
        .lock()
        .expect("crash flags lock")
        .remove(runtime_key)
        .unwrap_or(false)
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
            let supply_type = read_trimmed(path.join("type"))?;

            match supply_type.as_str() {
                "Battery" => {
                    saw_battery = true;
                    if let Ok(status) = read_trimmed(path.join("status")) {
                        if status.eq_ignore_ascii_case("Discharging") {
                            return Ok(true);
                        }
                    }
                }
                "Mains" | "USB" => {
                    if let Ok(online) = read_trimmed(path.join("online")) {
                        if online == "1" {
                            saw_online_external_power = true;
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(saw_battery && !saw_online_external_power)
    }
}

fn read_trimmed(path: PathBuf) -> std::io::Result<String> {
    Ok(fs::read_to_string(path)?.trim().to_string())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use backlayer_types::{
        AssetMetadata, AssetSourceKind, AssignmentSettings, CompatibilityInfo, DaemonState,
        MonitorAssignment, MonitorInfo, PausePolicy, RuntimePlan, WallpaperKind,
    };

    use super::{PowerStateProbe, RendererBackend, RuntimeCoordinator, preview_fallback_spec};

    #[test]
    fn runtime_plan_maps_assignments_to_outputs() {
        let coordinator = RuntimeCoordinator::new(
            backlayer_wayland::LayerShellRuntime::new(),
            backlayer_renderer_image::ImageRenderer::default(),
            backlayer_renderer_shader::ShaderRenderer::default(),
            backlayer_renderer_video::VideoRenderer::default(),
        );
        let plan = coordinator.build_plan(&sample_state());

        assert_eq!(plan.unresolved_assignments.len(), 0);
        assert_eq!(plan.sessions.len(), 1);
        assert_eq!(plan.sessions[0].spec.output_name, "DP-3");
        assert!(matches!(
            plan.sessions[0].spec.backend,
            RendererBackend::Shader
        ));
    }

    #[test]
    fn power_probe_detects_battery_when_discharging() {
        let root = temp_power_supply_root("discharging");
        write_supply(&root, "BAT0", "Battery", Some("Discharging"), None);
        write_supply(&root, "AC", "Mains", None, Some("0"));

        let probe = PowerStateProbe {
            power_supply_root: root.clone(),
        };

        assert!(probe.on_battery_at(&root).expect("probe should succeed"));

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn power_probe_detects_external_power_when_ac_online() {
        let root = temp_power_supply_root("charging");
        write_supply(&root, "BAT0", "Battery", Some("Charging"), None);
        write_supply(&root, "AC", "Mains", None, Some("1"));

        let probe = PowerStateProbe {
            power_supply_root: root.clone(),
        };

        assert!(!probe.on_battery_at(&root).expect("probe should succeed"));

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn preview_fallback_spec_converts_imported_video_to_image_runtime() {
        let preview_root = temp_power_supply_root("preview-fallback");
        let preview = preview_root.join("preview.png");
        fs::write(&preview, b"png").expect("preview should write");

        let spec = backlayer_types::RendererLaunchSpec {
            monitor_id: "hypr:dell:u2720q:cn0xyz123456".into(),
            output_name: "DP-3".into(),
            asset: AssetMetadata {
                id: "we.video".into(),
                name: "Imported Video".into(),
                kind: WallpaperKind::Video,
                animated: false,
                image_fit: None,
                source_kind: AssetSourceKind::WallpaperEngineImport,
                preview_image: Some(preview.clone()),
                compatibility: CompatibilityInfo::default(),
                import_metadata: None,
                entrypoint: PathBuf::from("wallpaper.mp4"),
                asset_path: None,
            },
            backend: RendererBackend::Video,
        };

        let fallback = preview_fallback_spec(&spec).expect("fallback should exist");
        assert_eq!(fallback.backend, RendererBackend::Image);
        assert_eq!(fallback.asset.kind, WallpaperKind::Image);
        assert_eq!(fallback.asset.entrypoint, preview);

        fs::remove_dir_all(preview_root).ok();
    }

    fn sample_state() -> DaemonState {
        DaemonState {
            monitors: vec![MonitorInfo {
                id: "hypr:dell:u2720q:cn0xyz123456".into(),
                output_name: "DP-3".into(),
                description: "Dell Inc. DELL U2720Q".into(),
                make: "Dell Inc.".into(),
                model: "DELL U2720Q".into(),
                serial: Some("CN0XYZ123456".into()),
                width: 3840,
                height: 2160,
                x: 0,
                y: 0,
                scale: 1.0,
                refresh_rate: 60.0,
                focused: false,
                disabled: false,
            }],
            assignments: vec![MonitorAssignment {
                monitor_id: "hypr:dell:u2720q:cn0xyz123456".into(),
                wallpaper: AssetMetadata {
                    id: "demo.neon-grid".into(),
                    name: "Neon Grid".into(),
                    kind: WallpaperKind::Shader,
                    animated: false,
                    image_fit: None,
                    source_kind: AssetSourceKind::Native,
                    preview_image: None,
                    compatibility: CompatibilityInfo::default(),
                    import_metadata: None,
                    entrypoint: PathBuf::from("assets/demo.neon-grid/shaders/neon-grid.wgsl"),
                    asset_path: None,
                },
                settings: AssignmentSettings::default(),
            }],
            pause: PausePolicy {
                pause_on_fullscreen: true,
                pause_on_battery: true,
                fps_limit: 30,
            },
            runtime_dependencies: backlayer_types::RuntimeDependencies {
                video: backlayer_renderer_video::VideoRenderer::default().dependency_status(),
            },
            runtime: RuntimePlan::default(),
            recent_events: Vec::new(),
        }
    }

    fn temp_power_supply_root(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should advance")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("backlayer-power-{label}-{unique}"));
        fs::create_dir_all(&root).expect("temp root should exist");
        root
    }

    fn write_supply(
        root: &PathBuf,
        name: &str,
        supply_type: &str,
        status: Option<&str>,
        online: Option<&str>,
    ) {
        let dir = root.join(name);
        fs::create_dir_all(&dir).expect("supply dir should exist");
        fs::write(dir.join("type"), format!("{supply_type}\n")).expect("type should write");
        if let Some(status) = status {
            fs::write(dir.join("status"), format!("{status}\n")).expect("status should write");
        }
        if let Some(online) = online {
            fs::write(dir.join("online"), format!("{online}\n")).expect("online should write");
        }
    }
}
