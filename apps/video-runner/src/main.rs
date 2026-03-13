use std::{path::PathBuf, thread, time::Duration};

use anyhow::{Context, Result, anyhow};
use backlayer_renderer_image::ImageRenderer;
use backlayer_types::{
    AssetMetadata, AssetSourceKind, CompatibilityInfo, ImageFitMode, WallpaperKind,
};
use backlayer_wayland::LayerShellRuntime;
use tracing::info;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("video_runner=info,backlayer=info")
        .compact()
        .init();

    let output_name = std::env::args()
        .nth(1)
        .context("missing output name argument")?;
    let asset_id = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "video-runner".to_string());
    let preview_path = std::env::args()
        .nth(3)
        .map(PathBuf::from)
        .context("missing preview image path")?;
    let original_entrypoint = std::env::args()
        .nth(4)
        .map(PathBuf::from)
        .context("missing original video entrypoint")?;

    let runtime = LayerShellRuntime::new();
    let mut session = runtime
        .start_session_on_output(Some(&output_name))
        .with_context(|| format!("failed to start layer-shell session for {output_name}"))?;
    let renderer = ImageRenderer::default();
    let preview_asset = AssetMetadata {
        id: format!("{asset_id}.preview"),
        name: format!("{asset_id} Preview"),
        kind: WallpaperKind::Image,
        animated: false,
        image_fit: Some(ImageFitMode::Contain),
        source_kind: AssetSourceKind::Native,
        preview_image: None,
        compatibility: CompatibilityInfo::default(),
        import_metadata: None,
        entrypoint: preview_path.clone(),
    };

    let detail = renderer
        .render_asset_once(&preview_asset, &mut session)
        .map_err(|error| anyhow!(error.to_string()))?;

    info!(
        output = %output_name,
        asset_id = %asset_id,
        preview = %preview_path.display(),
        source = %original_entrypoint.display(),
        detail = %detail,
        "video runner started in preview fallback mode"
    );

    loop {
        session
            .dispatch_pending()
            .map_err(|error| anyhow!("wayland dispatch failed: {error}"))?;
        thread::sleep(Duration::from_millis(16));
    }
}
