use std::{thread, time::Duration};

use anyhow::{Context, Result, anyhow};
use backlayer_renderer_shader::ShaderRenderer;
use backlayer_wayland::LayerShellRuntime;
use tracing::info;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("animation_probe=info,backlayer=info")
        .compact()
        .init();

    let output_name = std::env::args().nth(1);
    let fps = std::env::args()
        .nth(2)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(30)
        .max(1);
    let runtime = LayerShellRuntime::new();
    let mut session = runtime
        .start_session_on_output(output_name.as_deref())
        .context("failed to start layer-shell session")?;
    let mut shader_runtime = ShaderRenderer::default()
        .create_animated_probe_runtime("animation-probe", &session)
        .map_err(|error| anyhow!(error.to_string()))?;
    let (width, height) = session.dimensions();

    info!(
        detail = %shader_runtime.detail(),
        width,
        height,
        fps,
        "animation probe started"
    );

    let frame_interval = Duration::from_millis((1000 / fps).max(1));

    loop {
        shader_runtime
            .render_frame()
            .map_err(|error| anyhow!(error.to_string()))?;
        session
            .dispatch_pending()
            .map_err(|error| anyhow!("wayland dispatch failed: {error}"))?;
        thread::sleep(frame_interval);
    }
}
