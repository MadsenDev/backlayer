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
    pub ffmpeg_tools_found: bool,
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
            ffmpeg_tools_found: command_succeeds("which", &["ffmpeg"])
                && command_succeeds("which", &["ffprobe"]),
            mpv_binary_found: command_succeeds("which", &["mpv"]),
            libmpv_found: command_succeeds("pkg-config", &["--exists", "mpv"]),
        }
    }

    pub fn dependency_status(&self) -> RendererDependencyStatus {
        let support = self.probe_runtime_support();
        let detail = match (
            support.ffmpeg_tools_found,
            support.mpv_binary_found,
            support.libmpv_found,
        ) {
            (true, true, true) => {
                "ffmpeg and ffprobe were detected, along with mpv and libmpv. Backlayer can use first-pass FFmpeg CLI playback now, while libmpv integration remains future work."
                    .to_string()
            }
            (true, _, _) => {
                "ffmpeg and ffprobe were detected. Backlayer can use first-pass FFmpeg CLI playback, but libmpv integration is still pending.".to_string()
            }
            (false, true, true) => {
                "mpv and libmpv were detected, but ffmpeg and ffprobe were not found for Backlayer's current playback path.".to_string()
            }
            (false, _, _) => {
                "ffmpeg and ffprobe were not detected; video wallpapers are unavailable on this build path.".to_string()
            }
        };

        RendererDependencyStatus {
            available: support.ffmpeg_tools_found,
            mode: Some(if support.ffmpeg_tools_found {
                "ffmpeg_decode".into()
            } else {
                "missing_video_runtime".into()
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
    fn dependency_status_reports_video_runtime_mode() {
        let status = VideoRenderer::default().dependency_status();

        assert!(status.mode.is_some());
        assert!(status.detail.is_some());
    }
}
