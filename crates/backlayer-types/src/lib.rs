use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetMetadata {
    pub id: String,
    pub name: String,
    pub kind: WallpaperKind,
    #[serde(default)]
    pub animated: bool,
    #[serde(default)]
    pub image_fit: Option<ImageFitMode>,
    #[serde(default)]
    pub source_kind: AssetSourceKind,
    #[serde(default)]
    pub preview_image: Option<PathBuf>,
    #[serde(default)]
    pub compatibility: CompatibilityInfo,
    #[serde(default)]
    pub import_metadata: Option<ImportMetadata>,
    pub entrypoint: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AssetSourceKind {
    #[default]
    Native,
    WallpaperEngineImport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CompatibilityInfo {
    #[serde(default)]
    pub status: CompatibilityStatus,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityStatus {
    #[default]
    Supported,
    Partial,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportMetadata {
    pub source_app: ImportSourceApp,
    pub source_path: PathBuf,
    #[serde(default)]
    pub manifest_path: Option<PathBuf>,
    #[serde(default)]
    pub workshop_id: Option<String>,
    #[serde(default)]
    pub original_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportSourceApp {
    WallpaperEngine,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageFitMode {
    Cover,
    Contain,
    Stretch,
    Center,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WallpaperKind {
    Image,
    Video,
    Shader,
    Scene,
    Web,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RendererBackend {
    Image,
    Shader,
    Video,
    Scene,
    Web,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MonitorAssignment {
    pub monitor_id: String,
    pub wallpaper: AssetMetadata,
    #[serde(default)]
    pub settings: AssignmentSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AssignmentSettings {
    pub image_fit: Option<ImageFitMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SceneBlendMode {
    Alpha,
    Add,
    Screen,
    Multiply,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SceneBehaviorKind {
    Drift,
    Pulse,
    Orbit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneBehavior {
    pub kind: SceneBehaviorKind,
    #[serde(default)]
    pub speed: f32,
    #[serde(default)]
    pub amount_x: f32,
    #[serde(default)]
    pub amount_y: f32,
    #[serde(default)]
    pub amount: f32,
    #[serde(default)]
    pub phase: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneImageSource {
    pub key: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SceneEffectKind {
    Glow,
    Vignette,
    Scanlines,
    Fog,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneEffectNode {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub effect: SceneEffectKind,
    #[serde(default)]
    pub color_hex: Option<String>,
    #[serde(default)]
    pub opacity: f32,
    #[serde(default)]
    pub intensity: f32,
    #[serde(default)]
    pub speed: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SceneEmitterPreset {
    Embers,
    Rain,
    Dust,
    Snow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SceneEmitterShape {
    Point,
    Box,
    Line,
    Circle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneCurvePoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneColorStop {
    pub x: f32,
    pub color_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneEmitterNode {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub preset: SceneEmitterPreset,
    #[serde(default)]
    pub shape: Option<SceneEmitterShape>,
    #[serde(default)]
    pub origin_x: Option<f32>,
    #[serde(default)]
    pub origin_y: Option<f32>,
    #[serde(default)]
    pub direction_deg: Option<f32>,
    #[serde(default)]
    pub region_width: Option<f32>,
    #[serde(default)]
    pub region_height: Option<f32>,
    #[serde(default)]
    pub region_radius: Option<f32>,
    #[serde(default)]
    pub line_length: Option<f32>,
    #[serde(default)]
    pub line_angle_deg: Option<f32>,
    #[serde(default)]
    pub emission_rate: f32,
    #[serde(default)]
    pub burst_count: u32,
    #[serde(default)]
    pub burst_on_start: bool,
    #[serde(default)]
    pub max_particles: u32,
    #[serde(default)]
    pub opacity: f32,
    #[serde(default)]
    pub size: f32,
    #[serde(default)]
    pub speed: f32,
    #[serde(default)]
    pub min_speed: Option<f32>,
    #[serde(default)]
    pub max_speed: Option<f32>,
    #[serde(default)]
    pub min_life: Option<f32>,
    #[serde(default)]
    pub max_life: Option<f32>,
    #[serde(default)]
    pub spread: f32,
    #[serde(default)]
    pub gravity_x: f32,
    #[serde(default)]
    pub gravity_y: f32,
    #[serde(default)]
    pub drag: f32,
    #[serde(default)]
    pub color_hex: Option<String>,
    #[serde(default)]
    pub particle_image_key: Option<String>,
    #[serde(default)]
    pub size_curve: Vec<SceneCurvePoint>,
    #[serde(default)]
    pub alpha_curve: Vec<SceneCurvePoint>,
    #[serde(default)]
    pub color_curve: Vec<SceneColorStop>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneSpriteNode {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub image_key: String,
    #[serde(default)]
    pub fit: Option<ImageFitMode>,
    #[serde(default)]
    pub blend: Option<SceneBlendMode>,
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default = "default_one")]
    pub scale: f32,
    #[serde(default)]
    pub rotation_deg: f32,
    #[serde(default = "default_one")]
    pub opacity: f32,
    #[serde(default)]
    pub behaviors: Vec<SceneBehavior>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SceneNode {
    Sprite(SceneSpriteNode),
    Effect(SceneEffectNode),
    Emitter(SceneEmitterNode),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NativeSceneDocument {
    pub schema: String,
    pub version: u32,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub images: Vec<SceneImageSource>,
    #[serde(default)]
    pub nodes: Vec<SceneNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateSceneImageSourceRequest {
    pub key: String,
    pub data_url: String,
    pub filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateSceneAssetRequest {
    pub name: String,
    #[serde(default)]
    pub existing_asset_id: Option<String>,
    #[serde(default)]
    pub base_asset_id: Option<String>,
    #[serde(default)]
    pub base_image_data_url: Option<String>,
    #[serde(default)]
    pub base_image_filename: Option<String>,
    #[serde(default)]
    pub extra_images: Vec<CreateSceneImageSourceRequest>,
    #[serde(default)]
    pub nodes: Vec<SceneNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditableSceneImage {
    pub key: String,
    pub data_url: String,
    pub filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditableSceneAsset {
    pub asset: AssetMetadata,
    pub document: NativeSceneDocument,
    pub images: Vec<EditableSceneImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MonitorInfo {
    pub id: String,
    pub output_name: String,
    pub description: String,
    pub make: String,
    pub model: String,
    pub serial: Option<String>,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub scale: f64,
    pub refresh_rate: f64,
    pub focused: bool,
    pub disabled: bool,
}

impl MonitorInfo {
    pub fn matches_assignment(&self, assignment: &MonitorAssignment) -> bool {
        self.id == assignment.monitor_id || self.output_name == assignment.monitor_id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PausePolicy {
    pub pause_on_fullscreen: bool,
    pub pause_on_battery: bool,
    pub fps_limit: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IpcTransport {
    UnixSocket { path: PathBuf },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BacklayerConfig {
    pub assignments: Vec<MonitorAssignment>,
    pub pause: PausePolicy,
    pub ipc: IpcTransport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RendererLaunchSpec {
    pub monitor_id: String,
    pub output_name: String,
    pub asset: AssetMetadata,
    pub backend: RendererBackend,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RendererSessionStatus {
    Ready {
        output_name: String,
        configured: bool,
        persistent: bool,
        paused_reason: Option<String>,
        detail: Option<String>,
    },
    Unsupported {
        reason: String,
    },
    Failed {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RendererSession {
    pub spec: RendererLaunchSpec,
    pub status: RendererSessionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeEvent {
    pub runtime_key: String,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RuntimeDependencies {
    pub video: RendererDependencyStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RendererDependencyStatus {
    pub available: bool,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FeatureFlags {
    #[serde(default = "default_false")]
    pub workshop_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RuntimePlan {
    pub sessions: Vec<RendererSession>,
    pub unresolved_assignments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonRequest {
    GetState,
    ListAssets,
    ImportWorkshopPath {
        path: PathBuf,
    },
    ReimportAsset {
        asset_id: String,
    },
    RemoveAsset {
        asset_id: String,
    },
    AssignWallpaper {
        monitor_id: String,
        asset_id: String,
    },
    UpdateAssignmentSettings {
        monitor_id: String,
        settings: AssignmentSettings,
    },
    UpdatePausePolicy {
        pause: PausePolicy,
    },
    CreateSceneAsset {
        request: CreateSceneAssetRequest,
    },
    LoadEditableSceneAsset {
        asset_id: String,
    },
    RestartRendererSession {
        monitor_id: String,
        asset_id: String,
    },
    SimulateRendererCrash {
        monitor_id: String,
        asset_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DaemonState {
    pub monitors: Vec<MonitorInfo>,
    pub assignments: Vec<MonitorAssignment>,
    pub pause: PausePolicy,
    #[serde(default)]
    pub runtime_dependencies: RuntimeDependencies,
    pub runtime: RuntimePlan,
    pub recent_events: Vec<RuntimeEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonResponse {
    State { state: DaemonState },
    Assets { assets: Vec<AssetMetadata> },
    ImportResult { assets: Vec<AssetMetadata> },
    Asset { asset: AssetMetadata },
    EditableSceneAsset { scene: EditableSceneAsset },
    Ack,
    Error { message: String },
}

fn default_false() -> bool {
    false
}

fn default_true() -> bool {
    true
}

fn default_one() -> f32 {
    1.0
}

impl Default for BacklayerConfig {
    fn default() -> Self {
        Self {
            assignments: Vec::new(),
            pause: PausePolicy {
                pause_on_fullscreen: true,
                pause_on_battery: true,
                fps_limit: 30,
            },
            ipc: IpcTransport::UnixSocket {
                path: PathBuf::from("~/.config/backlayer/backlayer.sock"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        AssetMetadata, AssetSourceKind, AssignmentSettings, CompatibilityInfo, ImportMetadata,
        ImportSourceApp, MonitorAssignment, MonitorInfo, WallpaperKind,
    };

    #[test]
    fn monitor_assignment_matches_stable_id_and_legacy_output_name() {
        let monitor = MonitorInfo {
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
        };
        let wallpaper = AssetMetadata {
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
        };

        assert!(monitor.matches_assignment(&MonitorAssignment {
            monitor_id: "hypr:dell:u2720q:cn0xyz123456".into(),
            wallpaper: wallpaper.clone(),
            settings: AssignmentSettings::default(),
        }));
        assert!(monitor.matches_assignment(&MonitorAssignment {
            monitor_id: "DP-3".into(),
            wallpaper,
            settings: AssignmentSettings::default(),
        }));
    }

    #[test]
    fn imported_asset_metadata_round_trips() {
        let asset = AssetMetadata {
            id: "we.scene".into(),
            name: "Imported Scene".into(),
            kind: WallpaperKind::Scene,
            animated: true,
            image_fit: None,
            source_kind: AssetSourceKind::WallpaperEngineImport,
            preview_image: Some(PathBuf::from("preview.jpg")),
            compatibility: CompatibilityInfo {
                status: super::CompatibilityStatus::Partial,
                warnings: vec!["scene runtime is not implemented yet".into()],
            },
            import_metadata: Some(ImportMetadata {
                source_app: ImportSourceApp::WallpaperEngine,
                source_path: PathBuf::from("/tmp/workshop/123"),
                manifest_path: Some(PathBuf::from("/tmp/workshop/123/project.json")),
                workshop_id: Some("123".into()),
                original_type: Some("scene".into()),
            }),
            entrypoint: PathBuf::from("scene.pkg"),
        };

        let encoded = toml::to_string(&asset).expect("asset should serialize");
        let decoded: AssetMetadata = toml::from_str(&encoded).expect("asset should deserialize");

        assert_eq!(decoded, asset);
    }
}
