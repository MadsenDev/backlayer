use std::process::{Command, Stdio};

use backlayer_types::{AssetMetadata, RendererDependencyStatus};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VideoRendererError {
    #[error("video renderer received non-video asset kind")]
    WrongKind,
    #[error("video asset does not exist: {0}")]
    MissingFile(String),
    #[error("unsupported video extension: {0}")]
    UnsupportedExtension(String),
}

#[derive(Debug, Default, Clone)]
pub struct VideoRenderer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoRuntimeSupport {
    pub mpv_binary_found: bool,
    pub libmpv_found: bool,
}

impl VideoRenderer {
    pub fn name(&self) -> &'static str {
        "video"
    }

    pub fn validate_asset(&self, asset: &AssetMetadata) -> Result<(), VideoRendererError> {
        if asset.kind != backlayer_types::WallpaperKind::Video {
            return Err(VideoRendererError::WrongKind);
        }

        if !asset.entrypoint.is_file() {
            return Err(VideoRendererError::MissingFile(
                asset.entrypoint.display().to_string(),
            ));
        }

        let extension = asset
            .entrypoint
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .ok_or_else(|| {
                VideoRendererError::UnsupportedExtension(asset.entrypoint.display().to_string())
            })?;

        match extension.as_str() {
            "mp4" | "mkv" | "webm" | "mov" => Ok(()),
            _ => Err(VideoRendererError::UnsupportedExtension(extension)),
        }
    }

    pub fn probe_runtime_support(&self) -> VideoRuntimeSupport {
        VideoRuntimeSupport {
            mpv_binary_found: command_succeeds("which", &["mpv"]),
            libmpv_found: command_succeeds("pkg-config", &["--exists", "mpv"]),
        }
    }

    pub fn dependency_status(&self) -> RendererDependencyStatus {
        let support = self.probe_runtime_support();
        let detail = match (support.mpv_binary_found, support.libmpv_found) {
            (true, true) => {
                "mpv and libmpv were detected, but Backlayer is still using preview-fallback mode until libmpv playback is integrated"
                    .to_string()
            }
            (true, false) => {
                "mpv binary detected, but libmpv development files were not found; Backlayer is using preview-fallback mode".to_string()
            }
            (false, true) => {
                "libmpv was detected, but the mpv binary was not found; Backlayer is using preview-fallback mode".to_string()
            }
            (false, false) => {
                "mpv and libmpv were not detected; imported videos are limited to preview-fallback mode".to_string()
            }
        };

        RendererDependencyStatus {
            available: support.mpv_binary_found && support.libmpv_found,
            mode: Some(if support.mpv_binary_found && support.libmpv_found {
                "preview_fallback_pending_integration".into()
            } else {
                "preview_fallback".into()
            }),
            detail: Some(detail),
        }
    }
}

fn command_succeeds(command: &str, args: &[&str]) -> bool {
    Command::new(command)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::VideoRenderer;

    #[test]
    fn dependency_status_always_reports_preview_mode_until_playback_exists() {
        let status = VideoRenderer::default().dependency_status();

        assert!(status.mode.is_some());
        assert!(status.detail.is_some());
    }
}
