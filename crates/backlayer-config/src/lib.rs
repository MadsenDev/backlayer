use std::{
    env, fs,
    hash::{Hash, Hasher},
    io::{Seek, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use backlayer_types::{
    AssetMetadata, AssetSourceKind, AssignmentSettings, BacklayerConfig, CompatibilityInfo,
    CompatibilityStatus, CreateNativeAssetRequest, CreateSceneAssetRequest,
    CreateSceneImageSourceRequest, EditableSceneAsset, EditableSceneImage, ImportMetadata,
    ImportSourceApp, IpcTransport, MonitorAssignment, NativeSceneDocument, PausePolicy,
    SceneBehavior, SceneBlendMode, SceneColorStop, SceneCurvePoint, SceneEffectKind,
    SceneEffectNode, SceneEmitterNode, SceneEmitterPreset, SceneEmitterShape, SceneImageSource,
    SceneNode, SceneNormalizedPoint, SceneNormalizedRect, SceneParticleAreaNode,
    SceneParticleAreaShape, SceneSpriteNode, WallpaperKind,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use image::{DynamicImage, Rgba, RgbaImage, imageops};
use serde::Deserialize;
use thiserror::Error;
use zip::{ZipArchive, ZipWriter, write::FileOptions};

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config")]
    Read(#[from] std::io::Error),
    #[error("failed to parse config")]
    Parse(#[from] toml::de::Error),
    #[error("failed to parse json")]
    Json(#[from] serde_json::Error),
    #[error("failed to serialize config")]
    Serialize(#[from] toml::ser::Error),
    #[error("home directory is not available")]
    MissingHomeDirectory,
    #[error("path is not a supported Wallpaper Engine item or import root: {0}")]
    UnsupportedImportPath(String),
}

#[derive(Debug, Default, Clone)]
pub struct ConfigStore;

impl ConfigStore {
    pub fn workshop_enabled(&self) -> bool {
        env::var("BACKLAYER_ENABLE_WORKSHOP")
            .map(|value| {
                let normalized = value.trim().to_ascii_lowercase();
                !matches!(normalized.as_str(), "0" | "false" | "no" | "off")
            })
            .unwrap_or(false)
    }

    pub fn default_config_path(&self) -> PathBuf {
        PathBuf::from("~/.config/backlayer/config.toml")
    }

    pub fn default_socket_path(&self) -> PathBuf {
        PathBuf::from("~/.config/backlayer/backlayer.sock")
    }

    pub fn default_assets_path(&self) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .map(|path| path.join("assets"))
            .unwrap_or_else(|| PathBuf::from("assets"))
    }

    pub fn default_imports_path(&self) -> PathBuf {
        PathBuf::from("~/.config/backlayer/imports")
    }

    pub fn default_user_assets_path(&self) -> PathBuf {
        PathBuf::from("~/.config/backlayer/assets")
    }

    pub fn default_package_cache_path(&self) -> PathBuf {
        PathBuf::from("~/.config/backlayer/cache/packages")
    }

    pub fn default_workshop_imports_path(&self) -> PathBuf {
        self.default_imports_path().join("wallpaper-engine")
    }

    pub fn wallpaper_engine_workshop_search_paths(&self) -> Vec<PathBuf> {
        [
            "~/.local/share/Steam/steamapps/workshop/content/431960",
            "~/.steam/steam/steamapps/workshop/content/431960",
            "~/.var/app/com.valvesoftware.Steam/data/Steam/steamapps/workshop/content/431960",
            "~/.var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/workshop/content/431960",
        ]
        .into_iter()
        .filter_map(|candidate| self.resolve_path(candidate).ok())
        .collect()
    }

    pub fn discover_wallpaper_engine_workshop_paths(&self) -> Vec<PathBuf> {
        if !self.workshop_enabled() {
            return Vec::new();
        }

        self.wallpaper_engine_workshop_search_paths()
            .into_iter()
            .filter(|path| path.exists() && path.is_dir())
            .collect()
    }

    pub fn resolve_path(&self, path: impl AsRef<Path>) -> Result<PathBuf, ConfigError> {
        let path = path.as_ref();
        let display = path.to_string_lossy();

        if display == "~" {
            return env::var_os("HOME")
                .map(PathBuf::from)
                .ok_or(ConfigError::MissingHomeDirectory);
        }

        if let Some(stripped) = display.strip_prefix("~/") {
            return env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(stripped))
                .ok_or(ConfigError::MissingHomeDirectory);
        }

        Ok(path.to_path_buf())
    }

    pub fn load_from_str(&self, raw: &str) -> Result<BacklayerConfig, ConfigError> {
        toml::from_str(raw).map_err(ConfigError::from)
    }

    pub fn load_from_path(&self, path: impl AsRef<Path>) -> Result<BacklayerConfig, ConfigError> {
        let path = self.resolve_path(path)?;
        let raw = fs::read_to_string(path)?;
        self.load_from_str(&raw)
    }

    pub fn load_or_default(&self) -> BacklayerConfig {
        self.load_from_path(self.default_config_path())
            .unwrap_or_else(|_| BacklayerConfig::default())
    }

    pub fn default_asset_metadata(&self) -> AssetMetadata {
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

    pub fn sample_config(&self) -> BacklayerConfig {
        BacklayerConfig {
            assignments: vec![MonitorAssignment {
                monitor_id: "hypr:unknown:unknown:dp-1".into(),
                wallpaper: self.default_asset_metadata(),
                settings: AssignmentSettings::default(),
            }],
            pause: PausePolicy {
                pause_on_fullscreen: true,
                pause_on_battery: true,
                fps_limit: 30,
            },
            ipc: IpcTransport::UnixSocket {
                path: self.default_socket_path(),
            },
        }
    }

    pub fn serialize(&self, config: &BacklayerConfig) -> Result<String, ConfigError> {
        toml::to_string_pretty(config).map_err(ConfigError::from)
    }

    pub fn load_asset_metadata(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<AssetMetadata, ConfigError> {
        self.load_asset_metadata_from_manifest(path.as_ref(), None)
    }

    pub fn discover_assets(
        &self,
        root: impl AsRef<Path>,
    ) -> Result<Vec<AssetMetadata>, ConfigError> {
        let mut assets = Vec::new();
        let root = self.resolve_path(root)?;

        if !root.exists() {
            return Ok(assets);
        }

        for entry in fs::read_dir(root)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let metadata_path = entry.path().join("backlayer.toml");
                if metadata_path.is_file() {
                    assets.push(self.load_asset_metadata_from_manifest(
                        &metadata_path,
                        Some(entry.path()),
                    )?);
                }
                continue;
            }
            if entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("backlayer"))
            {
                assets.push(self.load_packaged_asset_metadata(entry.path())?);
            }
        }

        assets.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(assets)
    }

    pub fn discover_all_assets(&self) -> Result<Vec<AssetMetadata>, ConfigError> {
        let mut assets = self.discover_assets(self.default_assets_path())?;
        assets.extend(self.discover_assets(self.default_user_assets_path())?);
        if self.workshop_enabled() {
            assets.extend(self.discover_assets(self.default_workshop_imports_path())?);
        }
        assets.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(assets)
    }

    fn load_asset_metadata_from_manifest(
        &self,
        manifest_path: &Path,
        asset_path: Option<PathBuf>,
    ) -> Result<AssetMetadata, ConfigError> {
        let raw = fs::read_to_string(manifest_path)?;
        let mut asset: AssetMetadata = toml::from_str(&raw).map_err(ConfigError::from)?;
        let asset_root = manifest_path.parent().unwrap_or_else(|| Path::new("."));

        if asset.entrypoint.is_relative() {
            asset.entrypoint = asset_root.join(&asset.entrypoint);
        }

        if let Some(preview_image) = asset.preview_image.clone() {
            if preview_image.is_relative() {
                asset.preview_image = Some(asset_root.join(preview_image));
            }
        }

        asset.asset_path = asset_path;
        Ok(asset)
    }

    fn load_packaged_asset_metadata(
        &self,
        package_path: impl AsRef<Path>,
    ) -> Result<AssetMetadata, ConfigError> {
        let package_path = self.resolve_path(package_path)?;
        let extracted_root = self.extract_package_to_cache(&package_path)?;
        let mut asset = self.load_asset_metadata_from_manifest(
            &extracted_root.join("backlayer.toml"),
            Some(package_path),
        )?;
        asset.asset_path = Some(self.resolve_path(asset.asset_path.unwrap())?);
        Ok(asset)
    }

    fn extract_package_to_cache(&self, package_path: &Path) -> Result<PathBuf, ConfigError> {
        let cache_root = self.resolve_path(self.default_package_cache_path())?;
        fs::create_dir_all(&cache_root)?;
        let extract_root = cache_root.join(package_cache_key(package_path)?);
        let manifest_path = extract_root.join("backlayer.toml");
        if manifest_path.is_file() {
            return Ok(extract_root);
        }

        if extract_root.exists() {
            fs::remove_dir_all(&extract_root)?;
        }
        fs::create_dir_all(&extract_root)?;

        let file = fs::File::open(package_path)?;
        let mut archive =
            ZipArchive::new(file).map_err(|error| ConfigError::UnsupportedImportPath(error.to_string()))?;
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|error| ConfigError::UnsupportedImportPath(error.to_string()))?;
            let Some(relative_path) = entry.enclosed_name().map(Path::to_path_buf) else {
                continue;
            };
            let destination = extract_root.join(relative_path);
            if entry.is_dir() {
                fs::create_dir_all(&destination)?;
                continue;
            }
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut target = fs::File::create(&destination)?;
            std::io::copy(&mut entry, &mut target)?;
        }

        Ok(extract_root)
    }

    pub fn import_wallpaper_engine_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<Vec<AssetMetadata>, ConfigError> {
        if !self.workshop_enabled() {
            return Err(ConfigError::UnsupportedImportPath(
                "workshop compatibility is disabled by BACKLAYER_ENABLE_WORKSHOP".into(),
            ));
        }

        let path = self.resolve_path(path)?;
        if !path.exists() {
            return Err(ConfigError::Read(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("path does not exist: {}", path.display()),
            )));
        }

        let imports_root = self.resolve_path(self.default_workshop_imports_path())?;
        fs::create_dir_all(&imports_root)?;

        if let Some(item) = detect_workshop_item(&path)? {
            return Ok(vec![self.import_detected_item(item, &imports_root)?]);
        }

        if path.is_dir() {
            let mut imported = Vec::new();
            for entry in fs::read_dir(&path)? {
                let entry = entry?;
                if !entry.file_type()?.is_dir() {
                    continue;
                }
                if let Some(item) = detect_workshop_item(entry.path())? {
                    imported.push(self.import_detected_item(item, &imports_root)?);
                }
            }

            if !imported.is_empty() {
                imported.sort_by(|left, right| left.id.cmp(&right.id));
                return Ok(imported);
            }
        }

        Err(ConfigError::UnsupportedImportPath(
            path.display().to_string(),
        ))
    }

    pub fn save_to_path(
        &self,
        path: impl AsRef<Path>,
        config: &BacklayerConfig,
    ) -> Result<(), ConfigError> {
        let path = self.resolve_path(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(path, self.serialize(config)?)?;
        Ok(())
    }

    pub fn remove_managed_asset(&self, asset: &AssetMetadata) -> Result<(), ConfigError> {
        match asset.source_kind {
            AssetSourceKind::WallpaperEngineImport => self.remove_asset_from_root(
                asset,
                self.resolve_path(self.default_workshop_imports_path())?
                    .as_path(),
            ),
            AssetSourceKind::Native => self.remove_asset_from_root(
                asset,
                self.resolve_path(self.default_user_assets_path())?
                    .as_path(),
            ),
        }
    }

    pub fn create_native_scene_asset(
        &self,
        request: &CreateSceneAssetRequest,
        base_asset: Option<&AssetMetadata>,
    ) -> Result<AssetMetadata, ConfigError> {
        let (base_image, base_ext) = if let Some(base_asset) = base_asset {
            if base_asset.kind != WallpaperKind::Image {
                return Err(ConfigError::UnsupportedImportPath(format!(
                    "scene composer currently requires an image asset, got {}",
                    base_asset.id
                )));
            }

            let base_image = image::open(&base_asset.entrypoint).map_err(|error| {
                ConfigError::UnsupportedImportPath(format!(
                    "failed to load base image {}: {error}",
                    base_asset.entrypoint.display()
                ))
            })?;

            let base_ext = base_asset
                .entrypoint
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("png")
                .to_string();

            (base_image, base_ext)
        } else if let Some(path) = request.base_image_path.as_deref() {
            let base_image = image::open(path).map_err(|error| {
                ConfigError::UnsupportedImportPath(format!(
                    "failed to load base image {}: {error}",
                    path.display()
                ))
            })?;
            let base_ext = path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("png")
                .to_string();

            (base_image, base_ext)
        } else if let Some(data_url) = request.base_image_data_url.as_deref() {
            decode_data_url_image(data_url, request.base_image_filename.as_deref())?
        } else {
            return Err(ConfigError::UnsupportedImportPath(
                "scene composer requires a base asset, base image path, or base image data URL"
                    .into(),
            ));
        };

        let user_assets_root = self.resolve_path(self.default_user_assets_path())?;
        let scene_id = if let Some(existing_asset_id) = request.existing_asset_id.as_deref() {
            let existing_asset = self.load_editable_scene_asset(existing_asset_id)?.asset;
            if existing_asset.kind != WallpaperKind::Scene
                || existing_asset.source_kind != AssetSourceKind::Native
            {
                return Err(ConfigError::UnsupportedImportPath(format!(
                    "asset is not an editable native scene: {existing_asset_id}"
                )));
            }
            remove_path_if_exists(&user_assets_root.join(existing_asset_id))?;
            remove_path_if_exists(&user_assets_root.join(format!("{existing_asset_id}.backlayer")))?;
            existing_asset_id.to_string()
        } else {
            self.allocate_scene_id(&request.name)?
        };
        let package_path = user_assets_root.join(format!("{scene_id}.backlayer"));
        let scene_root = build_root_for(&scene_id, "scene");
        fs::create_dir_all(scene_root.join("images"))?;

        let base_target = scene_root.join("images").join(format!("base.{base_ext}"));
        base_image.save(&base_target).map_err(|error| {
            ConfigError::UnsupportedImportPath(format!(
                "failed to save scene base image {}: {error}",
                base_target.display()
            ))
        })?;

        let mut image_sources = vec![SceneImageSource {
            key: "base".into(),
            path: PathBuf::from(format!("images/base.{base_ext}")),
        }];
        for source in &request.extra_images {
            image_sources.push(self.write_extra_scene_image_source(&scene_root, source)?);
        }

        let nodes = sanitize_scene_nodes(&request.nodes, &image_sources);
        let scene_json = NativeSceneDocument {
            schema: "backlayer_scene_v2".into(),
            version: 2,
            width: base_image.width().max(1),
            height: base_image.height().max(1),
            images: image_sources,
            nodes: if nodes.is_empty() {
                vec![SceneNode::Sprite(SceneSpriteNode {
                    id: "sprite-base".into(),
                    name: "Base image".into(),
                    enabled: true,
                    image_key: "base".into(),
                    fit: Some(backlayer_types::ImageFitMode::Cover),
                    blend: Some(SceneBlendMode::Alpha),
                    x: 0.0,
                    y: 0.0,
                    scale: 1.0,
                    rotation_deg: 0.0,
                    opacity: 1.0,
                    particle_occluder: false,
                    particle_surface: false,
                    particle_region: None,
                    behaviors: Vec::new(),
                })]
            } else {
                nodes
            },
        };
        fs::write(
            scene_root.join("scene.json"),
            serde_json::to_vec_pretty(&scene_json)?,
        )?;

        let preview = compose_scene_preview(&base_image, &scene_json);
        let preview_path = scene_root.join("preview.png");
        preview.save(&preview_path).map_err(|error| {
            ConfigError::UnsupportedImportPath(format!(
                "failed to save scene preview {}: {error}",
                preview_path.display()
            ))
        })?;

        let metadata = AssetMetadata {
            id: scene_id.clone(),
            name: request.name.trim().to_string(),
            kind: WallpaperKind::Scene,
            animated: true,
            image_fit: None,
            source_kind: AssetSourceKind::Native,
            preview_image: Some(PathBuf::from("preview.png")),
            compatibility: CompatibilityInfo {
                status: CompatibilityStatus::Supported,
                warnings: Vec::new(),
            },
            import_metadata: None,
            entrypoint: PathBuf::from("scene.json"),
            asset_path: None,
        };

        fs::write(
            scene_root.join("backlayer.toml"),
            toml::to_string_pretty(&metadata)?,
        )?;

        write_package_from_directory(&scene_root, &package_path)?;
        let _ = fs::remove_dir_all(&scene_root);
        self.load_packaged_asset_metadata(package_path)
    }

    pub fn create_native_file_asset(
        &self,
        request: &CreateNativeAssetRequest,
    ) -> Result<AssetMetadata, ConfigError> {
        if matches!(request.kind, WallpaperKind::Scene | WallpaperKind::Web) {
            return Err(ConfigError::UnsupportedImportPath(format!(
                "native asset creation does not support {:?}",
                request.kind
            )));
        }

        let (bytes, extension) = decode_data_url_bytes(&request.data_url, Some(&request.filename))?;
        let asset_id = self.allocate_native_asset_id(&request.name, &request.kind)?;
        let user_assets_root = self.resolve_path(self.default_user_assets_path())?;
        let package_path = user_assets_root.join(format!("{asset_id}.backlayer"));
        let asset_root = build_root_for(&asset_id, "asset");
        let media_dir = asset_root.join("files");
        fs::create_dir_all(&media_dir)?;

        let entrypoint = media_dir.join(format!("source.{extension}"));
        fs::write(&entrypoint, bytes)?;

        let preview_image = match request.kind {
            WallpaperKind::Video => generate_video_preview(&entrypoint, &asset_root)
                .ok()
                .flatten(),
            WallpaperKind::Image => None,
            WallpaperKind::Shader => None,
            WallpaperKind::Scene | WallpaperKind::Web => None,
        };

        let metadata = AssetMetadata {
            id: asset_id.clone(),
            name: request.name.trim().to_string(),
            kind: request.kind.clone(),
            animated: matches!(request.kind, WallpaperKind::Video),
            image_fit: matches!(request.kind, WallpaperKind::Image)
                .then_some(backlayer_types::ImageFitMode::Cover),
            source_kind: AssetSourceKind::Native,
            preview_image,
            compatibility: CompatibilityInfo {
                status: CompatibilityStatus::Supported,
                warnings: Vec::new(),
            },
            import_metadata: None,
            entrypoint: relative_path(&asset_root, &entrypoint),
            asset_path: None,
        };

        fs::write(
            asset_root.join("backlayer.toml"),
            toml::to_string_pretty(&metadata)?,
        )?;

        write_package_from_directory(&asset_root, &package_path)?;
        let _ = fs::remove_dir_all(&asset_root);
        self.load_packaged_asset_metadata(package_path)
    }

    pub fn load_editable_scene_asset(
        &self,
        asset_id: &str,
    ) -> Result<EditableSceneAsset, ConfigError> {
        let asset_root = self
            .find_native_asset_container(asset_id)?
            .ok_or_else(|| {
                ConfigError::UnsupportedImportPath(format!(
                    "cannot edit missing native scene asset: {asset_id}"
                ))
            })?;
        let metadata = if asset_root
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("backlayer"))
        {
            self.load_packaged_asset_metadata(&asset_root)?
        } else {
            self.load_asset_metadata_from_manifest(&asset_root.join("backlayer.toml"), Some(asset_root.clone()))?
        };
        if metadata.kind != WallpaperKind::Scene || metadata.source_kind != AssetSourceKind::Native
        {
            return Err(ConfigError::UnsupportedImportPath(format!(
                "asset is not an editable native scene: {asset_id}"
            )));
        }

        let extracted_root = if asset_root
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("backlayer"))
        {
            self.extract_package_to_cache(&asset_root)?
        } else {
            asset_root.clone()
        };
        let scene_path = extracted_root.join(
            metadata
                .entrypoint
                .strip_prefix(&extracted_root)
                .unwrap_or(&metadata.entrypoint),
        );
        let document: NativeSceneDocument =
            serde_json::from_slice(&fs::read(&scene_path).map_err(ConfigError::Read)?)?;
        if document.schema != "backlayer_scene_v2" {
            return Err(ConfigError::UnsupportedImportPath(format!(
                "scene asset does not use backlayer_scene_v2: {asset_id}"
            )));
        }

        let images = document
            .images
            .iter()
            .map(|image| {
                let path = extracted_root.join(&image.path);
                let dimensions = image::image_dimensions(&path).ok();
                Ok(EditableSceneImage {
                    key: image.key.clone(),
                    filename: path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("image.png")
                        .to_string(),
                    path,
                    width: dimensions.map(|(width, _)| width),
                    height: dimensions.map(|(_, height)| height),
                })
            })
            .collect::<Result<Vec<_>, ConfigError>>()?;

        Ok(EditableSceneAsset {
            asset: metadata,
            document,
            images,
        })
    }

    fn write_extra_scene_image_source(
        &self,
        scene_root: &Path,
        source: &CreateSceneImageSourceRequest,
    ) -> Result<SceneImageSource, ConfigError> {
        let key = sanitize_component(&source.key);
        let target = if let Some(path) = source.existing_path.as_deref() {
            self.copy_existing_scene_image_source(scene_root, &key, path)?
        } else if let Some(data_url) = source.data_url.as_deref() {
            let (image, extension) = decode_data_url_image(data_url, Some(&source.filename))?;
            let file_name = format!("{key}.{extension}");
            let target = scene_root.join("images").join(&file_name);
            image.save(&target).map_err(|error| {
                ConfigError::UnsupportedImportPath(format!(
                    "failed to save scene source image {}: {error}",
                    target.display()
                ))
            })?;
            target
        } else {
            return Err(ConfigError::UnsupportedImportPath(format!(
                "scene image source {key} is missing both data_url and existing_path"
            )));
        };

        Ok(SceneImageSource {
            key,
            path: relative_path(scene_root, &target),
        })
    }

    fn copy_existing_scene_image_source(
        &self,
        scene_root: &Path,
        key: &str,
        path: &Path,
    ) -> Result<PathBuf, ConfigError> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("png");
        let file_name = format!("{key}.{extension}");
        let target = scene_root.join("images").join(&file_name);
        fs::copy(path, &target).map_err(|error| {
            ConfigError::UnsupportedImportPath(format!(
                "failed to copy scene source image {}: {error}",
                path.display()
            ))
        })?;
        Ok(target)
    }

    fn allocate_scene_id(&self, name: &str) -> Result<String, ConfigError> {
        let root = self.resolve_path(self.default_user_assets_path())?;
        fs::create_dir_all(&root)?;
        let base = slugify_name(name);
        let prefix = format!("scene.{base}");

        if !root.join(&prefix).exists() && !root.join(format!("{prefix}.backlayer")).exists() {
            return Ok(prefix);
        }

        for index in 2..=999 {
            let candidate = format!("{prefix}-{index}");
            if !root.join(&candidate).exists()
                && !root.join(format!("{candidate}.backlayer")).exists()
            {
                return Ok(candidate);
            }
        }

        Err(ConfigError::UnsupportedImportPath(format!(
            "could not allocate scene id for {name}"
        )))
    }

    fn allocate_native_asset_id(
        &self,
        name: &str,
        kind: &WallpaperKind,
    ) -> Result<String, ConfigError> {
        let root = self.resolve_path(self.default_user_assets_path())?;
        fs::create_dir_all(&root)?;
        let base = slugify_name(name);
        let prefix = format!(
            "{}.{}",
            match kind {
                WallpaperKind::Image => "image",
                WallpaperKind::Video => "video",
                WallpaperKind::Shader => "shader",
                WallpaperKind::Scene => "scene",
                WallpaperKind::Web => "web",
            },
            base
        );

        if !root.join(&prefix).exists() {
            return Ok(prefix);
        }

        for index in 2..=999 {
            let candidate = format!("{prefix}-{index}");
            if !root.join(&candidate).exists() {
                return Ok(candidate);
            }
        }

        Err(ConfigError::UnsupportedImportPath(format!(
            "could not allocate asset id for {name}"
        )))
    }

    fn import_detected_item(
        &self,
        item: DetectedWorkshopItem,
        imports_root: &Path,
    ) -> Result<AssetMetadata, ConfigError> {
        let import_id = import_asset_id(&item);
        let destination = imports_root.join(&import_id);
        if destination.exists() {
            fs::remove_dir_all(&destination)?;
        }
        copy_dir_recursive(&item.root, &destination)?;

        let entrypoint = destination.join(&item.entrypoint_rel);
        let preview_image = item
            .preview_rel
            .as_ref()
            .map(|relative| destination.join(relative));
        let metadata = AssetMetadata {
            id: import_id.clone(),
            name: item.title.clone(),
            kind: item.kind.clone(),
            animated: matches!(item.kind, WallpaperKind::Scene | WallpaperKind::Web),
            image_fit: None,
            source_kind: AssetSourceKind::WallpaperEngineImport,
            preview_image,
            compatibility: CompatibilityInfo {
                status: item.compatibility.clone(),
                warnings: item.warnings.clone(),
            },
            import_metadata: Some(ImportMetadata {
                source_app: ImportSourceApp::WallpaperEngine,
                source_path: item.root.clone(),
                manifest_path: item.manifest_path.clone(),
                workshop_id: item.workshop_id.clone(),
                original_type: item.original_type.clone(),
            }),
            entrypoint,
            asset_path: Some(destination.clone()),
        };

        let metadata_path = destination.join("backlayer.toml");
        fs::write(metadata_path, toml::to_string_pretty(&metadata)?)?;
        Ok(metadata)
    }

    fn remove_asset_from_root(
        &self,
        asset: &AssetMetadata,
        managed_root: &Path,
    ) -> Result<(), ConfigError> {
        if let Some(asset_path) = asset.asset_path.as_ref() {
            let resolved = self.resolve_path(asset_path)?;
            if resolved.starts_with(managed_root) {
                remove_path_if_exists(&resolved)?;
                return Ok(());
            }
        }

        let candidate = managed_root.join(&asset.id);
        if candidate.exists() {
            fs::remove_dir_all(candidate)?;
            return Ok(());
        }
        let packaged = managed_root.join(format!("{}.backlayer", asset.id));
        if packaged.exists() {
            fs::remove_file(packaged)?;
            return Ok(());
        }

        let entrypoint = self.resolve_path(&asset.entrypoint)?;
        if !entrypoint.starts_with(managed_root) {
            return Err(ConfigError::UnsupportedImportPath(asset.id.clone()));
        }

        let mut current = entrypoint.parent().map(Path::to_path_buf);
        while let Some(path) = current {
            if path.parent() == Some(managed_root) {
                fs::remove_dir_all(&path)?;
                return Ok(());
            }
            if path == managed_root {
                break;
            }
            current = path.parent().map(Path::to_path_buf);
        }

        Err(ConfigError::UnsupportedImportPath(asset.id.clone()))
    }

    fn find_native_asset_container(&self, asset_id: &str) -> Result<Option<PathBuf>, ConfigError> {
        let root = self.resolve_path(self.default_user_assets_path())?;
        let packaged = root.join(format!("{asset_id}.backlayer"));
        if packaged.is_file() {
            return Ok(Some(packaged));
        }

        let directory = root.join(asset_id);
        if directory.join("backlayer.toml").is_file() {
            return Ok(Some(directory));
        }

        Ok(None)
    }
}

fn build_root_for(asset_id: &str, prefix: &str) -> PathBuf {
    let unique = format!(
        "{}-{}-{}",
        prefix,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    std::env::temp_dir().join("backlayer-build").join(unique).join(asset_id)
}

fn remove_path_if_exists(path: &Path) -> Result<(), ConfigError> {
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else if path.is_file() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn package_cache_key(package_path: &Path) -> Result<String, ConfigError> {
    let metadata = fs::metadata(package_path)?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    package_path.to_string_lossy().hash(&mut hasher);
    metadata.len().hash(&mut hasher);
    modified.hash(&mut hasher);
    Ok(format!("{:016x}", hasher.finish()))
}

fn write_package_from_directory(root: &Path, package_path: &Path) -> Result<(), ConfigError> {
    if let Some(parent) = package_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = fs::File::create(package_path)?;
    let mut archive = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    write_directory_to_zip(root, root, &mut archive, options)
        .map_err(|error| ConfigError::UnsupportedImportPath(error.to_string()))?;
    archive
        .finish()
        .map_err(|error| ConfigError::UnsupportedImportPath(error.to_string()))?;
    Ok(())
}

fn write_directory_to_zip<W: Write + Seek>(
    root: &Path,
    current: &Path,
    archive: &mut ZipWriter<W>,
    options: FileOptions,
) -> zip::result::ZipResult<()> {
    for entry in fs::read_dir(current)
        .map_err(zip::result::ZipError::Io)?
    {
        let entry = entry.map_err(zip::result::ZipError::Io)?;
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .expect("path should stay under root")
            .to_string_lossy()
            .replace('\\', "/");

        if entry.file_type().map_err(zip::result::ZipError::Io)?.is_dir() {
            archive.add_directory(format!("{relative}/"), options)?;
            write_directory_to_zip(root, &path, archive, options)?;
        } else {
            archive.start_file(relative, options)?;
            let mut file = fs::File::open(&path).map_err(zip::result::ZipError::Io)?;
            std::io::copy(&mut file, archive).map_err(zip::result::ZipError::Io)?;
        }
    }

    Ok(())
}

fn sanitize_scene_nodes(nodes: &[SceneNode], images: &[SceneImageSource]) -> Vec<SceneNode> {
    let mut sanitized = Vec::new();
    let valid_keys = images
        .iter()
        .map(|source| source.key.as_str())
        .collect::<Vec<_>>();

    for node in nodes {
        match node {
            SceneNode::Sprite(sprite) => {
                let image_key = if sprite.image_key.trim().is_empty()
                    || !valid_keys.iter().any(|key| *key == sprite.image_key.trim())
                {
                    "base".into()
                } else {
                    sprite.image_key.trim().to_string()
                };
                sanitized.push(SceneNode::Sprite(SceneSpriteNode {
                    id: if sprite.id.trim().is_empty() {
                        format!("sprite-{}", sanitized.len() + 1)
                    } else {
                        sprite.id.trim().to_string()
                    },
                    name: if sprite.name.trim().is_empty() {
                        "Sprite".into()
                    } else {
                        sprite.name.trim().to_string()
                    },
                    enabled: sprite.enabled,
                    image_key: image_key.clone(),
                    fit: if image_key == "base" {
                        Some(backlayer_types::ImageFitMode::Cover)
                    } else {
                        sprite.fit.clone()
                    },
                    blend: sprite.blend.clone(),
                    x: sprite.x,
                    y: sprite.y,
                    scale: sprite.scale.clamp(0.05, 4.0),
                    rotation_deg: sprite.rotation_deg.clamp(-180.0, 180.0),
                    opacity: sprite.opacity.clamp(0.0, 1.0),
                    particle_occluder: sprite.particle_occluder,
                    particle_surface: sprite.particle_surface,
                    particle_region: sanitize_scene_rect(sprite.particle_region.as_ref()),
                    behaviors: sanitize_behaviors(&sprite.behaviors),
                }))
            }
            SceneNode::Effect(effect) => sanitized.push(SceneNode::Effect(SceneEffectNode {
                id: if effect.id.trim().is_empty() {
                    format!("effect-{}", sanitized.len() + 1)
                } else {
                    effect.id.trim().to_string()
                },
                name: if effect.name.trim().is_empty() {
                    "Effect".into()
                } else {
                    effect.name.trim().to_string()
                },
                enabled: effect.enabled,
                effect: effect.effect.clone(),
                color_hex: Some(sanitize_effect_color_hex(
                    effect.color_hex.as_deref(),
                    &effect.effect,
                )),
                opacity: effect.opacity.clamp(0.0, 1.0),
                intensity: effect.intensity.clamp(0.0, 2.5),
                speed: effect.speed.clamp(0.0, 4.0),
            })),
            SceneNode::Emitter(emitter) => sanitized.push(SceneNode::Emitter(SceneEmitterNode {
                id: if emitter.id.trim().is_empty() {
                    format!("emitter-{}", sanitized.len() + 1)
                } else {
                    emitter.id.trim().to_string()
                },
                name: if emitter.name.trim().is_empty() {
                    "Emitter".into()
                } else {
                    emitter.name.trim().to_string()
                },
                enabled: emitter.enabled,
                preset: emitter.preset.clone(),
                shape: Some(
                    emitter
                        .shape
                        .clone()
                        .unwrap_or_else(|| default_emitter_shape(&emitter.preset)),
                ),
                origin_x: Some(
                    emitter
                        .origin_x
                        .unwrap_or(default_emitter_origin_x(&emitter.preset))
                        .clamp(0.0, 1.0),
                ),
                origin_y: Some(
                    emitter
                        .origin_y
                        .unwrap_or(default_emitter_origin_y(&emitter.preset))
                        .clamp(0.0, 1.0),
                ),
                direction_deg: Some(normalize_degrees(
                    emitter
                        .direction_deg
                        .unwrap_or(default_emitter_direction_deg(&emitter.preset)),
                )),
                region_width: Some(
                    emitter
                        .region_width
                        .unwrap_or(default_emitter_region_width(&emitter.preset))
                        .clamp(0.0, 1.0),
                ),
                region_height: Some(
                    emitter
                        .region_height
                        .unwrap_or(default_emitter_region_height(&emitter.preset))
                        .clamp(0.0, 1.0),
                ),
                region_radius: Some(
                    emitter
                        .region_radius
                        .unwrap_or(default_emitter_region_radius(&emitter.preset))
                        .clamp(0.01, 1.0),
                ),
                line_length: Some(
                    emitter
                        .line_length
                        .unwrap_or(default_emitter_line_length(&emitter.preset))
                        .clamp(0.01, 1.0),
                ),
                line_angle_deg: Some(normalize_degrees(
                    emitter
                        .line_angle_deg
                        .unwrap_or(default_emitter_line_angle_deg(&emitter.preset)),
                )),
                emission_rate: emitter.emission_rate.clamp(1.0, 160.0),
                burst_count: emitter.burst_count.clamp(0, 512),
                burst_on_start: emitter.burst_on_start,
                max_particles: emitter.max_particles.clamp(8, 1024),
                opacity: emitter.opacity.clamp(0.0, 1.0),
                size: emitter.size.clamp(1.0, 24.0),
                speed: emitter.speed.clamp(1.0, 1200.0),
                min_speed: Some(
                    emitter
                        .min_speed
                        .unwrap_or(default_emitter_min_speed(&emitter.preset))
                        .clamp(0.0, 1200.0),
                ),
                max_speed: Some(
                    emitter
                        .max_speed
                        .unwrap_or(default_emitter_max_speed(&emitter.preset))
                        .clamp(0.0, 1400.0),
                ),
                min_life: Some(
                    emitter
                        .min_life
                        .unwrap_or(default_emitter_min_life(&emitter.preset))
                        .clamp(0.2, 20.0),
                ),
                max_life: Some(
                    emitter
                        .max_life
                        .unwrap_or(default_emitter_max_life(&emitter.preset))
                        .clamp(0.2, 24.0),
                ),
                spread: emitter.spread.clamp(0.0, 180.0),
                gravity_x: emitter.gravity_x.clamp(-400.0, 400.0),
                gravity_y: emitter.gravity_y.clamp(-400.0, 400.0),
                drag: emitter.drag.clamp(0.0, 8.0),
                color_hex: Some(sanitize_emitter_color_hex(
                    emitter.color_hex.as_deref(),
                    &emitter.preset,
                )),
                particle_image_key: sanitize_scene_image_key(
                    emitter.particle_image_key.as_deref(),
                    &valid_keys,
                ),
                particle_rotation_deg: Some(normalize_degrees(
                    emitter.particle_rotation_deg.unwrap_or(0.0),
                )),
                size_curve: sanitize_scalar_curve(
                    &emitter.size_curve,
                    &default_emitter_size_curve(&emitter.preset),
                ),
                alpha_curve: sanitize_scalar_curve(
                    &emitter.alpha_curve,
                    &default_emitter_alpha_curve(&emitter.preset),
                ),
                color_curve: sanitize_color_curve(
                    &emitter.color_curve,
                    &default_emitter_color_curve(&emitter.preset),
                ),
            })),
            SceneNode::ParticleArea(area) => {
                let region = sanitize_scene_rect(Some(&area.region)).unwrap_or(SceneNormalizedRect {
                    x: 0.25,
                    y: 0.25,
                    width: 0.5,
                    height: 0.25,
                });
                sanitized.push(SceneNode::ParticleArea(SceneParticleAreaNode {
                    id: if area.id.trim().is_empty() {
                        format!("particle-area-{}", sanitized.len() + 1)
                    } else {
                        area.id.trim().to_string()
                    },
                    name: if area.name.trim().is_empty() {
                        "Particle area".into()
                    } else {
                        area.name.trim().to_string()
                    },
                    enabled: area.enabled,
                    shape: Some(
                        area.shape
                            .clone()
                            .unwrap_or(SceneParticleAreaShape::Rect),
                    ),
                    region,
                    points: sanitize_scene_points(&area.points),
                    occluder: area.occluder,
                    surface: area.surface,
                }))
            }
        }
    }

    if !sanitized
        .iter()
        .any(|node| matches!(node, SceneNode::Sprite(sprite) if sprite.image_key == "base"))
    {
        sanitized.insert(
            0,
            SceneNode::Sprite(SceneSpriteNode {
                id: "sprite-base".into(),
                name: "Base image".into(),
                enabled: true,
                image_key: "base".into(),
                fit: Some(backlayer_types::ImageFitMode::Cover),
                blend: Some(SceneBlendMode::Alpha),
                x: 0.0,
                y: 0.0,
                scale: 1.0,
                rotation_deg: 0.0,
                opacity: 1.0,
                particle_occluder: false,
                particle_surface: false,
                particle_region: None,
                behaviors: Vec::new(),
            }),
        );
    }

    sanitized
}

fn sanitize_behaviors(behaviors: &[SceneBehavior]) -> Vec<SceneBehavior> {
    behaviors
        .iter()
        .map(|behavior| SceneBehavior {
            kind: behavior.kind.clone(),
            speed: behavior.speed.clamp(0.0, 8.0),
            amount_x: behavior.amount_x.clamp(-800.0, 800.0),
            amount_y: behavior.amount_y.clamp(-800.0, 800.0),
            amount: behavior.amount.clamp(-800.0, 800.0),
            phase: behavior.phase,
        })
        .collect()
}

fn sanitize_scene_rect(rect: Option<&SceneNormalizedRect>) -> Option<SceneNormalizedRect> {
    rect.map(|rect| {
        let x = rect.x.clamp(0.0, 1.0);
        let y = rect.y.clamp(0.0, 1.0);
        let width = rect.width.clamp(0.01, 1.0 - x);
        let height = rect.height.clamp(0.01, 1.0 - y);
        SceneNormalizedRect {
            x,
            y,
            width,
            height,
        }
    })
}

fn sanitize_scene_points(points: &[SceneNormalizedPoint]) -> Vec<SceneNormalizedPoint> {
    points
        .iter()
        .map(|point| SceneNormalizedPoint {
            x: point.x.clamp(0.0, 1.0),
            y: point.y.clamp(0.0, 1.0),
        })
        .collect()
}

fn default_emitter_origin_x(preset: &SceneEmitterPreset) -> f32 {
    match preset {
        SceneEmitterPreset::Rain => 0.55,
        SceneEmitterPreset::Snow => 0.5,
        SceneEmitterPreset::Dust => 0.5,
        SceneEmitterPreset::Embers => 0.5,
    }
}

fn default_emitter_shape(preset: &SceneEmitterPreset) -> SceneEmitterShape {
    match preset {
        SceneEmitterPreset::Rain => SceneEmitterShape::Line,
        SceneEmitterPreset::Snow => SceneEmitterShape::Box,
        SceneEmitterPreset::Dust => SceneEmitterShape::Box,
        SceneEmitterPreset::Embers => SceneEmitterShape::Circle,
    }
}

fn default_emitter_region_width(preset: &SceneEmitterPreset) -> f32 {
    match preset {
        SceneEmitterPreset::Rain => 0.42,
        SceneEmitterPreset::Snow => 0.28,
        SceneEmitterPreset::Dust => 0.16,
        SceneEmitterPreset::Embers => 0.14,
    }
}

fn default_emitter_region_height(preset: &SceneEmitterPreset) -> f32 {
    match preset {
        SceneEmitterPreset::Rain => 0.08,
        SceneEmitterPreset::Snow => 0.06,
        SceneEmitterPreset::Dust => 0.12,
        SceneEmitterPreset::Embers => 0.08,
    }
}

fn default_emitter_region_radius(preset: &SceneEmitterPreset) -> f32 {
    match preset {
        SceneEmitterPreset::Rain => 0.12,
        SceneEmitterPreset::Snow => 0.18,
        SceneEmitterPreset::Dust => 0.1,
        SceneEmitterPreset::Embers => 0.09,
    }
}

fn default_emitter_line_length(preset: &SceneEmitterPreset) -> f32 {
    match preset {
        SceneEmitterPreset::Rain => 0.42,
        SceneEmitterPreset::Snow => 0.2,
        SceneEmitterPreset::Dust => 0.16,
        SceneEmitterPreset::Embers => 0.12,
    }
}

fn default_emitter_line_angle_deg(preset: &SceneEmitterPreset) -> f32 {
    default_emitter_direction_deg(preset)
}

fn default_effect_color_hex(effect: &SceneEffectKind) -> &'static str {
    match effect {
        SceneEffectKind::Glow => "#ffc785",
        SceneEffectKind::Vignette => "#070d14",
        SceneEffectKind::Scanlines => "#ffd69b",
        SceneEffectKind::Fog => "#dbe8ff",
    }
}

fn default_emitter_origin_y(preset: &SceneEmitterPreset) -> f32 {
    match preset {
        SceneEmitterPreset::Rain => 0.0,
        SceneEmitterPreset::Snow => 0.0,
        SceneEmitterPreset::Dust => 0.56,
        SceneEmitterPreset::Embers => 1.0,
    }
}

fn default_emitter_direction_deg(preset: &SceneEmitterPreset) -> f32 {
    match preset {
        SceneEmitterPreset::Embers => -90.0,
        SceneEmitterPreset::Rain => 100.0,
        SceneEmitterPreset::Dust => -26.0,
        SceneEmitterPreset::Snow => 92.0,
    }
}

fn default_emitter_color_hex(preset: &SceneEmitterPreset) -> &'static str {
    match preset {
        SceneEmitterPreset::Embers => "#ff9452",
        SceneEmitterPreset::Rain => "#bedcff",
        SceneEmitterPreset::Dust => "#e0ecff",
        SceneEmitterPreset::Snow => "#f4f7ff",
    }
}

fn default_emitter_min_speed(preset: &SceneEmitterPreset) -> f32 {
    match preset {
        SceneEmitterPreset::Embers => 48.0,
        SceneEmitterPreset::Rain => 320.0,
        SceneEmitterPreset::Dust => 14.0,
        SceneEmitterPreset::Snow => 20.0,
    }
}

fn default_emitter_max_speed(preset: &SceneEmitterPreset) -> f32 {
    match preset {
        SceneEmitterPreset::Embers => 110.0,
        SceneEmitterPreset::Rain => 620.0,
        SceneEmitterPreset::Dust => 42.0,
        SceneEmitterPreset::Snow => 58.0,
    }
}

fn default_emitter_min_life(preset: &SceneEmitterPreset) -> f32 {
    match preset {
        SceneEmitterPreset::Embers => 2.8,
        SceneEmitterPreset::Rain => 1.7,
        SceneEmitterPreset::Dust => 4.5,
        SceneEmitterPreset::Snow => 6.0,
    }
}

fn default_emitter_max_life(preset: &SceneEmitterPreset) -> f32 {
    match preset {
        SceneEmitterPreset::Embers => 5.0,
        SceneEmitterPreset::Rain => 2.5,
        SceneEmitterPreset::Dust => 9.0,
        SceneEmitterPreset::Snow => 9.0,
    }
}

fn default_emitter_size_curve(preset: &SceneEmitterPreset) -> Vec<SceneCurvePoint> {
    match preset {
        SceneEmitterPreset::Rain => vec![
            SceneCurvePoint { x: 0.0, y: 0.7 },
            SceneCurvePoint { x: 1.0, y: 1.0 },
        ],
        SceneEmitterPreset::Snow => vec![
            SceneCurvePoint { x: 0.0, y: 0.8 },
            SceneCurvePoint { x: 0.5, y: 1.0 },
            SceneCurvePoint { x: 1.0, y: 0.85 },
        ],
        SceneEmitterPreset::Dust => vec![
            SceneCurvePoint { x: 0.0, y: 0.55 },
            SceneCurvePoint { x: 0.5, y: 1.0 },
            SceneCurvePoint { x: 1.0, y: 1.2 },
        ],
        SceneEmitterPreset::Embers => vec![
            SceneCurvePoint { x: 0.0, y: 0.7 },
            SceneCurvePoint { x: 0.55, y: 1.0 },
            SceneCurvePoint { x: 1.0, y: 0.35 },
        ],
    }
}

fn default_emitter_alpha_curve(preset: &SceneEmitterPreset) -> Vec<SceneCurvePoint> {
    match preset {
        SceneEmitterPreset::Rain => vec![
            SceneCurvePoint { x: 0.0, y: 0.9 },
            SceneCurvePoint { x: 1.0, y: 0.3 },
        ],
        SceneEmitterPreset::Snow => vec![
            SceneCurvePoint { x: 0.0, y: 0.25 },
            SceneCurvePoint { x: 0.18, y: 0.7 },
            SceneCurvePoint { x: 1.0, y: 0.1 },
        ],
        SceneEmitterPreset::Dust => vec![
            SceneCurvePoint { x: 0.0, y: 0.1 },
            SceneCurvePoint { x: 0.35, y: 0.55 },
            SceneCurvePoint { x: 1.0, y: 0.0 },
        ],
        SceneEmitterPreset::Embers => vec![
            SceneCurvePoint { x: 0.0, y: 0.25 },
            SceneCurvePoint { x: 0.2, y: 1.0 },
            SceneCurvePoint { x: 1.0, y: 0.0 },
        ],
    }
}

fn default_emitter_color_curve(preset: &SceneEmitterPreset) -> Vec<SceneColorStop> {
    match preset {
        SceneEmitterPreset::Rain => vec![
            SceneColorStop {
                x: 0.0,
                color_hex: "#e1f1ff".into(),
            },
            SceneColorStop {
                x: 1.0,
                color_hex: "#7bb7ff".into(),
            },
        ],
        SceneEmitterPreset::Snow => vec![
            SceneColorStop {
                x: 0.0,
                color_hex: "#ffffff".into(),
            },
            SceneColorStop {
                x: 1.0,
                color_hex: "#dbe8ff".into(),
            },
        ],
        SceneEmitterPreset::Dust => vec![
            SceneColorStop {
                x: 0.0,
                color_hex: "#fff1d9".into(),
            },
            SceneColorStop {
                x: 1.0,
                color_hex: "#d5b98e".into(),
            },
        ],
        SceneEmitterPreset::Embers => vec![
            SceneColorStop {
                x: 0.0,
                color_hex: "#fff1af".into(),
            },
            SceneColorStop {
                x: 0.55,
                color_hex: "#ff8b4a".into(),
            },
            SceneColorStop {
                x: 1.0,
                color_hex: "#72250b".into(),
            },
        ],
    }
}

fn normalize_degrees(value: f32) -> f32 {
    let mut normalized = value % 360.0;
    if normalized > 180.0 {
        normalized -= 360.0;
    }
    if normalized < -180.0 {
        normalized += 360.0;
    }
    normalized
}

fn sanitize_emitter_color_hex(value: Option<&str>, preset: &SceneEmitterPreset) -> String {
    let fallback = default_emitter_color_hex(preset).to_string();
    let Some(value) = value else {
        return fallback;
    };
    let value = value.trim();
    let normalized = if value.starts_with('#') {
        value.to_string()
    } else {
        format!("#{value}")
    };
    if normalized.len() != 7
        || !normalized.as_bytes()[1..]
            .iter()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return fallback;
    }
    normalized.to_lowercase()
}

fn sanitize_effect_color_hex(value: Option<&str>, effect: &SceneEffectKind) -> String {
    let fallback = default_effect_color_hex(effect).to_string();
    let Some(value) = value else {
        return fallback;
    };
    let value = value.trim();
    let normalized = if value.starts_with('#') {
        value.to_string()
    } else {
        format!("#{value}")
    };
    if normalized.len() != 7
        || !normalized.as_bytes()[1..]
            .iter()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return fallback;
    }
    normalized.to_lowercase()
}

fn sanitize_scene_image_key(value: Option<&str>, valid_keys: &[&str]) -> Option<String> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }
    valid_keys
        .iter()
        .find(|key| **key == value)
        .map(|key| (*key).to_string())
}

fn sanitize_scalar_curve(
    points: &[SceneCurvePoint],
    fallback: &[SceneCurvePoint],
) -> Vec<SceneCurvePoint> {
    let mut points = if points.is_empty() {
        fallback.to_vec()
    } else {
        points
            .iter()
            .map(|point| SceneCurvePoint {
                x: point.x.clamp(0.0, 1.0),
                y: point.y.clamp(0.0, 2.5),
            })
            .collect::<Vec<_>>()
    };
    points.sort_by(|left, right| {
        left.x
            .partial_cmp(&right.x)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    points.dedup_by(|left, right| (left.x - right.x).abs() < 0.001);
    if points.is_empty() {
        return fallback.to_vec();
    }
    if points.first().map(|point| point.x > 0.0).unwrap_or(true) {
        let first_y = points.first().map(|point| point.y).unwrap_or(1.0);
        points.insert(0, SceneCurvePoint { x: 0.0, y: first_y });
    }
    if points.last().map(|point| point.x < 1.0).unwrap_or(true) {
        let last_y = points.last().map(|point| point.y).unwrap_or(1.0);
        points.push(SceneCurvePoint { x: 1.0, y: last_y });
    }
    points
}

fn sanitize_color_curve(
    points: &[SceneColorStop],
    fallback: &[SceneColorStop],
) -> Vec<SceneColorStop> {
    let mut points = if points.is_empty() {
        fallback.to_vec()
    } else {
        points
            .iter()
            .map(|point| SceneColorStop {
                x: point.x.clamp(0.0, 1.0),
                color_hex: sanitize_effect_color_hex(
                    Some(point.color_hex.as_str()),
                    &SceneEffectKind::Glow,
                ),
            })
            .collect::<Vec<_>>()
    };
    points.sort_by(|left, right| {
        left.x
            .partial_cmp(&right.x)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    points.dedup_by(|left, right| (left.x - right.x).abs() < 0.001);
    if points.is_empty() {
        return fallback.to_vec();
    }
    if points.first().map(|point| point.x > 0.0).unwrap_or(true) {
        let first = points.first().cloned().unwrap();
        points.insert(
            0,
            SceneColorStop {
                x: 0.0,
                color_hex: first.color_hex,
            },
        );
    }
    if points.last().map(|point| point.x < 1.0).unwrap_or(true) {
        let last = points.last().cloned().unwrap();
        points.push(SceneColorStop {
            x: 1.0,
            color_hex: last.color_hex,
        });
    }
    points
}

fn parse_color_components(value: &str) -> [f32; 3] {
    let value = value.strip_prefix('#').unwrap_or(value);
    let parse = |range: std::ops::Range<usize>| -> f32 {
        u8::from_str_radix(&value[range], 16).unwrap_or(255) as f32 / 255.0
    };
    [parse(0..2), parse(2..4), parse(4..6)]
}

fn compose_scene_preview(base_image: &DynamicImage, scene: &NativeSceneDocument) -> RgbaImage {
    let mut canvas = RgbaImage::from_pixel(
        scene.width.max(1),
        scene.height.max(1),
        Rgba([0, 0, 0, 255]),
    );
    let base_rgba = base_image.to_rgba8();

    for node in &scene.nodes {
        match node {
            SceneNode::Sprite(sprite) if sprite.enabled && sprite.image_key == "base" => {
                overlay_sprite_preview(&mut canvas, &base_rgba, sprite);
            }
            SceneNode::Effect(effect) if effect.enabled => {
                overlay_effect_preview(&mut canvas, effect);
            }
            SceneNode::Emitter(emitter) if emitter.enabled => {
                overlay_emitter_preview(&mut canvas, emitter, &scene.schema, scene.nodes.len());
            }
            _ => {}
        }
    }

    canvas
}

fn overlay_sprite_preview(canvas: &mut RgbaImage, source: &RgbaImage, sprite: &SceneSpriteNode) {
    let fit = sprite
        .fit
        .clone()
        .unwrap_or(backlayer_types::ImageFitMode::Cover);
    let (target_width, target_height, x, y) = preview_sprite_layout(
        canvas.width(),
        canvas.height(),
        source.width(),
        source.height(),
        sprite,
        &fit,
    );
    let mut layer = if target_width != source.width() || target_height != source.height() {
        imageops::resize(
            source,
            target_width.max(1),
            target_height.max(1),
            imageops::FilterType::Triangle,
        )
    } else {
        source.clone()
    };
    apply_layer_opacity(&mut layer, sprite.opacity);
    imageops::overlay(canvas, &layer, x, y);
}

fn preview_sprite_layout(
    canvas_width: u32,
    canvas_height: u32,
    source_width: u32,
    source_height: u32,
    sprite: &SceneSpriteNode,
    fit: &backlayer_types::ImageFitMode,
) -> (u32, u32, i64, i64) {
    let scaled_source_width = ((source_width as f32) * sprite.scale).round().max(1.0) as u32;
    let scaled_source_height = ((source_height as f32) * sprite.scale).round().max(1.0) as u32;
    let source_aspect = scaled_source_width as f32 / scaled_source_height as f32;
    let canvas_aspect = canvas_width as f32 / canvas_height as f32;

    let (base_width, base_height) = match fit {
        backlayer_types::ImageFitMode::Contain => {
            if source_aspect > canvas_aspect {
                (
                    canvas_width,
                    ((canvas_width as f32) / source_aspect).round().max(1.0) as u32,
                )
            } else {
                (
                    ((canvas_height as f32) * source_aspect).round().max(1.0) as u32,
                    canvas_height,
                )
            }
        }
        backlayer_types::ImageFitMode::Stretch => (canvas_width, canvas_height),
        backlayer_types::ImageFitMode::Center => (scaled_source_width, scaled_source_height),
        backlayer_types::ImageFitMode::Cover => {
            if source_aspect > canvas_aspect {
                (
                    ((canvas_height as f32) * source_aspect).round().max(1.0) as u32,
                    canvas_height,
                )
            } else {
                (
                    canvas_width,
                    ((canvas_width as f32) / source_aspect).round().max(1.0) as u32,
                )
            }
        }
    };
    let (target_width, target_height) = match fit {
        backlayer_types::ImageFitMode::Center => (base_width, base_height),
        _ => (
            ((base_width as f32) * sprite.scale).round().max(1.0) as u32,
            ((base_height as f32) * sprite.scale).round().max(1.0) as u32,
        ),
    };

    let x = ((canvas_width as i64 - target_width as i64) / 2) + sprite.x.round() as i64;
    let y = ((canvas_height as i64 - target_height as i64) / 2) + sprite.y.round() as i64;
    (target_width, target_height, x, y)
}

fn overlay_effect_preview(canvas: &mut RgbaImage, effect: &SceneEffectNode) {
    let [red, green, blue] = parse_color_components(&sanitize_effect_color_hex(
        effect.color_hex.as_deref(),
        &effect.effect,
    ));
    let mut layer = match effect.effect {
        SceneEffectKind::Glow => render_glow_overlay(
            canvas.width(),
            canvas.height(),
            effect.opacity * effect.intensity.max(0.2),
            [red, green, blue],
        ),
        SceneEffectKind::Vignette => render_vignette_overlay(
            canvas.width(),
            canvas.height(),
            effect.opacity * effect.intensity.max(0.2),
            [red, green, blue],
        ),
        SceneEffectKind::Scanlines => render_scanline_overlay(
            canvas.width(),
            canvas.height(),
            effect.opacity * effect.intensity.max(0.2),
            [red, green, blue],
        ),
        SceneEffectKind::Fog => render_fog_overlay(
            canvas.width(),
            canvas.height(),
            effect.opacity * effect.intensity.max(0.2),
            [red, green, blue],
        ),
    };
    apply_layer_opacity(&mut layer, effect.opacity);
    imageops::overlay(canvas, &layer, 0, 0);
}

fn overlay_emitter_preview(
    canvas: &mut RgbaImage,
    emitter: &SceneEmitterNode,
    scene_name: &str,
    index: usize,
) {
    let preview_width = canvas.width();
    let preview_height = canvas.height();
    let mut layer = match emitter.preset {
        SceneEmitterPreset::Embers => render_particle_overlay(
            preview_width,
            preview_height,
            emitter.opacity,
            scene_name,
            index,
            true,
        ),
        SceneEmitterPreset::Dust => render_particle_overlay(
            preview_width,
            preview_height,
            emitter.opacity * 0.7,
            scene_name,
            index,
            false,
        ),
        SceneEmitterPreset::Rain => render_rain_overlay(
            preview_width,
            preview_height,
            emitter.opacity,
            scene_name,
            index,
        ),
        SceneEmitterPreset::Snow => render_snow_overlay(
            preview_width,
            preview_height,
            emitter.opacity,
            scene_name,
            index,
        ),
    };
    tint_emitter_preview_layer(&mut layer, emitter);
    apply_layer_opacity(&mut layer, emitter.opacity);
    let origin_x = emitter
        .origin_x
        .unwrap_or(default_emitter_origin_x(&emitter.preset))
        .clamp(0.0, 1.0);
    let origin_y = emitter
        .origin_y
        .unwrap_or(default_emitter_origin_y(&emitter.preset))
        .clamp(0.0, 1.0);
    let offset_x = ((origin_x - 0.5) * preview_width as f32 * 0.55).round() as i64;
    let offset_y = ((origin_y - 0.5) * preview_height as f32 * 0.55).round() as i64;
    imageops::overlay(canvas, &layer, offset_x, offset_y);
}

fn apply_layer_opacity(layer: &mut RgbaImage, opacity: f32) {
    let clamped = opacity.clamp(0.0, 1.0);
    if clamped >= 0.999 {
        return;
    }
    for pixel in layer.pixels_mut() {
        pixel.0[3] = ((pixel.0[3] as f32) * clamped).round().clamp(0.0, 255.0) as u8;
    }
}

fn tint_emitter_preview_layer(layer: &mut RgbaImage, emitter: &SceneEmitterNode) {
    let [red, green, blue] = parse_color_components(&sanitize_emitter_color_hex(
        emitter.color_hex.as_deref(),
        &emitter.preset,
    ));
    let red = (red * 255.0).round().clamp(0.0, 255.0) as u8;
    let green = (green * 255.0).round().clamp(0.0, 255.0) as u8;
    let blue = (blue * 255.0).round().clamp(0.0, 255.0) as u8;
    for pixel in layer.pixels_mut() {
        if pixel.0[3] == 0 {
            continue;
        }
        pixel.0[0] = ((pixel.0[0] as u16 * red as u16) / 255) as u8;
        pixel.0[1] = ((pixel.0[1] as u16 * green as u16) / 255) as u8;
        pixel.0[2] = ((pixel.0[2] as u16 * blue as u16) / 255) as u8;
    }
}

fn render_glow_overlay(width: u32, height: u32, opacity: f32, color: [f32; 3]) -> RgbaImage {
    let mut image = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;
    let radius = width.max(height) as f32 * 0.45;
    let alpha_scale = (opacity.clamp(0.0, 1.0) * 140.0).round() as u8;

    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let dx = x as f32 - center_x;
        let dy = y as f32 - center_y;
        let distance = (dx * dx + dy * dy).sqrt();
        if distance > radius {
            continue;
        }

        let strength = 1.0 - (distance / radius);
        let alpha = (strength.powf(2.2) * alpha_scale as f32)
            .round()
            .clamp(0.0, 255.0) as u8;
        *pixel = Rgba([
            (color[0] * 255.0).round().clamp(0.0, 255.0) as u8,
            (color[1] * 255.0).round().clamp(0.0, 255.0) as u8,
            (color[2] * 255.0).round().clamp(0.0, 255.0) as u8,
            alpha,
        ]);
    }

    image
}

fn render_vignette_overlay(width: u32, height: u32, opacity: f32, color: [f32; 3]) -> RgbaImage {
    let mut image = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;
    let max_distance = (center_x * center_x + center_y * center_y).sqrt();
    let alpha_scale = (opacity.clamp(0.0, 1.0) * 190.0).round() as u8;

    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let dx = x as f32 - center_x;
        let dy = y as f32 - center_y;
        let normalized = (dx * dx + dy * dy).sqrt() / max_distance.max(1.0);
        let strength = ((normalized - 0.45) / 0.55).clamp(0.0, 1.0);
        let alpha = (strength.powf(1.8) * alpha_scale as f32)
            .round()
            .clamp(0.0, 255.0) as u8;
        *pixel = Rgba([
            (color[0] * 255.0).round().clamp(0.0, 255.0) as u8,
            (color[1] * 255.0).round().clamp(0.0, 255.0) as u8,
            (color[2] * 255.0).round().clamp(0.0, 255.0) as u8,
            alpha,
        ]);
    }

    image
}

fn render_scanline_overlay(width: u32, height: u32, opacity: f32, color: [f32; 3]) -> RgbaImage {
    let mut image = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    let alpha = (opacity.clamp(0.0, 1.0) * 96.0).round() as u8;

    for y in 0..height {
        if y % 6 == 0 {
            for x in 0..width {
                image.put_pixel(
                    x,
                    y,
                    Rgba([
                        (color[0] * 255.0).round().clamp(0.0, 255.0) as u8,
                        (color[1] * 255.0).round().clamp(0.0, 255.0) as u8,
                        (color[2] * 255.0).round().clamp(0.0, 255.0) as u8,
                        alpha,
                    ]),
                );
            }
        }
    }

    image
}

fn render_fog_overlay(width: u32, height: u32, opacity: f32, color: [f32; 3]) -> RgbaImage {
    let mut image = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let nx = x as f32 / width.max(1) as f32;
        let ny = y as f32 / height.max(1) as f32;
        let band =
            ((ny - 0.18) / 0.62).clamp(0.0, 1.0) * (1.0 - ((ny - 0.68) / 0.36).clamp(0.0, 1.0));
        let wave = ((nx * 7.5).sin() * 0.5 + 0.5) * 0.18;
        let alpha = ((band + wave) * 255.0 * opacity * 0.34).clamp(0.0, 255.0) as u8;
        *pixel = Rgba([
            (color[0] * 255.0).round().clamp(0.0, 255.0) as u8,
            (color[1] * 255.0).round().clamp(0.0, 255.0) as u8,
            (color[2] * 255.0).round().clamp(0.0, 255.0) as u8,
            alpha,
        ]);
    }
    image
}

fn render_particle_overlay(
    width: u32,
    height: u32,
    opacity: f32,
    scene_name: &str,
    index: usize,
    warm: bool,
) -> RgbaImage {
    let mut image = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    let mut seed = stable_seed(&(scene_name, index, warm));
    let alpha_scale = opacity.clamp(0.0, 1.0);
    let count = ((width.max(height) / 18).max(24)) as usize;

    for _ in 0..count {
        let x = next_u32(&mut seed) % width.max(1);
        let y = next_u32(&mut seed) % height.max(1);
        let radius = 1 + (next_u32(&mut seed) % 5);
        let alpha = (((40 + (next_u32(&mut seed) % 120)) as f32) * alpha_scale)
            .round()
            .clamp(0.0, 255.0) as u8;
        let color = if warm {
            match next_u32(&mut seed) % 3 {
                0 => [255, 158, 87, alpha],
                1 => [255, 212, 140, alpha],
                _ => [255, 108, 72, alpha],
            }
        } else {
            [200, 228, 255, alpha]
        };
        paint_circle(&mut image, x as i32, y as i32, radius as i32, Rgba(color));
    }

    image
}

fn render_rain_overlay(
    width: u32,
    height: u32,
    opacity: f32,
    scene_name: &str,
    index: usize,
) -> RgbaImage {
    let mut image = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    let mut seed = stable_seed(&(scene_name, index, "rain"));
    let alpha = (opacity.clamp(0.0, 1.0) * 110.0).round() as u8;
    let count = ((width / 22).max(18)) as usize;

    for _ in 0..count {
        let start_x = (next_u32(&mut seed) % width.max(1)) as i32;
        let start_y = (next_u32(&mut seed) % height.max(1)) as i32;
        let length = 14 + (next_u32(&mut seed) % 26) as i32;
        for step in 0..length {
            let x = start_x - (step / 3);
            let y = start_y + step;
            if x >= 0 && y >= 0 && (x as u32) < width && (y as u32) < height {
                image.put_pixel(x as u32, y as u32, Rgba([190, 220, 255, alpha]));
            }
        }
    }

    image
}

fn render_snow_overlay(
    width: u32,
    height: u32,
    opacity: f32,
    scene_name: &str,
    index: usize,
) -> RgbaImage {
    let mut image = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    let mut seed = stable_seed(&(scene_name, index, "snow"));
    let count = ((width / 26).max(22)) as usize;
    for _ in 0..count {
        let x = next_u32(&mut seed) % width.max(1);
        let y = next_u32(&mut seed) % height.max(1);
        let radius = 1 + (next_u32(&mut seed) % 3) as i32;
        let alpha = (((90 + (next_u32(&mut seed) % 100)) as f32) * opacity.clamp(0.0, 1.0))
            .round()
            .clamp(0.0, 255.0) as u8;
        paint_circle(
            &mut image,
            x as i32,
            y as i32,
            radius,
            Rgba([240, 246, 255, alpha]),
        );
    }
    image
}

fn paint_circle(image: &mut RgbaImage, center_x: i32, center_y: i32, radius: i32, color: Rgba<u8>) {
    for y in (center_y - radius)..=(center_y + radius) {
        for x in (center_x - radius)..=(center_x + radius) {
            if x < 0 || y < 0 || x as u32 >= image.width() || y as u32 >= image.height() {
                continue;
            }
            let dx = x - center_x;
            let dy = y - center_y;
            if dx * dx + dy * dy <= radius * radius {
                image.put_pixel(x as u32, y as u32, color);
            }
        }
    }
}

fn stable_seed<T: Hash>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn next_u32(seed: &mut u64) -> u32 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 32) as u32
}

fn decode_data_url_image(
    data_url: &str,
    file_name: Option<&str>,
) -> Result<(DynamicImage, String), ConfigError> {
    let (bytes, extension) = decode_data_url_bytes(data_url, file_name)?;
    let image = image::load_from_memory(&bytes).map_err(|error| {
        ConfigError::UnsupportedImportPath(format!("failed to load selected image: {error}"))
    })?;

    Ok((image, extension))
}

fn decode_data_url_bytes(
    data_url: &str,
    file_name: Option<&str>,
) -> Result<(Vec<u8>, String), ConfigError> {
    let (header, payload) = data_url
        .split_once(',')
        .ok_or_else(|| ConfigError::UnsupportedImportPath("invalid data URL".into()))?;

    if !header.ends_with(";base64") {
        return Err(ConfigError::UnsupportedImportPath(
            "only base64 data URLs are supported".into(),
        ));
    }

    let bytes = STANDARD.decode(payload).map_err(|error| {
        ConfigError::UnsupportedImportPath(format!("failed to decode selected file: {error}"))
    })?;

    let mime = header
        .strip_prefix("data:")
        .and_then(|value| value.strip_suffix(";base64"))
        .unwrap_or("application/octet-stream");
    let extension = extension_for_mime(mime)
        .or_else(|| {
            file_name
                .and_then(|name| Path::new(name).extension())
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_ascii_lowercase())
        })
        .unwrap_or_else(|| "png".to_string());

    Ok((bytes, extension))
}

fn extension_for_mime(mime: &str) -> Option<String> {
    match mime {
        "image/png" => Some("png".into()),
        "image/jpeg" => Some("jpg".into()),
        "image/webp" => Some("webp".into()),
        "image/x-portable-pixmap" => Some("ppm".into()),
        "video/mp4" => Some("mp4".into()),
        "video/x-matroska" => Some("mkv".into()),
        "video/webm" => Some("webm".into()),
        "video/quicktime" => Some("mov".into()),
        "text/plain" => Some("wgsl".into()),
        "application/octet-stream" => None,
        _ => None,
    }
}

fn relative_path(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| path.to_path_buf())
}

fn generate_video_preview(
    entrypoint: &Path,
    asset_root: &Path,
) -> Result<Option<PathBuf>, ConfigError> {
    let preview_path = asset_root.join("preview.png");
    let status = std::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-v")
        .arg("error")
        .arg("-i")
        .arg(entrypoint)
        .arg("-frames:v")
        .arg("1")
        .arg("-update")
        .arg("1")
        .arg(&preview_path)
        .status()?;

    if !status.success() || !preview_path.is_file() {
        return Ok(None);
    }

    Ok(Some(PathBuf::from("preview.png")))
}

fn slugify_name(name: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for character in name.trim().chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }

    let cleaned = slug.trim_matches('-').chars().take(48).collect::<String>();

    if cleaned.is_empty() {
        "scene".into()
    } else {
        cleaned
    }
}

#[derive(Debug, Clone)]
struct DetectedWorkshopItem {
    root: PathBuf,
    title: String,
    kind: WallpaperKind,
    entrypoint_rel: PathBuf,
    preview_rel: Option<PathBuf>,
    workshop_id: Option<String>,
    original_type: Option<String>,
    manifest_path: Option<PathBuf>,
    compatibility: CompatibilityStatus,
    warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct WallpaperEngineProject {
    #[serde(default)]
    title: Option<String>,
    #[serde(default, rename = "type")]
    wallpaper_type: Option<String>,
    #[serde(default)]
    file: Option<String>,
    #[serde(default)]
    preview: Option<String>,
    #[serde(default)]
    workshopid: Option<serde_json::Value>,
}

fn detect_workshop_item(
    path: impl AsRef<Path>,
) -> Result<Option<DetectedWorkshopItem>, ConfigError> {
    let path = path.as_ref();
    if !path.is_dir() {
        return Ok(None);
    }

    let project_path = path.join("project.json");
    if project_path.is_file() {
        let raw = fs::read_to_string(&project_path)?;
        let project: WallpaperEngineProject = serde_json::from_str(&raw)?;
        let mut warnings = Vec::new();
        let kind = match project.wallpaper_type.as_deref().map(normalize_type_name) {
            Some(kind) => kind,
            None => {
                warnings.push("project.json did not declare a wallpaper type; imported using heuristic detection".into());
                detect_kind_from_files(path)?
            }
        };
        let entrypoint_rel = resolve_entrypoint(path, &project, &kind)?;
        let preview_rel = resolve_preview_path(path, &project);
        let workshop_id = project.workshopid.as_ref().and_then(|value| match value {
            serde_json::Value::String(text) => Some(text.clone()),
            serde_json::Value::Number(number) => Some(number.to_string()),
            _ => None,
        });
        let original_type = project.wallpaper_type.clone();
        let compatibility = compatibility_for_kind(&kind);
        warnings.extend(default_import_warnings(&kind));

        return Ok(Some(DetectedWorkshopItem {
            root: path.to_path_buf(),
            title: project.title.unwrap_or_else(|| fallback_title(path)),
            kind,
            entrypoint_rel,
            preview_rel,
            workshop_id,
            original_type,
            manifest_path: Some(project_path),
            compatibility,
            warnings,
        }));
    }

    let kind = match detect_kind_from_files(path) {
        Ok(kind) => kind,
        Err(ConfigError::UnsupportedImportPath(_)) => return Ok(None),
        Err(error) => return Err(error),
    };
    let entrypoint_rel = fallback_entrypoint(path, &kind)?;
    Ok(Some(DetectedWorkshopItem {
        root: path.to_path_buf(),
        title: fallback_title(path),
        kind: kind.clone(),
        entrypoint_rel,
        preview_rel: None,
        workshop_id: None,
        original_type: None,
        manifest_path: None,
        compatibility: compatibility_for_kind(&kind),
        warnings: {
            let mut warnings =
                vec!["project.json missing; imported using heuristic detection".into()];
            warnings.extend(default_import_warnings(&kind));
            warnings
        },
    }))
}

fn resolve_preview_path(root: &Path, project: &WallpaperEngineProject) -> Option<PathBuf> {
    project
        .preview
        .as_deref()
        .map(PathBuf::from)
        .filter(|relative| root.join(relative).exists())
        .or_else(|| find_named_preview(root))
        .or_else(|| find_first_with_extensions(root, &["png", "jpg", "jpeg", "gif", "webp"], true))
}

fn compatibility_for_kind(kind: &WallpaperKind) -> CompatibilityStatus {
    match kind {
        WallpaperKind::Video => CompatibilityStatus::Partial,
        WallpaperKind::Scene | WallpaperKind::Web => CompatibilityStatus::Partial,
        WallpaperKind::Image | WallpaperKind::Shader => CompatibilityStatus::Supported,
    }
}

fn default_import_warnings(kind: &WallpaperKind) -> Vec<String> {
    match kind {
        WallpaperKind::Video => vec!["video import is recognized and first-pass playback is available through video-runner, but libmpv and hardware decode are still unfinished".into()],
        WallpaperKind::Scene => vec!["scene wallpaper imported for compatibility tracking, but a scene runtime is not implemented yet".into()],
        WallpaperKind::Web => vec!["web wallpaper imported for compatibility tracking, but a web runtime is not implemented yet".into()],
        WallpaperKind::Image | WallpaperKind::Shader => Vec::new(),
    }
}

fn normalize_type_name(value: &str) -> WallpaperKind {
    match value.trim().to_ascii_lowercase().as_str() {
        "video" => WallpaperKind::Video,
        "web" | "website" => WallpaperKind::Web,
        "scene" | "preset" => WallpaperKind::Scene,
        "image" => WallpaperKind::Image,
        _ => WallpaperKind::Scene,
    }
}

fn detect_kind_from_files(path: &Path) -> Result<WallpaperKind, ConfigError> {
    if path.join("index.html").is_file() {
        return Ok(WallpaperKind::Web);
    }
    if path.join("scene.pkg").is_file() || path.join("scene.json").is_file() {
        return Ok(WallpaperKind::Scene);
    }
    if find_first_with_extensions(path, &["mp4", "mkv", "webm", "mov"], false).is_some() {
        return Ok(WallpaperKind::Video);
    }
    if find_first_with_extensions(path, &["png", "jpg", "jpeg", "gif", "webp"], false).is_some() {
        return Ok(WallpaperKind::Image);
    }

    Err(ConfigError::UnsupportedImportPath(
        path.display().to_string(),
    ))
}

fn resolve_entrypoint(
    root: &Path,
    project: &WallpaperEngineProject,
    kind: &WallpaperKind,
) -> Result<PathBuf, ConfigError> {
    if let Some(file) = project.file.as_deref() {
        let relative = PathBuf::from(file);
        if root.join(&relative).exists() {
            return Ok(relative);
        }
    }

    fallback_entrypoint(root, kind)
}

fn fallback_entrypoint(root: &Path, kind: &WallpaperKind) -> Result<PathBuf, ConfigError> {
    match kind {
        WallpaperKind::Web => {
            let path = PathBuf::from("index.html");
            if root.join(&path).exists() {
                Ok(path)
            } else {
                Err(ConfigError::UnsupportedImportPath(
                    root.display().to_string(),
                ))
            }
        }
        WallpaperKind::Scene => find_first_with_extensions(root, &["pkg", "json"], false)
            .ok_or_else(|| ConfigError::UnsupportedImportPath(root.display().to_string())),
        WallpaperKind::Video => {
            find_first_with_extensions(root, &["mp4", "mkv", "webm", "mov"], false)
                .ok_or_else(|| ConfigError::UnsupportedImportPath(root.display().to_string()))
        }
        WallpaperKind::Image => {
            find_first_with_extensions(root, &["png", "jpg", "jpeg", "gif", "webp"], false)
                .ok_or_else(|| ConfigError::UnsupportedImportPath(root.display().to_string()))
        }
        WallpaperKind::Shader => Err(ConfigError::UnsupportedImportPath(
            root.display().to_string(),
        )),
    }
}

fn find_named_preview(root: &Path) -> Option<PathBuf> {
    let candidates = [
        "preview.png",
        "preview.jpg",
        "preview.jpeg",
        "preview.webp",
        "thumbnail.png",
        "thumbnail.jpg",
        "thumbnail.jpeg",
        "thumb.png",
        "thumb.jpg",
        "screenshot.png",
        "screenshot.jpg",
    ];

    for candidate in candidates {
        let path = root.join(candidate);
        if path.is_file() {
            return Some(PathBuf::from(candidate));
        }
    }

    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().ok()?.is_dir() {
                stack.push(path);
                continue;
            }

            let file_name = path.file_name()?.to_str()?.to_ascii_lowercase();
            if ["preview", "thumbnail", "thumb", "screenshot"]
                .iter()
                .any(|needle| file_name.contains(needle))
            {
                let extension = path.extension()?.to_str()?.to_ascii_lowercase();
                if ["png", "jpg", "jpeg", "gif", "webp"].contains(&extension.as_str()) {
                    return path.strip_prefix(root).ok().map(Path::to_path_buf);
                }
            }
        }
    }

    None
}

fn find_first_with_extensions(
    root: &Path,
    extensions: &[&str],
    recursive: bool,
) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().ok()?.is_dir() {
                if recursive {
                    stack.push(path);
                }
                continue;
            }
            let extension = path.extension()?.to_str()?.to_ascii_lowercase();
            if extensions.iter().any(|candidate| *candidate == extension) {
                return path.strip_prefix(root).ok().map(Path::to_path_buf);
            }
        }
    }

    None
}

fn fallback_title(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.replace(['-', '_'], " "))
        .unwrap_or_else(|| "Imported Wallpaper".into())
}

fn import_asset_id(item: &DetectedWorkshopItem) -> String {
    let suffix = item
        .workshop_id
        .clone()
        .or_else(|| {
            item.root
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "unknown".into());

    format!("we.{}", sanitize_component(&suffix))
}

fn sanitize_component(input: &str) -> String {
    input
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<(), ConfigError> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::{Mutex, OnceLock},
        time::{SystemTime, UNIX_EPOCH},
    };

    use backlayer_types::{
        CreateSceneAssetRequest, CreateSceneImageSourceRequest, ImageFitMode, NativeSceneDocument,
        SceneBehavior, SceneBehaviorKind, SceneBlendMode, SceneColorStop, SceneCurvePoint,
        SceneEffectKind, SceneEffectNode, SceneEmitterNode, SceneEmitterPreset, SceneEmitterShape,
        SceneNode, SceneSpriteNode, WallpaperKind,
    };
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    use super::ConfigStore;

    const PNG_1X1_RGBA: &[u8] = &[
        0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, b'I', b'H',
        b'D', b'R', 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
        0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, b'I', b'D', b'A', b'T', 0x78,
        0x9C, 0x63, 0xF8, 0xCF, 0xC0, 0xF0, 0x1F, 0x00, 0x05, 0x00, 0x01, 0xFF, 0x89, 0x99,
        0x3D, 0x1D, 0x00, 0x00, 0x00, 0x00, b'I', b'E', b'N', b'D', 0xAE, 0x42, 0x60, 0x82,
    ];

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn sample_config_serializes_to_toml() {
        let store = ConfigStore;
        let rendered = store
            .serialize(&store.sample_config())
            .expect("sample config should serialize");

        assert!(rendered.contains("pause_on_fullscreen = true"));
        assert!(rendered.contains("fps_limit = 30"));
        assert!(rendered.contains("monitor_id = \"hypr:unknown:unknown:dp-1\""));
    }

    #[test]
    fn loads_config_from_toml_string() {
        let store = ConfigStore;
        let loaded = store
            .load_from_str(
                r#"
                [[assignments]]
                monitor_id = "hypr:chimei-innolux-corporation:0x14c9:chimei-innolux-corporation-0x14c9"

                [assignments.wallpaper]
                id = "demo.neon-grid"
                name = "Neon Grid"
                kind = "shader"
                animated = false
                entrypoint = "assets/demo.neon-grid/shaders/neon-grid.wgsl"

                [pause]
                pause_on_fullscreen = true
                pause_on_battery = false
                fps_limit = 45

                [ipc]
                kind = "unix_socket"
                path = "/tmp/backlayer.sock"
                "#,
            )
            .expect("config fixture should parse");

        assert_eq!(loaded.assignments.len(), 1);
        assert_eq!(
            loaded.assignments[0].monitor_id,
            "hypr:chimei-innolux-corporation:0x14c9:chimei-innolux-corporation-0x14c9"
        );
        assert_eq!(loaded.pause.fps_limit, 45);
        assert_eq!(loaded.assignments[0].wallpaper.image_fit, None);
        assert_eq!(loaded.assignments[0].settings.image_fit, None);
    }

    #[test]
    fn resolves_tilde_prefixed_paths() {
        let _guard = env_lock().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let store = ConfigStore;
        let home = std::env::var("HOME").expect("HOME should exist in tests");

        let resolved = store
            .resolve_path(PathBuf::from("~/config.toml"))
            .expect("tilde path should resolve");

        assert_eq!(resolved, PathBuf::from(home).join("config.toml"));
    }

    #[test]
    fn loads_asset_metadata_from_toml_string() {
        let asset = toml::from_str::<backlayer_types::AssetMetadata>(
            r#"
            id = "demo.sunset-stripes"
            name = "Sunset Stripes"
            kind = "image"
            animated = false
            image_fit = "cover"
            entrypoint = "assets/demo.sunset-stripes/images/sunset-stripes.ppm"
            "#,
        )
        .expect("asset metadata fixture should parse");

        assert_eq!(asset.id, "demo.sunset-stripes");
        assert_eq!(asset.kind, backlayer_types::WallpaperKind::Image);
        assert!(!asset.animated);
        assert_eq!(asset.image_fit, Some(backlayer_types::ImageFitMode::Cover));
    }

    #[test]
    fn imports_wallpaper_engine_web_item() {
        let _guard = env_lock().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let store = ConfigStore;
        let root = temp_fixture_dir("backlayer-import-web");
        let item = root.join("123");
        fs::create_dir_all(&item).expect("workshop item directory should exist");
        fs::write(
            item.join("project.json"),
            r#"{"title":"Rainy Window","type":"web","file":"index.html","preview":"preview.jpg","workshopid":"123"}"#,
        )
        .expect("project.json should be written");
        fs::write(item.join("index.html"), "<html></html>").expect("index.html should be written");
        fs::write(item.join("preview.jpg"), b"jpg").expect("preview should be written");
        unsafe {
            std::env::set_var("HOME", &root);
            std::env::set_var("BACKLAYER_ENABLE_WORKSHOP", "1");
        }

        let imported = store
            .import_wallpaper_engine_path(&item)
            .expect("web item should import");

        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].id, "we.123");
        assert_eq!(imported[0].kind, backlayer_types::WallpaperKind::Web);
        assert_eq!(
            imported[0].source_kind,
            backlayer_types::AssetSourceKind::WallpaperEngineImport
        );
        assert!(imported[0].entrypoint.ends_with("index.html"));
        assert_eq!(
            imported[0].compatibility.status,
            backlayer_types::CompatibilityStatus::Partial
        );
    }

    #[test]
    fn imports_workshop_root_with_multiple_items() {
        let _guard = env_lock().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let store = ConfigStore;
        let root = temp_fixture_dir("backlayer-import-root");
        let workshop_root = root.join("workshop");
        let video_item = workshop_root.join("100");
        fs::create_dir_all(&video_item).expect("video item dir should exist");
        fs::write(
            video_item.join("project.json"),
            r#"{"title":"Ocean Drift","type":"video","file":"wallpaper.mp4","workshopid":100}"#,
        )
        .expect("video project should be written");
        fs::write(video_item.join("wallpaper.mp4"), b"mp4").expect("video should be written");
        let scene_item = workshop_root.join("101");
        fs::create_dir_all(&scene_item).expect("scene item dir should exist");
        fs::write(scene_item.join("scene.pkg"), b"pkg").expect("scene pkg should be written");
        unsafe {
            std::env::set_var("HOME", &root);
            std::env::set_var("BACKLAYER_ENABLE_WORKSHOP", "1");
        }

        let imported = store
            .import_wallpaper_engine_path(&workshop_root)
            .expect("root should import supported children");

        assert_eq!(imported.len(), 2);
        assert!(
            imported
                .iter()
                .any(|asset| asset.kind == backlayer_types::WallpaperKind::Video)
        );
        assert!(
            imported
                .iter()
                .any(|asset| asset.kind == backlayer_types::WallpaperKind::Scene)
        );
    }

    #[test]
    fn native_file_assets_are_packaged_as_backlayer_files() {
        let _guard = env_lock().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let store = ConfigStore;
        let root = temp_fixture_dir("backlayer-native-package");
        unsafe {
            std::env::set_var("HOME", &root);
        }

        let asset = store
            .create_native_file_asset(&backlayer_types::CreateNativeAssetRequest {
                name: "Packaged Image".into(),
                kind: backlayer_types::WallpaperKind::Image,
                data_url: format!("data:image/png;base64,{}", STANDARD.encode(PNG_1X1_RGBA)),
                filename: "wallpaper.png".into(),
            })
            .expect("native image asset should be created");

        assert!(
            asset.asset_path
                .as_ref()
                .is_some_and(|path| path.ends_with("image.packaged-image.backlayer"))
        );
        assert!(
            root.join(".config/backlayer/assets/image.packaged-image.backlayer")
                .is_file()
        );

        let discovered = store.discover_all_assets().expect("assets should discover");
        assert!(discovered.iter().any(|candidate| candidate.id == asset.id));
    }

    #[test]
    fn import_uses_named_preview_when_project_does_not_declare_one() {
        let _guard = env_lock().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let store = ConfigStore;
        let root = temp_fixture_dir("backlayer-import-named-preview");
        let item = root.join("200");
        fs::create_dir_all(&item).expect("workshop item directory should exist");
        fs::write(
            item.join("project.json"),
            r#"{"title":"Named Preview","type":"web","file":"index.html","workshopid":"200"}"#,
        )
        .expect("project.json should be written");
        fs::write(item.join("index.html"), "<html></html>").expect("index.html should be written");
        fs::write(item.join("preview.png"), b"png").expect("preview should be written");
        unsafe {
            std::env::set_var("HOME", &root);
            std::env::set_var("BACKLAYER_ENABLE_WORKSHOP", "1");
        }

        let imported = store
            .import_wallpaper_engine_path(&item)
            .expect("item should import");

        let asset = imported.first().expect("asset should exist");
        assert!(
            asset
                .preview_image
                .as_ref()
                .is_some_and(|path| path.ends_with("preview.png"))
        );
    }

    #[test]
    fn import_falls_back_to_first_image_when_no_preview_name_exists() {
        let _guard = env_lock().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let store = ConfigStore;
        let root = temp_fixture_dir("backlayer-import-first-image");
        let item = root.join("201");
        fs::create_dir_all(item.join("images")).expect("images dir should exist");
        fs::write(
            item.join("project.json"),
            r#"{"title":"Image Fallback","type":"web","file":"index.html","workshopid":"201"}"#,
        )
        .expect("project.json should be written");
        fs::write(item.join("index.html"), "<html></html>").expect("index.html should be written");
        fs::write(item.join("images").join("banner.jpg"), b"jpg")
            .expect("banner should be written");
        unsafe {
            std::env::set_var("HOME", &root);
            std::env::set_var("BACKLAYER_ENABLE_WORKSHOP", "1");
        }

        let imported = store
            .import_wallpaper_engine_path(&item)
            .expect("item should import");

        let asset = imported.first().expect("asset should exist");
        assert!(
            asset
                .preview_image
                .as_ref()
                .is_some_and(|path| path.ends_with("images/banner.jpg"))
        );
    }

    #[test]
    fn creates_native_scene_v2_asset_document() {
        let _guard = env_lock().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let store = ConfigStore;
        let root = temp_fixture_dir("backlayer-native-scene-v2");
        unsafe {
            std::env::set_var("HOME", &root);
        }

        let source = root.join("source.png");
        image::RgbaImage::from_pixel(8, 8, image::Rgba([12, 18, 26, 255]))
            .save(&source)
            .expect("source image should write");

        let asset = store
            .create_native_scene_asset(
                &CreateSceneAssetRequest {
                    name: "My Scene".into(),
                    existing_asset_id: None,
                    base_asset_id: None,
                    base_image_data_url: Some(format!(
                        "data:image/png;base64,{}",
                        STANDARD.encode(fs::read(&source).expect("source should read"))
                    )),
                    base_image_filename: Some("source.png".into()),
                    base_image_path: None,
                    extra_images: vec![CreateSceneImageSourceRequest {
                        key: "overlay".into(),
                        data_url: Some(format!(
                            "data:image/png;base64,{}",
                            STANDARD.encode(fs::read(&source).expect("source should read"))
                        )),
                        existing_path: None,
                        filename: "overlay.png".into(),
                    }],
                    nodes: vec![
                        SceneNode::Sprite(SceneSpriteNode {
                            id: "sprite-base".into(),
                            name: "Base".into(),
                            enabled: true,
                            image_key: "base".into(),
                            fit: Some(ImageFitMode::Cover),
                            blend: Some(SceneBlendMode::Alpha),
                            x: 0.0,
                            y: 0.0,
                            scale: 1.0,
                            rotation_deg: 0.0,
                            opacity: 1.0,
                            particle_occluder: false,
                            particle_surface: false,
                            particle_region: None,
                            behaviors: vec![SceneBehavior {
                                kind: SceneBehaviorKind::Drift,
                                speed: 1.0,
                                amount_x: 10.0,
                                amount_y: 5.0,
                                amount: 0.0,
                                phase: 0.0,
                            }],
                        }),
                        SceneNode::Sprite(SceneSpriteNode {
                            id: "sprite-overlay".into(),
                            name: "Overlay".into(),
                            enabled: true,
                            image_key: "overlay".into(),
                            fit: Some(ImageFitMode::Contain),
                            blend: Some(SceneBlendMode::Screen),
                            x: 12.0,
                            y: -6.0,
                            scale: 0.7,
                            rotation_deg: 0.0,
                            opacity: 0.65,
                            particle_occluder: false,
                            particle_surface: false,
                            particle_region: None,
                            behaviors: vec![SceneBehavior {
                                kind: SceneBehaviorKind::Pulse,
                                speed: 1.4,
                                amount_x: 0.0,
                                amount_y: 0.0,
                                amount: 0.08,
                                phase: 0.25,
                            }],
                        }),
                        SceneNode::Effect(SceneEffectNode {
                            id: "effect-glow".into(),
                            name: "Glow".into(),
                            enabled: true,
                            effect: SceneEffectKind::Glow,
                            color_hex: Some("#ffc785".into()),
                            opacity: 0.5,
                            intensity: 0.8,
                            speed: 1.0,
                        }),
                        SceneNode::Effect(SceneEffectNode {
                            id: "effect-fog".into(),
                            name: "Fog".into(),
                            enabled: true,
                            effect: SceneEffectKind::Fog,
                            color_hex: Some("#dbe8ff".into()),
                            opacity: 0.35,
                            intensity: 0.9,
                            speed: 0.8,
                        }),
                        SceneNode::Emitter(SceneEmitterNode {
                            id: "emit-snow".into(),
                            name: "Snow".into(),
                            enabled: true,
                            preset: SceneEmitterPreset::Snow,
                            shape: Some(SceneEmitterShape::Box),
                            origin_x: Some(0.5),
                            origin_y: Some(0.0),
                            direction_deg: Some(92.0),
                            region_width: Some(0.28),
                            region_height: Some(0.06),
                            region_radius: Some(0.18),
                            line_length: Some(0.2),
                            line_angle_deg: Some(92.0),
                            emission_rate: 16.0,
                            burst_count: 10,
                            burst_on_start: true,
                            max_particles: 48,
                            opacity: 0.7,
                            size: 2.5,
                            speed: 40.0,
                            min_speed: Some(20.0),
                            max_speed: Some(58.0),
                            min_life: Some(6.0),
                            max_life: Some(9.0),
                            spread: 20.0,
                            gravity_x: 4.0,
                            gravity_y: 18.0,
                            drag: 0.05,
                            color_hex: Some("#f4f7ff".into()),
                            particle_image_key: Some("overlay".into()),
                            particle_rotation_deg: Some(0.0),
                            size_curve: vec![
                                SceneCurvePoint { x: 0.0, y: 0.8 },
                                SceneCurvePoint { x: 0.5, y: 1.0 },
                                SceneCurvePoint { x: 1.0, y: 0.85 },
                            ],
                            alpha_curve: vec![
                                SceneCurvePoint { x: 0.0, y: 0.25 },
                                SceneCurvePoint { x: 0.18, y: 0.7 },
                                SceneCurvePoint { x: 1.0, y: 0.1 },
                            ],
                            color_curve: vec![
                                SceneColorStop {
                                    x: 0.0,
                                    color_hex: "#ffffff".into(),
                                },
                                SceneColorStop {
                                    x: 1.0,
                                    color_hex: "#dbe8ff".into(),
                                },
                            ],
                        }),
                    ],
                },
                None,
            )
            .expect("scene asset should be created");

        assert_eq!(asset.kind, WallpaperKind::Scene);
        assert!(asset.animated);

        let scene_package = store
            .resolve_path(store.default_user_assets_path())
            .expect("user assets path should resolve")
            .join(format!("{}.backlayer", asset.id));
        assert!(scene_package.is_file());
        let extracted_root = store
            .extract_package_to_cache(&scene_package)
            .expect("scene package should extract");
        let scene_path = extracted_root.join("scene.json");
        let document: NativeSceneDocument = serde_json::from_str(
            &fs::read_to_string(scene_path).expect("scene document should exist"),
        )
        .expect("scene document should parse");
        assert_eq!(document.schema, "backlayer_scene_v2");
        assert_eq!(document.version, 2);
        assert_eq!(document.images.len(), 2);
        assert_eq!(document.images[1].key, "overlay");
        assert!(document.images[1].path.ends_with("images/overlay.png"));
        assert_eq!(document.nodes.len(), 5);
        assert!(document.nodes.iter().any(|node| matches!(
            node,
            SceneNode::Effect(SceneEffectNode {
                effect: SceneEffectKind::Fog,
                ..
            })
        )));
        assert!(document.nodes.iter().any(|node| matches!(
            node,
            SceneNode::Emitter(SceneEmitterNode {
                preset: SceneEmitterPreset::Snow,
                ..
            })
        )));
    }

    fn temp_fixture_dir(prefix: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be monotonic")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{suffix}"));
        fs::create_dir_all(&path).expect("temp fixture dir should be created");
        path
    }
}
