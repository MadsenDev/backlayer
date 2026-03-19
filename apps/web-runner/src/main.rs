use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow};
use backlayer_renderer_image::ImageRenderer;
use backlayer_types::{
    AssetMetadata, AssetSourceKind, CompatibilityInfo, ImageFitMode, WallpaperKind,
};
use backlayer_wayland::LayerShellRuntime;
use tracing::info;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("web_runner=info,backlayer=info")
        .compact()
        .init();

    let output_name = std::env::args()
        .nth(1)
        .context("missing output name argument")?;
    let asset_id = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "web-runner".to_string());
    let preview_path = std::env::args()
        .nth(3)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    let original_entrypoint = std::env::args()
        .nth(4)
        .map(PathBuf::from)
        .context("missing original web entrypoint")?;

    let runtime = LayerShellRuntime::new();
    let mut session = runtime
        .start_session_on_output(Some(&output_name))
        .with_context(|| format!("failed to start layer-shell session for {output_name}"))?;
    let renderer = ImageRenderer::default();
    let resolved = resolve_runtime_target(&original_entrypoint, preview_path.as_deref())
        .context("failed to resolve web runtime target")?;
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
        entrypoint: resolved.path.clone(),
        asset_path: None,
    };

    let detail = renderer
        .render_asset_once(&preview_asset, &mut session)
        .map_err(|error| anyhow!(error.to_string()))?;

    info!(
        output = %output_name,
        asset_id = %asset_id,
        source = %original_entrypoint.display(),
        runtime_mode = %resolved.mode,
        target = %resolved.path.display(),
        detail = %detail,
        "web runner started"
    );

    loop {
        session
            .dispatch_pending()
            .map_err(|error| anyhow!("wayland dispatch failed: {error}"))?;
        thread::sleep(Duration::from_millis(16));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedRuntimeTarget {
    path: PathBuf,
    mode: &'static str,
}

fn resolve_runtime_target(
    entrypoint: &Path,
    preview_path: Option<&Path>,
) -> Result<ResolvedRuntimeTarget> {
    if let Some(image) = extract_image_target(entrypoint)? {
        return Ok(ResolvedRuntimeTarget {
            path: image,
            mode: "html_image",
        });
    }

    if let Some(color) = extract_background_color(entrypoint)? {
        return Ok(ResolvedRuntimeTarget {
            path: write_color_ppm(color)?,
            mode: "html_color",
        });
    }

    if let Some(preview) = preview_path.filter(|path| path.is_file()) {
        return Ok(ResolvedRuntimeTarget {
            path: preview.to_path_buf(),
            mode: "preview_fallback",
        });
    }

    Err(anyhow!(
        "no supported local image, background color, or preview fallback was found for {}",
        entrypoint.display()
    ))
}

fn extract_image_target(entrypoint: &Path) -> Result<Option<PathBuf>> {
    let html = fs::read_to_string(entrypoint)
        .with_context(|| format!("failed to read {}", entrypoint.display()))?;
    let entry_root = entrypoint.parent().unwrap_or_else(|| Path::new("."));

    for candidate in [
        extract_url_value(&html),
        extract_img_src(&html),
        extract_meta_image(&html),
    ]
    .into_iter()
    .flatten()
    {
        if candidate.starts_with("http://")
            || candidate.starts_with("https://")
            || candidate.starts_with("data:")
        {
            continue;
        }

        let resolved = entry_root.join(candidate);
        if resolved.is_file() {
            return Ok(Some(resolved));
        }
    }

    Ok(None)
}

fn extract_background_color(entrypoint: &Path) -> Result<Option<[u8; 3]>> {
    let html = fs::read_to_string(entrypoint)
        .with_context(|| format!("failed to read {}", entrypoint.display()))?;

    for needle in ["background-color:", "background:"] {
        if let Some(value) = extract_css_value(&html, needle)
            && let Some(color) = parse_hex_color(&value)
        {
            return Ok(Some(color));
        }
    }

    Ok(None)
}

fn extract_url_value(html: &str) -> Option<String> {
    let marker = "url(";
    let start = html.find(marker)? + marker.len();
    let remainder = &html[start..];
    let end = remainder.find(')')?;
    Some(
        remainder[..end]
            .trim()
            .trim_matches('\'')
            .trim_matches('"')
            .to_string(),
    )
}

fn extract_img_src(html: &str) -> Option<String> {
    extract_attr_value(html, "img", "src")
}

fn extract_meta_image(html: &str) -> Option<String> {
    extract_attr_value(html, "meta", "content")
}

fn extract_attr_value(html: &str, tag: &str, attr: &str) -> Option<String> {
    let tag_marker = format!("<{tag}");
    let attr_marker = format!("{attr}=");
    let mut search = html;

    while let Some(tag_index) = search.find(&tag_marker) {
        let tagged = &search[tag_index..];
        let end = tagged.find('>')?;
        let tag_slice = &tagged[..end];
        if let Some(attr_index) = tag_slice.find(&attr_marker) {
            let value = &tag_slice[attr_index + attr_marker.len()..];
            let quote = value.chars().next()?;
            if quote == '"' || quote == '\'' {
                let quoted = &value[1..];
                let value_end = quoted.find(quote)?;
                return Some(quoted[..value_end].to_string());
            }
        }
        search = &tagged[end..];
    }

    None
}

fn extract_css_value(html: &str, needle: &str) -> Option<String> {
    let start = html.find(needle)? + needle.len();
    let remainder = &html[start..];
    let end = remainder
        .find(';')
        .or_else(|| remainder.find('"'))
        .or_else(|| remainder.find('\''))
        .unwrap_or(remainder.len());
    Some(remainder[..end].trim().to_string())
}

fn parse_hex_color(value: &str) -> Option<[u8; 3]> {
    let hex = value.trim().strip_prefix('#')?;
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            Some([r, g, b])
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some([r, g, b])
        }
        _ => None,
    }
}

fn write_color_ppm(color: [u8; 3]) -> Result<PathBuf> {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should advance")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("backlayer-web-color-{unique}.ppm"));
    fs::write(
        &path,
        format!("P3\n1 1\n255\n{} {} {}\n", color[0], color[1], color[2]),
    )
    .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::{
        extract_background_color, extract_image_target, parse_hex_color, resolve_runtime_target,
    };

    #[test]
    fn resolves_first_local_image_reference() {
        let root = temp_root("image");
        fs::write(
            root.join("index.html"),
            r#"<html><body><img src="wallpaper.png"></body></html>"#,
        )
        .expect("html should write");
        fs::write(root.join("wallpaper.png"), b"png").expect("image should write");

        let resolved = extract_image_target(&root.join("index.html"))
            .expect("extract should succeed")
            .expect("image should resolve");
        assert_eq!(resolved, root.join("wallpaper.png"));

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn resolves_background_color_when_present() {
        let root = temp_root("color");
        fs::write(
            root.join("index.html"),
            r#"<html><body style="background-color: #1a2b3c;"></body></html>"#,
        )
        .expect("html should write");

        let color = extract_background_color(&root.join("index.html"))
            .expect("extract should succeed")
            .expect("color should resolve");
        assert_eq!(color, [0x1a, 0x2b, 0x3c]);

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn preview_fallback_is_used_when_html_has_no_supported_target() {
        let root = temp_root("preview");
        let html = root.join("index.html");
        let preview = root.join("preview.png");
        fs::write(&html, "<html><body>Hello</body></html>").expect("html should write");
        fs::write(&preview, b"png").expect("preview should write");

        let resolved =
            resolve_runtime_target(&html, Some(&preview)).expect("resolution should succeed");
        assert_eq!(resolved.mode, "preview_fallback");
        assert_eq!(resolved.path, preview);

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn parses_short_and_long_hex_colors() {
        assert_eq!(parse_hex_color("#abc"), Some([0xaa, 0xbb, 0xcc]));
        assert_eq!(parse_hex_color("#a1b2c3"), Some([0xa1, 0xb2, 0xc3]));
        assert_eq!(parse_hex_color("red"), None);
    }

    fn temp_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "backlayer-web-runner-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp root should exist");
        root
    }
}
