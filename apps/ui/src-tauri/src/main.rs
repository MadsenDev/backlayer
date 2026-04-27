#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    fs,
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

use backlayer_config::ConfigStore;
use backlayer_types::{
    AssetMetadata, AssignmentSettings, CreateNativeAssetRequest, CreateSceneAssetRequest,
    DaemonRequest, DaemonResponse, DaemonState, EditableSceneAsset, FeatureFlags, PausePolicy,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use image::ImageFormat;

#[tauri::command]
async fn daemon_get_state() -> Result<DaemonState, String> {
    run_blocking(|| match daemon_request(DaemonRequest::GetState) {
        Ok(DaemonResponse::State { state }) => Ok(state),
        Ok(DaemonResponse::Error { message }) => Err(message),
        Ok(other) => Err(format!("unexpected daemon response: {other:?}")),
        Err(error) => Err(error),
    })
    .await
}

#[tauri::command]
async fn daemon_list_assets() -> Result<Vec<AssetMetadata>, String> {
    run_blocking(|| match daemon_request(DaemonRequest::ListAssets) {
        Ok(DaemonResponse::Assets { assets }) => Ok(assets),
        Ok(DaemonResponse::Error { message }) => Err(message),
        Ok(other) => Err(format!("unexpected daemon response: {other:?}")),
        Err(error) => Err(error),
    })
    .await
}

#[tauri::command]
fn daemon_feature_flags() -> FeatureFlags {
    FeatureFlags {
        workshop_enabled: ConfigStore::default().workshop_enabled(),
    }
}

#[tauri::command]
async fn daemon_assign_wallpaper(monitor_id: String, asset_id: String) -> Result<(), String> {
    run_blocking(move || {
        match daemon_request(DaemonRequest::AssignWallpaper {
            monitor_id,
            asset_id,
        }) {
            Ok(DaemonResponse::Ack) => Ok(()),
            Ok(DaemonResponse::Error { message }) => Err(message),
            Ok(other) => Err(format!("unexpected daemon response: {other:?}")),
            Err(error) => Err(error),
        }
    })
    .await
}

#[tauri::command]
async fn daemon_update_pause_policy(pause: PausePolicy) -> Result<(), String> {
    run_blocking(
        move || match daemon_request(DaemonRequest::UpdatePausePolicy { pause }) {
            Ok(DaemonResponse::Ack) => Ok(()),
            Ok(DaemonResponse::Error { message }) => Err(message),
            Ok(other) => Err(format!("unexpected daemon response: {other:?}")),
            Err(error) => Err(error),
        },
    )
    .await
}

#[tauri::command]
async fn daemon_update_assignment_settings(
    monitor_id: String,
    settings: AssignmentSettings,
) -> Result<(), String> {
    run_blocking(move || {
        match daemon_request(DaemonRequest::UpdateAssignmentSettings {
            monitor_id,
            settings,
        }) {
            Ok(DaemonResponse::Ack) => Ok(()),
            Ok(DaemonResponse::Error { message }) => Err(message),
            Ok(other) => Err(format!("unexpected daemon response: {other:?}")),
            Err(error) => Err(error),
        }
    })
    .await
}

#[tauri::command]
async fn daemon_create_native_asset(
    request: CreateNativeAssetRequest,
) -> Result<AssetMetadata, String> {
    run_blocking(
        move || match daemon_request(DaemonRequest::CreateNativeAsset { request }) {
            Ok(DaemonResponse::Asset { asset }) => Ok(asset),
            Ok(DaemonResponse::Error { message }) => Err(message),
            Ok(other) => Err(format!("unexpected daemon response: {other:?}")),
            Err(error) => Err(error),
        },
    )
    .await
}

#[tauri::command]
async fn daemon_create_scene_asset(
    request: CreateSceneAssetRequest,
) -> Result<AssetMetadata, String> {
    run_blocking(
        move || match daemon_request(DaemonRequest::CreateSceneAsset { request }) {
            Ok(DaemonResponse::Asset { asset }) => Ok(asset),
            Ok(DaemonResponse::Error { message }) => Err(message),
            Ok(other) => Err(format!("unexpected daemon response: {other:?}")),
            Err(error) => Err(error),
        },
    )
    .await
}

#[tauri::command]
async fn daemon_load_editable_scene_asset(asset_id: String) -> Result<EditableSceneAsset, String> {
    run_blocking(
        move || match daemon_request(DaemonRequest::LoadEditableSceneAsset { asset_id }) {
            Ok(DaemonResponse::EditableSceneAsset { scene }) => Ok(scene),
            Ok(DaemonResponse::Error { message }) => Err(message),
            Ok(other) => Err(format!("unexpected daemon response: {other:?}")),
            Err(error) => Err(error),
        },
    )
    .await
}

#[tauri::command]
async fn daemon_simulate_renderer_crash(
    monitor_id: String,
    asset_id: String,
) -> Result<(), String> {
    run_blocking(move || {
        match daemon_request(DaemonRequest::SimulateRendererCrash {
            monitor_id,
            asset_id,
        }) {
            Ok(DaemonResponse::Ack) => Ok(()),
            Ok(DaemonResponse::Error { message }) => Err(message),
            Ok(other) => Err(format!("unexpected daemon response: {other:?}")),
            Err(error) => Err(error),
        }
    })
    .await
}

#[tauri::command]
async fn daemon_import_workshop_path(path: String) -> Result<Vec<AssetMetadata>, String> {
    run_blocking(move || {
        match daemon_request(DaemonRequest::ImportWorkshopPath { path: path.into() }) {
            Ok(DaemonResponse::ImportResult { assets }) => Ok(assets),
            Ok(DaemonResponse::Error { message }) => Err(message),
            Ok(other) => Err(format!("unexpected daemon response: {other:?}")),
            Err(error) => Err(error),
        }
    })
    .await
}

#[tauri::command]
async fn daemon_suggest_workshop_paths() -> Vec<String> {
    run_blocking(|| {
        Ok::<Vec<String>, String>(
            ConfigStore::default()
                .discover_wallpaper_engine_workshop_paths()
                .into_iter()
                .map(|path| path.display().to_string())
                .collect(),
        )
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
async fn daemon_reimport_asset(asset_id: String) -> Result<(), String> {
    run_blocking(
        move || match daemon_request(DaemonRequest::ReimportAsset { asset_id }) {
            Ok(DaemonResponse::Ack) => Ok(()),
            Ok(DaemonResponse::Error { message }) => Err(message),
            Ok(other) => Err(format!("unexpected daemon response: {other:?}")),
            Err(error) => Err(error),
        },
    )
    .await
}

#[tauri::command]
async fn daemon_remove_asset(asset_id: String) -> Result<(), String> {
    run_blocking(
        move || match daemon_request(DaemonRequest::RemoveAsset { asset_id }) {
            Ok(DaemonResponse::Ack) => Ok(()),
            Ok(DaemonResponse::Error { message }) => Err(message),
            Ok(other) => Err(format!("unexpected daemon response: {other:?}")),
            Err(error) => Err(error),
        },
    )
    .await
}

#[tauri::command]
async fn asset_preview_data_url(path: String) -> Result<String, String> {
    run_blocking(move || {
        let bytes =
            fs::read(&path).map_err(|error| format!("failed to read preview source: {error}"))?;
        let image = image::load_from_memory(&bytes)
            .map_err(|error| format!("failed to decode preview source: {error}"))?;

        let mut encoded = Vec::new();
        image
            .write_to(&mut std::io::Cursor::new(&mut encoded), ImageFormat::Png)
            .map_err(|error| format!("failed to encode preview source: {error}"))?;

        Ok(format!(
            "data:image/png;base64,{}",
            STANDARD.encode(encoded)
        ))
    })
    .await
}

async fn run_blocking<T, F>(operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(operation)
        .await
        .map_err(|error| format!("blocking task failed: {error}"))?
}

fn daemon_request(request: DaemonRequest) -> Result<DaemonResponse, String> {
    let store = ConfigStore::default();
    let socket_path = store
        .resolve_path(store.default_socket_path())
        .map_err(|error| error.to_string())?;

    let mut stream = UnixStream::connect(&socket_path)
        .map_err(|error| format!("failed to connect to {}: {error}", socket_path.display()))?;
    let payload = serde_json::to_vec(&request).map_err(|error| error.to_string())?;
    stream
        .write_all(&payload)
        .map_err(|error| format!("failed to write daemon request: {error}"))?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|error| format!("failed to finalize daemon request: {error}"))?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("failed to read daemon response: {error}"))?;

    serde_json::from_str(&response)
        .map_err(|error| format!("failed to decode daemon response: {error}"))
}

fn daemon_binary_path() -> std::path::PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("backlayerd");
            if candidate.exists() {
                return candidate;
            }
        }
    }
    std::path::PathBuf::from("backlayerd")
}

fn socket_is_alive(socket_path: &Path) -> bool {
    UnixStream::connect(socket_path).is_ok()
}

fn ensure_daemon_running() {
    let store = ConfigStore::default();
    let socket_path = match store.resolve_path(store.default_socket_path()) {
        Ok(p) => p,
        Err(_) => return,
    };

    if socket_is_alive(&socket_path) {
        return;
    }

    let _ = Command::new(daemon_binary_path())
        .arg("--serve")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    // Child handle dropped — daemon outlives the UI process

    for _ in 0..6 {
        std::thread::sleep(Duration::from_millis(500));
        if socket_is_alive(&socket_path) {
            break;
        }
    }
}

fn main() {
    tauri::Builder::default()
        .setup(|_app| {
            ensure_daemon_running();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            daemon_get_state,
            daemon_list_assets,
            daemon_feature_flags,
            daemon_assign_wallpaper,
            daemon_create_native_asset,
            daemon_create_scene_asset,
            daemon_load_editable_scene_asset,
            daemon_update_assignment_settings,
            daemon_update_pause_policy,
            daemon_simulate_renderer_crash,
            daemon_import_workshop_path,
            daemon_suggest_workshop_paths,
            daemon_reimport_asset,
            daemon_remove_asset,
            asset_preview_data_url
        ])
        .run(tauri::generate_context!())
        .expect("error while running backlayer ui");
}
