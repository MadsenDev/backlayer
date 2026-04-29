use std::{
    fs, io,
    io::{Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use backlayer_config::ConfigStore;
use backlayer_renderer_image::ImageRenderer;
use backlayer_renderer_shader::ShaderRenderer;
use backlayer_renderer_video::VideoRenderer;
use backlayer_types::{
    AssetMetadata, AssignmentSettings, BacklayerConfig, CompositorClient, CreateNativeAssetRequest,
    CreateSceneAssetRequest, DaemonRequest, DaemonResponse, DaemonState, EditableSceneAsset,
    MonitorAssignment, PausePolicy, RuntimeDependencies,
};
use backlayer_wayland::LayerShellRuntime;
use tracing::{info, warn};

use crate::runtime::{RuntimeCoordinator, RuntimeManager};

#[cfg(test)]
#[derive(Debug)]
struct MockCompositorClient {
    monitors: Vec<backlayer_types::MonitorInfo>,
}

#[cfg(test)]
impl CompositorClient for MockCompositorClient {
    fn compositor_name(&self) -> &'static str {
        "mock"
    }
    fn discover_monitors(
        &self,
    ) -> Result<Vec<backlayer_types::MonitorInfo>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.monitors.clone())
    }
    fn fullscreen_active(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        Ok(false)
    }
}

pub fn serve_forever(
    socket_path: &Path,
    config_path: &Path,
    state: DaemonState,
    assets: Vec<AssetMetadata>,
    compositor: Arc<dyn CompositorClient>,
) -> Result<()> {
    let listener = bind_listener(socket_path)?;
    listener
        .set_nonblocking(true)
        .context("failed to configure nonblocking ipc listener")?;
    let mut server =
        IpcServer::new_persistent(config_path.to_path_buf(), state, assets, compositor);
    info!(path = %socket_path.display(), "daemon ipc server listening");

    serve_listener_until_stopped(&listener, &mut server, Arc::new(AtomicBool::new(false)))?;

    Ok(())
}

#[cfg(test)]
pub fn serve_once(
    socket_path: &Path,
    config_path: &Path,
    state: DaemonState,
    assets: Vec<AssetMetadata>,
) -> Result<()> {
    let listener = bind_listener(socket_path)?;
    let mut server = IpcServer::new(config_path.to_path_buf(), state, assets);
    if let Ok((stream, _)) = listener.accept() {
        server.handle_client(stream)?;
    }
    Ok(())
}

fn bind_listener(socket_path: &Path) -> Result<UnixListener> {
    if let Some(parent) = socket_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create socket directory {}", parent.display()))?;
    }

    if socket_path.exists() {
        fs::remove_file(socket_path)
            .with_context(|| format!("failed to remove stale socket {}", socket_path.display()))?;
    }

    UnixListener::bind(socket_path)
        .with_context(|| format!("failed to bind socket {}", socket_path.display()))
}

fn serve_listener_until_stopped(
    listener: &UnixListener,
    server: &mut IpcServer,
    shutdown: Arc<AtomicBool>,
) -> Result<()> {
    while !shutdown.load(Ordering::Relaxed) {
        server.tick();

        match listener.accept() {
            Ok((stream, _)) => {
                if let Err(error) = server.handle_client(stream) {
                    warn!(%error, "ipc client handling failed");
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(error) => warn!(%error, "ipc accept failed"),
        }
    }

    Ok(())
}

struct IpcServer {
    config_store: ConfigStore,
    compositor: Arc<dyn CompositorClient>,
    refresh_monitors_during_requests: bool,
    monitor_refresh_interval: Duration,
    last_monitor_refresh_at: Instant,
    config_path: PathBuf,
    state: DaemonState,
    assets: Vec<AssetMetadata>,
    runtime_manager: Option<RuntimeManager>,
}

impl IpcServer {
    #[cfg(test)]
    fn new(config_path: PathBuf, state: DaemonState, assets: Vec<AssetMetadata>) -> Self {
        let compositor = Arc::new(MockCompositorClient {
            monitors: state.monitors.clone(),
        });
        let mut server = Self {
            config_store: ConfigStore::default(),
            compositor,
            refresh_monitors_during_requests: false,
            monitor_refresh_interval: Duration::from_secs(5),
            last_monitor_refresh_at: Instant::now(),
            config_path,
            state,
            assets,
            runtime_manager: None,
        };
        server.refresh_runtime_dependencies();
        server.refresh_runtime_plan();
        server
    }

    fn new_persistent(
        config_path: PathBuf,
        state: DaemonState,
        assets: Vec<AssetMetadata>,
        compositor: Arc<dyn CompositorClient>,
    ) -> Self {
        let runtime_manager = RuntimeManager::new(
            LayerShellRuntime::new(),
            ImageRenderer::default(),
            ShaderRenderer::default(),
            VideoRenderer::default(),
            compositor.clone(),
        );
        let mut server = Self {
            config_store: ConfigStore::default(),
            compositor,
            refresh_monitors_during_requests: true,
            monitor_refresh_interval: Duration::from_secs(5),
            last_monitor_refresh_at: Instant::now()
                .checked_sub(Duration::from_secs(5))
                .unwrap_or_else(Instant::now),
            config_path,
            state,
            assets,
            runtime_manager: Some(runtime_manager),
        };
        server.refresh_runtime_dependencies();
        server.refresh_runtime_plan();
        server
    }

    fn handle_client(&mut self, mut stream: UnixStream) -> Result<()> {
        let mut payload = String::new();
        stream
            .read_to_string(&mut payload)
            .context("failed to read client request")?;

        let response = match serde_json::from_str::<DaemonRequest>(&payload) {
            Ok(request) => self.handle_request(request),
            Err(error) => DaemonResponse::Error {
                message: format!("invalid request: {error}"),
            },
        };

        let response = serde_json::to_vec(&response).context("failed to serialize response")?;
        stream
            .write_all(&response)
            .context("failed to write client response")?;
        Ok(())
    }

    fn handle_request(&mut self, request: DaemonRequest) -> DaemonResponse {
        if matches!(
            request,
            DaemonRequest::AssignWallpaper { .. }
                | DaemonRequest::UpdatePausePolicy { .. }
                | DaemonRequest::UpdateAssignmentSettings { .. }
                | DaemonRequest::RestartRendererSession { .. }
                | DaemonRequest::SimulateRendererCrash { .. }
        ) {
            self.refresh_monitors_if_due();
        }

        match request {
            DaemonRequest::GetState => {
                if let Some(runtime_manager) = &self.runtime_manager {
                    self.state.runtime = runtime_manager.snapshot();
                    self.state.recent_events = runtime_manager.recent_events();
                }
                DaemonResponse::State {
                    state: self.state.clone(),
                }
            }
            DaemonRequest::ListAssets => match self.refresh_assets() {
                Ok(()) => DaemonResponse::Assets {
                    assets: self.assets.clone(),
                },
                Err(message) => DaemonResponse::Error { message },
            },
            DaemonRequest::ImportWorkshopPath { path } => match self.import_workshop_path(path) {
                Ok(assets) => DaemonResponse::ImportResult { assets },
                Err(message) => DaemonResponse::Error { message },
            },
            DaemonRequest::ReimportAsset { asset_id } => match self.reimport_asset(asset_id) {
                Ok(()) => DaemonResponse::Ack,
                Err(message) => DaemonResponse::Error { message },
            },
            DaemonRequest::RemoveAsset { asset_id } => match self.remove_asset(asset_id) {
                Ok(()) => DaemonResponse::Ack,
                Err(message) => DaemonResponse::Error { message },
            },
            DaemonRequest::AssignWallpaper {
                monitor_id,
                asset_id,
            } => match self.assign_wallpaper(monitor_id, asset_id) {
                Ok(()) => DaemonResponse::Ack,
                Err(message) => DaemonResponse::Error { message },
            },
            DaemonRequest::UpdatePausePolicy { pause } => match self.update_pause_policy(pause) {
                Ok(()) => DaemonResponse::Ack,
                Err(message) => DaemonResponse::Error { message },
            },
            DaemonRequest::CreateNativeAsset { request } => match self.create_native_asset(request)
            {
                Ok(asset) => DaemonResponse::Asset { asset },
                Err(message) => DaemonResponse::Error { message },
            },
            DaemonRequest::CreateSceneAsset { request } => match self.create_scene_asset(request) {
                Ok(asset) => DaemonResponse::Asset { asset },
                Err(message) => DaemonResponse::Error { message },
            },
            DaemonRequest::LoadEditableSceneAsset { asset_id } => {
                match self.load_editable_scene_asset(asset_id) {
                    Ok(scene) => DaemonResponse::EditableSceneAsset { scene },
                    Err(message) => DaemonResponse::Error { message },
                }
            }
            DaemonRequest::UpdateAssignmentSettings {
                monitor_id,
                settings,
            } => match self.update_assignment_settings(monitor_id, settings) {
                Ok(()) => DaemonResponse::Ack,
                Err(message) => DaemonResponse::Error { message },
            },
            DaemonRequest::RestartRendererSession {
                monitor_id,
                asset_id,
            } => match self.restart_renderer_session(monitor_id, asset_id) {
                Ok(()) => DaemonResponse::Ack,
                Err(message) => DaemonResponse::Error { message },
            },
            DaemonRequest::SimulateRendererCrash {
                monitor_id,
                asset_id,
            } => match self.simulate_renderer_crash(monitor_id, asset_id) {
                Ok(()) => DaemonResponse::Ack,
                Err(message) => DaemonResponse::Error { message },
            },
        }
    }

    fn tick(&mut self) {
        self.refresh_monitors_if_due();
    }

    fn refresh_monitors_if_due(&mut self) {
        if !self.refresh_monitors_during_requests {
            return;
        }
        if self.last_monitor_refresh_at.elapsed() < self.monitor_refresh_interval {
            return;
        }
        self.last_monitor_refresh_at = Instant::now();

        match self.compositor.discover_monitors() {
            Ok(monitors) => {
                if monitors != self.state.monitors {
                    info!(
                        compositor = self.compositor.compositor_name(),
                        old_count = self.state.monitors.len(),
                        new_count = monitors.len(),
                        "monitor set changed; refreshing runtime"
                    );
                    self.state.monitors = monitors;
                    self.refresh_runtime_plan();
                }
            }
            Err(error) => {
                warn!(%error, "failed to refresh monitors");
            }
        }
    }

    fn assign_wallpaper(&mut self, monitor_id: String, asset_id: String) -> Result<(), String> {
        self.refresh_assets()?;

        let asset = self
            .assets
            .iter()
            .find(|asset| asset.id == asset_id)
            .cloned()
            .ok_or_else(|| format!("unknown asset id: {asset_id}"))?;

        if !self
            .state
            .monitors
            .iter()
            .any(|monitor| monitor.id == monitor_id || monitor.output_name == monitor_id)
        {
            return Err(format!("unknown monitor id: {monitor_id}"));
        }

        if let Some(existing) = self
            .state
            .assignments
            .iter_mut()
            .find(|assignment| assignment.monitor_id == monitor_id)
        {
            existing.wallpaper = asset;
            existing.settings = AssignmentSettings {
                image_fit: existing.wallpaper.image_fit.clone(),
            };
        } else {
            self.state.assignments.push(MonitorAssignment {
                monitor_id,
                wallpaper: asset,
                settings: AssignmentSettings {
                    image_fit: self
                        .assets
                        .iter()
                        .find(|candidate| candidate.id == asset_id)
                        .and_then(|candidate| candidate.image_fit.clone()),
                },
            });
        }

        self.refresh_runtime_plan();
        self.persist_config().map_err(|error| error.to_string())
    }

    fn create_scene_asset(
        &mut self,
        request: CreateSceneAssetRequest,
    ) -> Result<AssetMetadata, String> {
        self.refresh_assets()?;

        let base_asset = match request.base_asset_id.as_deref() {
            Some(base_asset_id) => Some(
                self.assets
                    .iter()
                    .find(|asset| asset.id == base_asset_id)
                    .cloned()
                    .ok_or_else(|| format!("unknown base asset id: {base_asset_id}"))?,
            ),
            None => None,
        };

        let asset = self
            .config_store
            .create_native_scene_asset(&request, base_asset.as_ref())
            .map_err(|error| error.to_string())?;

        self.refresh_assets()?;
        Ok(asset)
    }

    fn create_native_asset(
        &mut self,
        request: CreateNativeAssetRequest,
    ) -> Result<AssetMetadata, String> {
        let asset = self
            .config_store
            .create_native_file_asset(&request)
            .map_err(|error| error.to_string())?;

        self.refresh_assets()?;
        Ok(asset)
    }

    fn load_editable_scene_asset(
        &mut self,
        asset_id: String,
    ) -> Result<EditableSceneAsset, String> {
        self.refresh_assets()?;
        self.config_store
            .load_editable_scene_asset(&asset_id)
            .map_err(|error| error.to_string())
    }

    fn refresh_runtime_dependencies(&mut self) {
        self.state.runtime_dependencies = RuntimeDependencies {
            video: VideoRenderer::default().dependency_status(),
        };
    }

    fn update_pause_policy(&mut self, pause: PausePolicy) -> Result<(), String> {
        self.state.pause = pause;
        self.refresh_runtime_plan();
        self.persist_config().map_err(|error| error.to_string())
    }

    fn import_workshop_path(&mut self, path: PathBuf) -> Result<Vec<AssetMetadata>, String> {
        let imported = self
            .config_store
            .import_wallpaper_engine_path(path)
            .map_err(|error| format!("failed to import Wallpaper Engine item: {error}"))?;
        self.refresh_assets()?;
        self.refresh_assignment_assets();
        self.refresh_runtime_plan();
        self.persist_config().map_err(|error| error.to_string())?;
        Ok(imported)
    }

    fn reimport_asset(&mut self, asset_id: String) -> Result<(), String> {
        let source_path = self
            .assets
            .iter()
            .find(|asset| asset.id == asset_id)
            .and_then(|asset| asset.import_metadata.as_ref())
            .map(|metadata| metadata.source_path.clone())
            .ok_or_else(|| format!("asset is not a reimportable workshop item: {asset_id}"))?;

        self.config_store
            .import_wallpaper_engine_path(source_path)
            .map_err(|error| format!("failed to reimport asset {asset_id}: {error}"))?;
        self.refresh_assets()?;
        self.refresh_assignment_assets();
        self.refresh_runtime_plan();
        self.persist_config().map_err(|error| error.to_string())
    }

    fn remove_asset(&mut self, asset_id: String) -> Result<(), String> {
        let asset = self
            .assets
            .iter()
            .find(|asset| asset.id == asset_id)
            .cloned()
            .ok_or_else(|| format!("unknown asset id: {asset_id}"))?;

        self.config_store
            .remove_managed_asset(&asset)
            .map_err(|error| format!("failed to remove asset {asset_id}: {error}"))?;

        self.state
            .assignments
            .retain(|assignment| assignment.wallpaper.id != asset_id);
        self.refresh_assets()?;
        self.refresh_runtime_plan();
        self.persist_config().map_err(|error| error.to_string())
    }

    fn update_assignment_settings(
        &mut self,
        monitor_id: String,
        settings: AssignmentSettings,
    ) -> Result<(), String> {
        let assignment = self
            .state
            .assignments
            .iter_mut()
            .find(|assignment| assignment.monitor_id == monitor_id)
            .ok_or_else(|| format!("unknown assignment for monitor id: {monitor_id}"))?;

        assignment.settings = settings;
        self.refresh_runtime_plan();
        self.persist_config().map_err(|error| error.to_string())
    }

    fn simulate_renderer_crash(
        &mut self,
        monitor_id: String,
        asset_id: String,
    ) -> Result<(), String> {
        let runtime_manager = self.runtime_manager.as_ref().ok_or_else(|| {
            "renderer crash simulation is only available in --serve mode".to_string()
        })?;
        runtime_manager.simulate_crash(&monitor_id, &asset_id)?;
        self.state.runtime = runtime_manager.snapshot();
        self.state.recent_events = runtime_manager.recent_events();
        Ok(())
    }

    fn restart_renderer_session(
        &mut self,
        monitor_id: String,
        asset_id: String,
    ) -> Result<(), String> {
        if !self.state.runtime.sessions.iter().any(|session| {
            session.spec.monitor_id == monitor_id && session.spec.asset.id == asset_id
        }) {
            return Err(format!(
                "unknown runtime session: {}:{}",
                monitor_id, asset_id
            ));
        }

        self.refresh_runtime_plan();
        Ok(())
    }

    fn persist_config(&self) -> Result<()> {
        let config = BacklayerConfig {
            assignments: self.state.assignments.clone(),
            pause: self.state.pause.clone(),
            ipc: backlayer_types::IpcTransport::UnixSocket {
                path: self.config_store.default_socket_path(),
            },
        };

        self.config_store
            .save_to_path(&self.config_path, &config)
            .context("failed to persist config")
    }

    fn refresh_runtime_plan(&mut self) {
        self.state.runtime = if let Some(runtime_manager) = &mut self.runtime_manager {
            runtime_manager.apply(&self.state)
        } else {
            RuntimeCoordinator::new(
                LayerShellRuntime::new(),
                ImageRenderer::default(),
                ShaderRenderer::default(),
                VideoRenderer::default(),
            )
            .start(&self.state)
        };
    }

    fn refresh_assignment_assets(&mut self) {
        for assignment in &mut self.state.assignments {
            if let Some(asset) = self
                .assets
                .iter()
                .find(|asset| asset.id == assignment.wallpaper.id)
                .cloned()
            {
                assignment.wallpaper = asset;
            }
        }
    }

    fn refresh_assets(&mut self) -> Result<(), String> {
        let discovered_assets = self
            .config_store
            .discover_all_assets()
            .map_err(|error| format!("failed to refresh assets: {error}"))?;

        if !discovered_assets.is_empty() {
            self.assets = discovered_assets;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::{Read, Write},
        net::Shutdown,
        os::unix::net::UnixStream,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        thread,
        time::Duration,
    };

    use backlayer_config::ConfigStore;
    use backlayer_renderer_video::VideoRenderer;
    use backlayer_types::{
        AssetMetadata, AssetSourceKind, AssignmentSettings, CompatibilityInfo, DaemonRequest,
        DaemonResponse, DaemonState, MonitorAssignment, MonitorInfo, PausePolicy,
        RuntimeDependencies, RuntimePlan, WallpaperKind,
    };

    use super::{
        IpcServer, MockCompositorClient, bind_listener, serve_listener_until_stopped, serve_once,
    };

    #[test]
    fn ipc_server_returns_state_response() {
        let socket_path =
            std::env::temp_dir().join(format!("backlayer-test-{}.sock", std::process::id()));
        let config_path =
            std::env::temp_dir().join(format!("backlayer-test-{}.toml", std::process::id()));
        let state = sample_state();
        let assets = vec![sample_asset()];
        let server_path = socket_path.clone();
        let server_config_path = config_path.clone();

        let handle = thread::spawn(move || {
            serve_once(&server_path, &server_config_path, state, assets)
                .expect("ipc server should serve a request");
        });

        thread::sleep(Duration::from_millis(100));

        let mut stream = UnixStream::connect(&socket_path).expect("client should connect");
        let request = serde_json::to_vec(&DaemonRequest::GetState).expect("request should encode");
        stream.write_all(&request).expect("request should write");
        stream
            .shutdown(Shutdown::Write)
            .expect("shutdown write half");

        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .expect("response should read");
        let decoded: DaemonResponse =
            serde_json::from_str(&response).expect("response should decode");

        match decoded {
            DaemonResponse::State { state } => {
                assert_eq!(state.monitors.len(), 1);
                assert_eq!(state.assignments.len(), 1);
            }
            other => panic!("unexpected response: {other:?}"),
        }

        handle.join().expect("server thread should finish");
        fs::remove_file(&socket_path).ok();
        fs::remove_file(&config_path).ok();
    }

    #[test]
    fn ipc_server_updates_assignment_and_persists_config() {
        let socket_path =
            std::env::temp_dir().join(format!("backlayer-assign-{}.sock", std::process::id()));
        let config_path =
            std::env::temp_dir().join(format!("backlayer-assign-{}.toml", std::process::id()));
        let state = sample_state_without_assignments();
        let assets = vec![sample_asset()];
        let server_path = socket_path.clone();
        let server_config_path = config_path.clone();

        let handle = thread::spawn(move || {
            serve_once(&server_path, &server_config_path, state, assets)
                .expect("ipc server should serve a request");
        });

        thread::sleep(Duration::from_millis(100));

        let mut stream = UnixStream::connect(&socket_path).expect("client should connect");
        let request = serde_json::to_vec(&DaemonRequest::AssignWallpaper {
            monitor_id: "hypr:dell:u2720q:cn0xyz123456".into(),
            asset_id: "demo.neon-grid".into(),
        })
        .expect("request should encode");
        stream.write_all(&request).expect("request should write");
        stream
            .shutdown(Shutdown::Write)
            .expect("shutdown write half");

        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .expect("response should read");
        let decoded: DaemonResponse =
            serde_json::from_str(&response).expect("response should decode");

        assert!(matches!(decoded, DaemonResponse::Ack));
        handle.join().expect("server thread should finish");

        let config = fs::read_to_string(&config_path).expect("config should be written");
        assert!(config.contains("monitor_id = \"hypr:dell:u2720q:cn0xyz123456\""));
        assert!(config.contains("id = \"demo.neon-grid\""));

        fs::remove_file(&socket_path).ok();
        fs::remove_file(&config_path).ok();
    }

    #[test]
    fn assignment_refreshes_runtime_plan_in_memory() {
        let mut server = IpcServer::new(
            std::env::temp_dir().join("backlayer-runtime-refresh.toml"),
            sample_state_without_assignments(),
            vec![sample_asset()],
        );

        let response = server.handle_request(DaemonRequest::AssignWallpaper {
            monitor_id: "hypr:dell:u2720q:cn0xyz123456".into(),
            asset_id: "demo.neon-grid".into(),
        });

        assert!(matches!(response, DaemonResponse::Ack));
        assert_eq!(server.state.runtime.sessions.len(), 1);
    }

    #[test]
    fn persistent_server_smoke_serves_runtime_state() {
        let socket_path =
            std::env::temp_dir().join(format!("backlayer-smoke-{}.sock", std::process::id()));
        let config_path =
            std::env::temp_dir().join(format!("backlayer-smoke-{}.toml", std::process::id()));
        let listener = bind_listener(&socket_path).expect("listener should bind");
        listener
            .set_nonblocking(true)
            .expect("listener should become nonblocking");
        let shutdown = Arc::new(AtomicBool::new(false));
        let server_shutdown = shutdown.clone();

        let handle = thread::spawn(move || {
            let compositor = Arc::new(MockCompositorClient {
                monitors: sample_state().monitors,
            });
            let mut server = IpcServer::new_persistent(
                config_path,
                sample_state(),
                vec![sample_asset()],
                compositor,
            );
            serve_listener_until_stopped(&listener, &mut server, server_shutdown)
                .expect("persistent server should run");
        });

        thread::sleep(Duration::from_millis(150));

        let mut stream = UnixStream::connect(&socket_path).expect("client should connect");
        let request = serde_json::to_vec(&DaemonRequest::GetState).expect("request should encode");
        stream.write_all(&request).expect("request should write");
        stream
            .shutdown(Shutdown::Write)
            .expect("shutdown write half");

        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .expect("response should read");
        let decoded: DaemonResponse =
            serde_json::from_str(&response).expect("response should decode");

        match decoded {
            DaemonResponse::State { state } => {
                assert_eq!(state.monitors.len(), 1);
                assert_eq!(state.assignments.len(), 1);
            }
            other => panic!("unexpected response: {other:?}"),
        }

        shutdown.store(true, Ordering::Relaxed);
        handle.join().expect("server thread should finish");
        fs::remove_file(&socket_path).ok();
    }

    #[test]
    fn reimport_refreshes_assigned_imported_asset_metadata() {
        let root = std::env::temp_dir().join(format!("backlayer-reimport-{}", std::process::id()));
        fs::create_dir_all(&root).expect("root should exist");
        let workshop_item = root.join("100");
        fs::create_dir_all(&workshop_item).expect("workshop item should exist");
        unsafe {
            std::env::set_var("BACKLAYER_ENABLE_WORKSHOP", "1");
        }
        fs::write(
            workshop_item.join("project.json"),
            r#"{"title":"Original Title","type":"web","file":"index.html","workshopid":"100"}"#,
        )
        .expect("project should write");
        fs::write(workshop_item.join("index.html"), "<html></html>").expect("index should write");
        unsafe {
            std::env::set_var("HOME", &root);
        }

        let store = ConfigStore::default();
        let imported = store
            .import_wallpaper_engine_path(&workshop_item)
            .expect("initial import should succeed");
        let imported_asset = imported.into_iter().next().expect("asset should exist");

        let mut server = IpcServer::new(
            root.join("config.toml"),
            DaemonState {
                monitors: sample_state().monitors,
                assignments: vec![MonitorAssignment {
                    monitor_id: "hypr:dell:u2720q:cn0xyz123456".into(),
                    wallpaper: imported_asset.clone(),
                    settings: AssignmentSettings::default(),
                }],
                pause: sample_state().pause,
                runtime_dependencies: sample_state().runtime_dependencies,
                runtime: RuntimePlan::default(),
                recent_events: Vec::new(),
            },
            vec![imported_asset.clone()],
        );

        fs::write(
            workshop_item.join("project.json"),
            r#"{"title":"Updated Title","type":"web","file":"index.html","workshopid":"100"}"#,
        )
        .expect("updated project should write");

        let response = server.handle_request(DaemonRequest::ReimportAsset {
            asset_id: imported_asset.id.clone(),
        });

        assert!(matches!(response, DaemonResponse::Ack));
        assert_eq!(server.state.assignments[0].wallpaper.name, "Updated Title");

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn removing_imported_asset_drops_assignment_and_asset() {
        let root = std::env::temp_dir().join(format!("backlayer-remove-{}", std::process::id()));
        fs::create_dir_all(&root).expect("root should exist");
        let workshop_item = root.join("101");
        fs::create_dir_all(&workshop_item).expect("workshop item should exist");
        unsafe {
            std::env::set_var("BACKLAYER_ENABLE_WORKSHOP", "1");
        }
        fs::write(
            workshop_item.join("project.json"),
            r#"{"title":"Removable Item","type":"web","file":"index.html","workshopid":"101"}"#,
        )
        .expect("project should write");
        fs::write(workshop_item.join("index.html"), "<html></html>").expect("index should write");
        unsafe {
            std::env::set_var("HOME", &root);
        }

        let store = ConfigStore::default();
        let imported = store
            .import_wallpaper_engine_path(&workshop_item)
            .expect("initial import should succeed");
        let imported_asset = imported.into_iter().next().expect("asset should exist");

        let mut server = IpcServer::new(
            root.join("config.toml"),
            DaemonState {
                monitors: sample_state().monitors,
                assignments: vec![MonitorAssignment {
                    monitor_id: "hypr:dell:u2720q:cn0xyz123456".into(),
                    wallpaper: imported_asset.clone(),
                    settings: AssignmentSettings::default(),
                }],
                pause: sample_state().pause,
                runtime_dependencies: sample_state().runtime_dependencies,
                runtime: RuntimePlan::default(),
                recent_events: Vec::new(),
            },
            vec![imported_asset.clone()],
        );

        let response = server.handle_request(DaemonRequest::RemoveAsset {
            asset_id: imported_asset.id.clone(),
        });

        assert!(matches!(response, DaemonResponse::Ack));
        assert!(server.state.assignments.is_empty());
        assert!(
            !server
                .assets
                .iter()
                .any(|asset| asset.id == imported_asset.id)
        );

        fs::remove_dir_all(root).ok();
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
                wallpaper: sample_asset(),
                settings: AssignmentSettings::default(),
            }],
            pause: PausePolicy {
                pause_on_fullscreen: true,
                pause_on_battery: true,
                fps_limit: 30,
            },
            runtime_dependencies: RuntimeDependencies {
                video: VideoRenderer::default().dependency_status(),
            },
            runtime: RuntimePlan::default(),
            recent_events: Vec::new(),
        }
    }

    fn sample_state_without_assignments() -> DaemonState {
        let mut state = sample_state();
        state.assignments.clear();
        state
    }

    fn sample_asset() -> AssetMetadata {
        AssetMetadata {
            id: "demo.neon-grid".into(),
            name: "Neon Grid".into(),
            kind: WallpaperKind::Shader,
            animated: false,
            image_fit: None,
            source_kind: AssetSourceKind::Native,
            preview_image: None,
            compatibility: CompatibilityInfo::default(),
            import_metadata: None,
            entrypoint: "assets/demo.neon-grid/shaders/neon-grid.wgsl".into(),
            asset_path: None,
        }
    }
}
