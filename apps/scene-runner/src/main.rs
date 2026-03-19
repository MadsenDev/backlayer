use std::{
    collections::HashMap,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow};
use backlayer_hyprland::HyprlandClient;
use backlayer_renderer_image::ImageRenderer;
use backlayer_types::{
    AssetMetadata, AssetSourceKind, CompatibilityInfo, ImageFitMode, NativeSceneDocument,
    SceneBehaviorKind, SceneBlendMode, SceneColorStop, SceneCurvePoint, SceneEffectKind,
    SceneEmitterNode, SceneEmitterPreset, SceneEmitterShape, SceneNode, WallpaperKind,
};
use backlayer_wayland::LayerShellRuntime;
use image::{DynamicImage, Rgba, RgbaImage, imageops};
use image_dds::{ImageFormat as DdsImageFormat, Surface};
use pollster::block_on;
use serde_json::Value;
use tracing::{debug, info};
use wgpu::util::DeviceExt;

const POLICY_CHECK_INTERVAL: Duration = Duration::from_millis(250);

const SPRITE_SHADER: &str = r#"
struct SpriteUniforms {
  surface_width: f32,
  surface_height: f32,
  rect_x: f32,
  rect_y: f32,
  rect_w: f32,
  rect_h: f32,
  opacity: f32,
  _padding: f32,
}

@group(0) @binding(0)
var sprite_texture: texture_2d<f32>;

@group(0) @binding(1)
var sprite_sampler: sampler;

@group(0) @binding(2)
var<uniform> sprite: SpriteUniforms;

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
  var positions = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -3.0),
    vec2<f32>(3.0, 1.0),
    vec2<f32>(-1.0, 1.0),
  );
  var out: VertexOutput;
  out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
  return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  let pixel = in.position.xy;
  let rect_min = vec2<f32>(sprite.rect_x, sprite.rect_y);
  let rect_size = vec2<f32>(max(sprite.rect_w, 1.0), max(sprite.rect_h, 1.0));
  let uv = (pixel - rect_min) / rect_size;
  if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
    discard;
  }
  let color = textureSample(sprite_texture, sprite_sampler, uv);
  return vec4<f32>(color.rgb, color.a * sprite.opacity);
}
"#;

const EFFECT_SHADER: &str = r#"
struct EffectUniforms {
  surface_width: f32,
  surface_height: f32,
  opacity: f32,
  intensity: f32,
  time_seconds: f32,
  speed: f32,
  color_r: f32,
  color_g: f32,
  color_b: f32,
  effect_kind: u32,
  _padding: f32,
}

@group(0) @binding(0)
var<uniform> effect: EffectUniforms;

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
  var positions = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -3.0),
    vec2<f32>(3.0, 1.0),
    vec2<f32>(-1.0, 1.0),
  );
  var out: VertexOutput;
  out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
  return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  let pixel = in.position.xy;
  let center = vec2<f32>(effect.surface_width * 0.5, effect.surface_height * 0.5);
  let uv = pixel / vec2<f32>(max(effect.surface_width, 1.0), max(effect.surface_height, 1.0));
  if (effect.effect_kind == 0u) {
    let distance = distance(pixel, center) / max(effect.surface_width, effect.surface_height);
    let strength = max(0.0, 1.0 - distance * 1.65);
    let pulse = effect.intensity * (0.78 + (sin(effect.time_seconds * max(effect.speed, 0.01)) + 1.0) * 0.11);
    return vec4<f32>(effect.color_r, effect.color_g, effect.color_b, strength * strength * effect.opacity * pulse);
  }
  if (effect.effect_kind == 1u) {
    let d = distance(pixel, center) / length(center);
    let strength = clamp((d - 0.42) / 0.58, 0.0, 1.0);
    return vec4<f32>(effect.color_r, effect.color_g, effect.color_b, pow(strength, 1.8) * effect.opacity * effect.intensity);
  }
  if (effect.effect_kind == 2u) {
    let offset = fract(effect.time_seconds * max(effect.speed, 0.01) * 0.35);
    let line_phase = fract((uv.y + offset) * 96.0);
    let distance_to_center = abs(line_phase - 0.5);
    let band = 1.0 - smoothstep(0.28, 0.5, distance_to_center);
    return vec4<f32>(effect.color_r, effect.color_g, effect.color_b, band * effect.opacity * effect.intensity * 0.18);
  }
  let fog_wave = sin((uv.x * 5.0) + (effect.time_seconds * max(effect.speed, 0.01))) * 0.03;
  let band = smoothstep(0.12 + fog_wave, 0.72 + fog_wave, uv.y) * (1.0 - smoothstep(0.56 + fog_wave, 1.0, uv.y));
  return vec4<f32>(effect.color_r, effect.color_g, effect.color_b, band * effect.opacity * effect.intensity * 0.22);
}
"#;

const PARTICLE_SHADER: &str = r#"
struct ParticleUniforms {
  surface_width: f32,
  surface_height: f32,
  _padding0: f32,
  _padding1: f32,
}

struct ParticleVsIn {
  @location(0) center: vec2<f32>,
  @location(1) size: vec2<f32>,
  @location(2) angle: f32,
  @location(3) shape: f32,
  @location(4) color: vec4<f32>,
}

struct ParticleVsOut {
  @builtin(position) position: vec4<f32>,
  @location(0) local_uv: vec2<f32>,
  @location(1) shape: f32,
  @location(2) color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> particle_uniforms: ParticleUniforms;

@vertex
fn vs_main(
  @builtin(vertex_index) vertex_index: u32,
  instance: ParticleVsIn,
) -> ParticleVsOut {
  var corners = array<vec2<f32>, 6>(
    vec2<f32>(-0.5, -0.5),
    vec2<f32>(0.5, -0.5),
    vec2<f32>(0.5, 0.5),
    vec2<f32>(-0.5, -0.5),
    vec2<f32>(0.5, 0.5),
    vec2<f32>(-0.5, 0.5),
  );

  let local = corners[vertex_index];
  let cos_a = cos(instance.angle);
  let sin_a = sin(instance.angle);
  let rotated = vec2<f32>(
    (local.x * instance.size.x * cos_a) - (local.y * instance.size.y * sin_a),
    (local.x * instance.size.x * sin_a) + (local.y * instance.size.y * cos_a),
  );
  let pixel = instance.center + rotated;
  let clip = vec2<f32>(
    (pixel.x / max(particle_uniforms.surface_width, 1.0)) * 2.0 - 1.0,
    1.0 - (pixel.y / max(particle_uniforms.surface_height, 1.0)) * 2.0,
  );

  var out: ParticleVsOut;
  out.position = vec4<f32>(clip, 0.0, 1.0);
  out.local_uv = local;
  out.shape = instance.shape;
  out.color = instance.color;
  return out;
}

@fragment
fn fs_main(in: ParticleVsOut) -> @location(0) vec4<f32> {
  if (in.shape > 0.5) {
    let radial = length(in.local_uv * 2.0);
    if (radial > 1.0) {
      discard;
    }
    let feather = smoothstep(1.0, 0.0, radial);
    return vec4<f32>(in.color.rgb, in.color.a * feather);
  }

  let edge = max(abs(in.local_uv.x) * 2.0, abs(in.local_uv.y) * 2.0);
  let feather = smoothstep(1.0, 0.82, 1.0 - edge);
  return vec4<f32>(in.color.rgb, in.color.a * feather);
}
"#;

fn main() -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("scene_runner=info,backlayer=info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .init();

    let output_name = std::env::args()
        .nth(1)
        .context("missing output name argument")?;
    let fps = std::env::args()
        .nth(2)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(24)
        .max(1);
    let pause_on_fullscreen = std::env::args().nth(3).as_deref() == Some("1");
    let pause_on_battery = std::env::args().nth(4).as_deref() == Some("1");
    let asset_id = std::env::args()
        .nth(5)
        .unwrap_or_else(|| "scene-runner".to_string());
    let preview_path = std::env::args()
        .nth(6)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    let original_entrypoint = std::env::args()
        .nth(7)
        .map(PathBuf::from)
        .context("missing original scene entrypoint")?;
    let debug_particle_areas = env_flag_enabled("BACKLAYER_DEBUG_PARTICLE_AREAS");

    let runtime = LayerShellRuntime::new();
    let mut session = runtime
        .start_session_on_output(Some(&output_name))
        .with_context(|| format!("failed to start layer-shell session for {output_name}"))?;
    if let Some(mut scene_runtime) =
        load_native_scene_runtime(&original_entrypoint).context("failed to load native scene")?
    {
        let mut gpu_runtime = GpuSceneRuntime::new(&session, &scene_runtime, debug_particle_areas)
            .context("failed to create native GPU scene runtime")?;
        scene_runtime.canvas_size = gpu_runtime.output_size();
        scene_runtime.update_emitters(1.0 / 120.0);
        gpu_runtime
            .render_scene(&scene_runtime, 0.0)
            .context("failed to render initial native scene frame")?;
        info!(
            output = %output_name,
            fps,
            pause_on_fullscreen,
            pause_on_battery,
            asset_id = %asset_id,
            source = %original_entrypoint.display(),
            runtime_mode = "native_scene_v2",
            detail = %"native scene frame submitted",
            "scene runner started"
        );

        let started_at = Instant::now();
        let frame_interval = Duration::from_millis((1000 / fps).max(1));
        let hyprland = HyprlandClient::new();
        let power = PowerStateProbe::default();
        let mut next_frame_at = Instant::now() + frame_interval;
        loop {
            session
                .dispatch_pending()
                .map_err(|error| anyhow!("wayland dispatch failed: {error}"))?;

            let now = Instant::now();
            let paused_for_fullscreen =
                pause_on_fullscreen && hyprland.fullscreen_active().unwrap_or(false);
            let paused_for_battery = pause_on_battery && power.on_battery().unwrap_or(false);
            let paused = paused_for_fullscreen || paused_for_battery;

            if !paused && now >= next_frame_at {
                let elapsed = started_at.elapsed().as_secs_f32();
                let dt = frame_interval.as_secs_f32();
                scene_runtime.update_emitters(dt);
                gpu_runtime
                    .render_scene(&scene_runtime, elapsed)
                    .map_err(|error| anyhow!("scene frame render failed: {error}"))?;
                next_frame_at = now + frame_interval;
            }

            let sleep_for = if paused {
                POLICY_CHECK_INTERVAL
            } else {
                next_frame_at
                    .saturating_duration_since(Instant::now())
                    .min(POLICY_CHECK_INTERVAL)
            };
            thread::sleep(sleep_for);
        }
    }

    let renderer = ImageRenderer::default();
    let resolved = resolve_runtime_target(&original_entrypoint, preview_path.as_deref())
        .context("failed to resolve scene runtime target")?;
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
        "scene runner started"
    );

    loop {
        session
            .dispatch_pending()
            .map_err(|error| anyhow!("wayland dispatch failed: {error}"))?;
        thread::sleep(POLICY_CHECK_INTERVAL);
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
    if let Some(composited) = compose_scene_target(entrypoint)? {
        return Ok(ResolvedRuntimeTarget {
            path: composited,
            mode: "scene_layers",
        });
    }

    if let Some(image) = extract_scene_image_target(entrypoint)? {
        return Ok(ResolvedRuntimeTarget {
            path: image,
            mode: "scene_image",
        });
    }

    if let Some(sibling) = find_first_sibling_image(entrypoint) {
        return Ok(ResolvedRuntimeTarget {
            path: sibling,
            mode: "scene_sibling_image",
        });
    }

    if let Some(preview) = preview_path.filter(|path| path.is_file()) {
        return Ok(ResolvedRuntimeTarget {
            path: preview.to_path_buf(),
            mode: "preview_fallback",
        });
    }

    Err(anyhow!(
        "no supported local image or preview fallback was found for {}",
        entrypoint.display()
    ))
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
            let kind = fs::read_to_string(path.join("type")).unwrap_or_default();
            match kind.trim() {
                "Battery" => {
                    saw_battery = true;
                    let status = fs::read_to_string(path.join("status")).unwrap_or_default();
                    if status.trim() == "Discharging" {
                        return Ok(true);
                    }
                }
                "Mains" | "USB" | "USB_C" => {
                    let online = fs::read_to_string(path.join("online")).unwrap_or_default();
                    if online.trim() == "1" {
                        saw_online_external_power = true;
                    }
                }
                _ => {}
            }
        }

        Ok(saw_battery && !saw_online_external_power)
    }
}

fn compose_scene_target(entrypoint: &Path) -> Result<Option<PathBuf>> {
    if entrypoint.extension().and_then(|ext| ext.to_str()) == Some("pkg") {
        return match compose_scene_pkg_target(entrypoint) {
            Ok(target) => Ok(target),
            Err(error) => {
                info!(
                    source = %entrypoint.display(),
                    error = %error,
                    "scene pkg composition failed"
                );
                Ok(None)
            }
        };
    }

    if entrypoint.extension().and_then(|ext| ext.to_str()) != Some("json") {
        return Ok(None);
    }

    let raw = fs::read_to_string(entrypoint)
        .with_context(|| format!("failed to read {}", entrypoint.display()))?;
    let value: Value = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", entrypoint.display()))?;
    let root = entrypoint.parent().unwrap_or_else(|| Path::new("."));
    let layers = extract_scene_layers(&value, root);
    if layers.len() < 2 {
        return Ok(None);
    }

    let canvas = compose_layers(&layers, canvas_size(&value, &layers));
    Ok(Some(write_scene_png(canvas)?))
}

fn extract_scene_image_target(entrypoint: &Path) -> Result<Option<PathBuf>> {
    if entrypoint.extension().and_then(|ext| ext.to_str()) == Some("pkg") {
        return match extract_scene_pkg_image_target(entrypoint) {
            Ok(target) => Ok(target),
            Err(error) => {
                info!(
                    source = %entrypoint.display(),
                    error = %error,
                    "scene pkg image extraction failed"
                );
                Ok(None)
            }
        };
    }

    if entrypoint.extension().and_then(|ext| ext.to_str()) != Some("json") {
        return Ok(None);
    }

    let raw = fs::read_to_string(entrypoint)
        .with_context(|| format!("failed to read {}", entrypoint.display()))?;
    let value: Value = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", entrypoint.display()))?;
    let root = entrypoint.parent().unwrap_or_else(|| Path::new("."));

    Ok(find_image_in_json(&value, root))
}

#[derive(Debug, Clone)]
struct NativeSceneRuntime {
    document: NativeSceneDocument,
    images: HashMap<String, RgbaImage>,
    emitters: Vec<EmitterState>,
    canvas_size: (u32, u32),
}

#[derive(Debug, Clone)]
struct EmitterState {
    node_id: String,
    accumulator: f32,
    seed: u64,
    burst_fired: bool,
    particles: Vec<SceneParticle>,
}

#[derive(Debug, Clone)]
struct SceneParticle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    max_life: f32,
    size: f32,
    alpha: f32,
    landed: bool,
}

#[derive(Debug, Clone)]
struct ParticleBlocker {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    polygon: Vec<(f32, f32)>,
    occluder: bool,
    surface: bool,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SpriteUniforms {
    surface_width: f32,
    surface_height: f32,
    rect_x: f32,
    rect_y: f32,
    rect_w: f32,
    rect_h: f32,
    opacity: f32,
    _padding: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct EffectUniforms {
    surface_width: f32,
    surface_height: f32,
    opacity: f32,
    intensity: f32,
    time_seconds: f32,
    speed: f32,
    color_r: f32,
    color_g: f32,
    color_b: f32,
    effect_kind: u32,
    _padding: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ParticleUniforms {
    surface_width: f32,
    surface_height: f32,
    _padding0: f32,
    _padding1: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ParticleInstance {
    center_x: f32,
    center_y: f32,
    size_x: f32,
    size_y: f32,
    angle: f32,
    shape: f32,
    color_r: f32,
    color_g: f32,
    color_b: f32,
    color_a: f32,
}

struct GpuSceneRuntime {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sprite_pipeline_alpha: wgpu::RenderPipeline,
    sprite_pipeline_additive: wgpu::RenderPipeline,
    effect_pipeline: wgpu::RenderPipeline,
    particle_pipeline: wgpu::RenderPipeline,
    effect_bind_group: wgpu::BindGroup,
    particle_bind_group: wgpu::BindGroup,
    effect_uniform_buffer: wgpu::Buffer,
    particle_uniform_buffer: wgpu::Buffer,
    sprite_uniform_buffer: wgpu::Buffer,
    image_bind_groups: HashMap<String, wgpu::BindGroup>,
    particle_instance_buffer: wgpu::Buffer,
    particle_capacity: usize,
    surface_size: (u32, u32),
    debug_particle_areas: bool,
}

impl NativeSceneRuntime {
    fn update_emitters(&mut self, delta_seconds: f32) {
        let blockers = build_particle_blockers(self);
        for emitter in self.document.nodes.iter().filter_map(|node| match node {
            SceneNode::Emitter(emitter) if emitter.enabled => Some(emitter),
            _ => None,
        }) {
            let Some(state) = self
                .emitters
                .iter_mut()
                .find(|state| state.node_id == emitter.id)
            else {
                continue;
            };

            if emitter.burst_on_start && !state.burst_fired {
                let burst_count = emitter.burst_count.min(emitter.max_particles) as usize;
                for _ in 0..burst_count {
                    if state.particles.len() >= emitter.max_particles as usize {
                        break;
                    }
                    state.particles.push(spawn_particle(
                        emitter,
                        self.canvas_size,
                        &mut state.seed,
                    ));
                }
                state.burst_fired = true;
            }

            if state.particles.is_empty() && emitter.emission_rate > 0.0 {
                let average_life =
                    (resolve_emitter_min_life(emitter) + resolve_emitter_max_life(emitter)) * 0.5;
                let warm_count = ((emitter.emission_rate * average_life).round() as usize)
                    .min(emitter.max_particles as usize);
                for _ in 0..warm_count {
                    if state.particles.len() >= emitter.max_particles as usize {
                        break;
                    }
                    let warm_age =
                        average_life * ((next_u32(&mut state.seed) as f32) / (u32::MAX as f32));
                    state.particles.push(spawn_particle_with_age(
                        emitter,
                        self.canvas_size,
                        &mut state.seed,
                        Some(warm_age),
                    ));
                }
            }

            state.accumulator += emitter.emission_rate * delta_seconds;
            let spawn_count = state.accumulator.floor() as usize;
            state.accumulator -= spawn_count as f32;

            for _ in 0..spawn_count {
                if state.particles.len() >= emitter.max_particles as usize {
                    break;
                }
                state
                    .particles
                    .push(spawn_particle(emitter, self.canvas_size, &mut state.seed));
            }

            for particle in &mut state.particles {
                if !particle.landed {
                    particle.vx += emitter.gravity_x * delta_seconds;
                    particle.vy += emitter.gravity_y * delta_seconds;
                    let drag_scale = (1.0 - emitter.drag * delta_seconds * 0.08).clamp(0.0, 1.0);
                    particle.vx *= drag_scale;
                    particle.vy *= drag_scale;
                    let previous_y = particle.y;
                    particle.x += particle.vx * delta_seconds;
                    particle.y += particle.vy * delta_seconds;
                    resolve_particle_surface_collision(emitter, particle, previous_y, &blockers);
                }
                particle.life += delta_seconds;
            }

            state
                .particles
                .retain(|particle| particle.life < particle.max_life);
        }
    }
}

fn load_native_scene_runtime(entrypoint: &Path) -> Result<Option<NativeSceneRuntime>> {
    if entrypoint.extension().and_then(|ext| ext.to_str()) != Some("json") {
        return Ok(None);
    }

    let raw = fs::read_to_string(entrypoint)
        .with_context(|| format!("failed to read {}", entrypoint.display()))?;
    let document: NativeSceneDocument = match serde_json::from_str(&raw) {
        Ok(document) => document,
        Err(_) => return Ok(None),
    };
    if document.schema != "backlayer_scene_v2" {
        return Ok(None);
    }

    let root = entrypoint.parent().unwrap_or_else(|| Path::new("."));
    let mut images = HashMap::new();
    for source in &document.images {
        let path = root.join(&source.path);
        if path.is_file() {
            let image = image::open(&path)
                .with_context(|| format!("failed to open {}", path.display()))?
                .to_rgba8();
            images.insert(source.key.clone(), image);
        }
    }

    let emitters = document
        .nodes
        .iter()
        .filter_map(|node| match node {
            SceneNode::Emitter(emitter) => Some(EmitterState {
                node_id: emitter.id.clone(),
                accumulator: 0.0,
                seed: stable_seed(&(emitter.id.clone(), format!("{:?}", emitter.preset))),
                burst_fired: false,
                particles: Vec::new(),
            }),
            _ => None,
        })
        .collect();

    Ok(Some(NativeSceneRuntime {
        canvas_size: (document.width.max(1), document.height.max(1)),
        document,
        images,
        emitters,
    }))
}

impl GpuSceneRuntime {
    fn new(
        session: &backlayer_wayland::LayerSurfaceSession,
        scene: &NativeSceneRuntime,
        debug_particle_areas: bool,
    ) -> Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = unsafe { session.create_wgpu_surface(&instance) }
            .map_err(|error| anyhow!("failed to create GPU surface: {error}"))?;
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .map_err(|error| anyhow!("failed to find adapter: {error}"))?;
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("backlayer-gpu-scene-runtime"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::default(),
        }))
        .map_err(|error| anyhow!("failed to request device: {error}"))?;

        let (surface_width, surface_height) = session.dimensions();
        let config = surface
            .get_default_config(&adapter, surface_width, surface_height)
            .ok_or_else(|| anyhow!("surface does not expose a default configuration"))?;
        surface.configure(&device, &config);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("backlayer-scene-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let sprite_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("backlayer-scene-sprite-uniforms"),
            size: std::mem::size_of::<SpriteUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let effect_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("backlayer-scene-effect-uniforms"),
            size: std::mem::size_of::<EffectUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let particle_uniform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("backlayer-scene-particle-uniforms"),
                contents: bytemuck::bytes_of(&ParticleUniforms {
                    surface_width: surface_width as f32,
                    surface_height: surface_height as f32,
                    _padding0: 0.0,
                    _padding1: 0.0,
                }),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let sprite_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("backlayer-scene-sprite-bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let effect_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("backlayer-scene-effect-bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let particle_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("backlayer-scene-particle-bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let sprite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("backlayer-scene-sprite-shader"),
            source: wgpu::ShaderSource::Wgsl(SPRITE_SHADER.into()),
        });
        let effect_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("backlayer-scene-effect-shader"),
            source: wgpu::ShaderSource::Wgsl(EFFECT_SHADER.into()),
        });
        let particle_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("backlayer-scene-particle-shader"),
            source: wgpu::ShaderSource::Wgsl(PARTICLE_SHADER.into()),
        });

        let alpha_blend = wgpu::BlendState::ALPHA_BLENDING;
        let additive_blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        };

        let sprite_pipeline_alpha = create_sprite_pipeline(
            &device,
            &sprite_bind_group_layout,
            &sprite_shader,
            config.format,
            alpha_blend,
            "backlayer-scene-sprite-alpha",
        );
        let sprite_pipeline_additive = create_sprite_pipeline(
            &device,
            &sprite_bind_group_layout,
            &sprite_shader,
            config.format,
            additive_blend,
            "backlayer-scene-sprite-additive",
        );

        let effect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("backlayer-scene-effect-pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("backlayer-scene-effect-pipeline"),
                    bind_group_layouts: &[&effect_bind_group_layout],
                    push_constant_ranges: &[],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &effect_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &effect_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(alpha_blend),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let particle_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("backlayer-scene-particle-pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("backlayer-scene-particle-pipeline"),
                    bind_group_layouts: &[&particle_bind_group_layout],
                    push_constant_ranges: &[],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &particle_shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<ParticleInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2,
                        1 => Float32x2,
                        2 => Float32,
                        3 => Float32,
                        4 => Float32x4
                    ],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &particle_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(additive_blend),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let effect_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("backlayer-scene-effect-bg"),
            layout: &effect_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: effect_uniform_buffer.as_entire_binding(),
            }],
        });
        let particle_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("backlayer-scene-particle-bg"),
            layout: &particle_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: particle_uniform_buffer.as_entire_binding(),
            }],
        });

        let mut image_bind_groups = HashMap::new();
        for (key, image) in &scene.images {
            let (texture, view) = create_rgba_texture(&device, &queue, key, image);
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("backlayer-scene-image-bg"),
                layout: &sprite_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: sprite_uniform_buffer.as_entire_binding(),
                    },
                ],
            });
            let _ = texture;
            image_bind_groups.insert(key.clone(), bind_group);
        }
        let particle_capacity = 4096usize;
        let particle_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("backlayer-scene-particle-instances"),
            size: (particle_capacity * std::mem::size_of::<ParticleInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            surface,
            device,
            queue,
            sprite_pipeline_alpha,
            sprite_pipeline_additive,
            effect_pipeline,
            particle_pipeline,
            effect_bind_group,
            particle_bind_group,
            effect_uniform_buffer,
            particle_uniform_buffer,
            sprite_uniform_buffer,
            image_bind_groups,
            particle_instance_buffer,
            particle_capacity,
            surface_size: (surface_width, surface_height),
            debug_particle_areas,
        })
    }

    fn output_size(&self) -> (u32, u32) {
        self.surface_size
    }

    fn render_scene(&mut self, scene: &NativeSceneRuntime, time_seconds: f32) -> Result<()> {
        let mut particle_instances = build_particle_instances(scene);
        if self.debug_particle_areas {
            particle_instances.extend(build_debug_particle_area_instances(
                scene,
                self.surface_size,
            ));
        }
        let particle_count = particle_instances.len().min(self.particle_capacity);
        if particle_count > 0 {
            self.queue.write_buffer(
                &self.particle_instance_buffer,
                0,
                bytemuck::cast_slice(&particle_instances[..particle_count]),
            );
        }

        let frame_texture = self
            .surface
            .get_current_texture()
            .map_err(|error| anyhow!("failed to acquire surface frame: {error}"))?;
        let view = frame_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("backlayer-gpu-scene-frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("backlayer-gpu-scene-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 5.0 / 255.0,
                            g: 7.0 / 255.0,
                            b: 10.0 / 255.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            for node in &scene.document.nodes {
                match node {
                    SceneNode::Sprite(sprite) if sprite.enabled => {
                        let Some(image) = scene.images.get(&sprite.image_key) else {
                            continue;
                        };
                        let Some(bind_group) = self.image_bind_groups.get(&sprite.image_key) else {
                            continue;
                        };
                        let (rect_w, rect_h, rect_x, rect_y, opacity) =
                            scene_sprite_layout(self.surface_size, image, sprite, time_seconds);
                        let uniforms = SpriteUniforms {
                            surface_width: self.surface_size.0 as f32,
                            surface_height: self.surface_size.1 as f32,
                            rect_x,
                            rect_y,
                            rect_w,
                            rect_h,
                            opacity,
                            _padding: 0.0,
                        };
                        self.queue.write_buffer(
                            &self.sprite_uniform_buffer,
                            0,
                            bytemuck::bytes_of(&uniforms),
                        );
                        pass.set_pipeline(
                            match sprite.blend.clone().unwrap_or(SceneBlendMode::Alpha) {
                                SceneBlendMode::Add | SceneBlendMode::Screen => {
                                    &self.sprite_pipeline_additive
                                }
                                SceneBlendMode::Alpha | SceneBlendMode::Multiply => {
                                    &self.sprite_pipeline_alpha
                                }
                            },
                        );
                        pass.set_bind_group(0, bind_group, &[]);
                        pass.draw(0..3, 0..1);
                    }
                    SceneNode::Effect(effect) if effect.enabled => {
                        let color = parse_effect_color(effect);
                        let uniforms = EffectUniforms {
                            surface_width: self.surface_size.0 as f32,
                            surface_height: self.surface_size.1 as f32,
                            opacity: effect.opacity.clamp(0.0, 1.0),
                            intensity: effect.intensity,
                            time_seconds,
                            speed: effect.speed,
                            color_r: color[0],
                            color_g: color[1],
                            color_b: color[2],
                            effect_kind: effect_kind_to_u32(&effect.effect),
                            _padding: 0.0,
                        };
                        self.queue.write_buffer(
                            &self.effect_uniform_buffer,
                            0,
                            bytemuck::bytes_of(&uniforms),
                        );
                        pass.set_pipeline(&self.effect_pipeline);
                        pass.set_bind_group(0, &self.effect_bind_group, &[]);
                        pass.draw(0..3, 0..1);
                    }
                    _ => {}
                }
            }
            if particle_count > 0 {
                self.queue.write_buffer(
                    &self.particle_uniform_buffer,
                    0,
                    bytemuck::bytes_of(&ParticleUniforms {
                        surface_width: self.surface_size.0 as f32,
                        surface_height: self.surface_size.1 as f32,
                        _padding0: 0.0,
                        _padding1: 0.0,
                    }),
                );
                pass.set_pipeline(&self.particle_pipeline);
                pass.set_bind_group(0, &self.particle_bind_group, &[]);
                pass.set_vertex_buffer(0, self.particle_instance_buffer.slice(..));
                pass.draw(0..6, 0..particle_count as u32);
            }
        }
        self.queue.submit(Some(encoder.finish()));
        frame_texture.present();
        Ok(())
    }
}

fn spawn_particle(
    emitter: &backlayer_types::SceneEmitterNode,
    canvas_size: (u32, u32),
    seed: &mut u64,
) -> SceneParticle {
    spawn_particle_with_age(emitter, canvas_size, seed, None)
}

fn spawn_particle_with_age(
    emitter: &backlayer_types::SceneEmitterNode,
    canvas_size: (u32, u32),
    seed: &mut u64,
    age_override: Option<f32>,
) -> SceneParticle {
    let random = |seed: &mut u64| -> f32 { (next_u32(seed) as f32) / (u32::MAX as f32) };
    let (origin_x, origin_y) = emitter_origin_pixels(emitter, canvas_size);
    let (spawn_x, spawn_y) =
        sample_emitter_position(emitter, canvas_size, seed, origin_x, origin_y);
    let spread_radians = emitter.spread.to_radians();
    let base_angle = emitter_direction_radians(emitter);
    let angle = base_angle + (random(seed) - 0.5) * spread_radians;
    let min_speed = resolve_emitter_min_speed(emitter);
    let max_speed = resolve_emitter_max_speed(emitter);
    let speed = min_speed + (random(seed) * (max_speed - min_speed));
    let min_life = resolve_emitter_min_life(emitter);
    let max_life = resolve_emitter_max_life(emitter);
    let max_life_value = min_life + (random(seed) * (max_life - min_life));
    let age = age_override.unwrap_or(0.0).clamp(0.0, max_life_value);
    let drag_scale = (1.0 - emitter.drag * age * 0.08).max(0.0);
    let vx = speed * angle.cos() * drag_scale;
    let vy = speed * angle.sin() * drag_scale;

    SceneParticle {
        x: spawn_x + (vx * age) + (0.5 * emitter.gravity_x * age * age),
        y: spawn_y + (vy * age) + (0.5 * emitter.gravity_y * age * age),
        vx,
        vy,
        life: age,
        max_life: max_life_value,
        size: emitter.size * (0.55 + random(seed) * 0.7),
        alpha: emitter.opacity * (0.55 + random(seed) * 0.45),
        landed: false,
    }
}

fn build_particle_blockers(scene: &NativeSceneRuntime) -> Vec<ParticleBlocker> {
    let mut blockers = Vec::new();
    for sprite in scene.document.nodes.iter().filter_map(|node| match node {
        SceneNode::Sprite(sprite) if sprite.enabled => Some(sprite),
        _ => None,
    }) {
        if !sprite.particle_occluder && !sprite.particle_surface {
            continue;
        }
        let Some(image) = scene.images.get(&sprite.image_key) else {
            continue;
        };
        let (width, height, x, y, _) = scene_sprite_layout(scene.canvas_size, image, sprite, 0.0);
        let (x, y, width, height) =
            resolve_particle_blocker_rect((x, y, width, height), sprite.particle_region.as_ref());
        blockers.push(ParticleBlocker {
            x,
            y,
            width,
            height,
            polygon: Vec::new(),
            occluder: sprite.particle_occluder,
            surface: sprite.particle_surface,
        });
    }
    for area in scene.document.nodes.iter().filter_map(|node| match node {
        SceneNode::ParticleArea(area) if area.enabled => Some(area),
        _ => None,
    }) {
        blockers.push(ParticleBlocker {
            x: scene.canvas_size.0 as f32 * area.region.x,
            y: scene.canvas_size.1 as f32 * area.region.y,
            width: scene.canvas_size.0 as f32 * area.region.width,
            height: scene.canvas_size.1 as f32 * area.region.height,
            polygon: if area.shape == Some(backlayer_types::SceneParticleAreaShape::Polygon) {
                area.points
                    .iter()
                    .map(|point| {
                        (
                            scene.canvas_size.0 as f32 * point.x,
                            scene.canvas_size.1 as f32 * point.y,
                        )
                    })
                    .collect()
            } else {
                Vec::new()
            },
            occluder: area.occluder,
            surface: area.surface,
        });
    }
    blockers
}

fn resolve_particle_blocker_rect(
    layout: (f32, f32, f32, f32),
    region: Option<&backlayer_types::SceneNormalizedRect>,
) -> (f32, f32, f32, f32) {
    let (x, y, width, height) = layout;
    let Some(region) = region else {
        return (x, y, width, height);
    };
    (
        x + (width * region.x.clamp(0.0, 1.0)),
        y + (height * region.y.clamp(0.0, 1.0)),
        width * region.width.clamp(0.0, 1.0),
        height * region.height.clamp(0.0, 1.0),
    )
}

fn particle_is_occluded(blockers: &[ParticleBlocker], x: f32, y: f32, radius: f32) -> bool {
    blockers
        .iter()
        .any(|blocker| blocker.occluder && blocker_contains(blocker, x, y, radius))
}

fn particle_segment_is_occluded(
    blockers: &[ParticleBlocker],
    start: (f32, f32),
    end: (f32, f32),
    thickness_radius: f32,
) -> bool {
    blockers.iter().any(|blocker| {
        blocker.occluder && blocker_contains_segment(blocker, start, end, thickness_radius)
    })
}

fn resolve_particle_surface_collision(
    emitter: &SceneEmitterNode,
    particle: &mut SceneParticle,
    previous_y: f32,
    blockers: &[ParticleBlocker],
) {
    let radius = particle.size.max(1.0);
    let Some(surface) = blockers
        .iter()
        .filter_map(|blocker| {
            if !blocker.surface {
                return None;
            }
            let surface_y = blocker_surface_y(blocker, particle.x)?;
            if previous_y <= surface_y && particle.y + radius >= surface_y {
                Some(surface_y)
            } else {
                None
            }
        })
        .min_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal))
    else {
        return;
    };

    match emitter.preset {
        SceneEmitterPreset::Snow | SceneEmitterPreset::Dust => {
            particle.y = surface - radius;
            particle.vx *= 0.15;
            particle.vy = 0.0;
            particle.landed = true;
        }
        SceneEmitterPreset::Rain | SceneEmitterPreset::Embers => {
            particle.life = particle.max_life;
        }
    }
}

fn blocker_contains(blocker: &ParticleBlocker, x: f32, y: f32, radius: f32) -> bool {
    if blocker.polygon.len() >= 3 {
        polygon_intersects_circle(&blocker.polygon, x, y, radius)
    } else {
        x >= blocker.x
            && x <= blocker.x + blocker.width
            && y + radius >= blocker.y
            && y - radius <= blocker.y + blocker.height
    }
}

fn blocker_contains_segment(
    blocker: &ParticleBlocker,
    start: (f32, f32),
    end: (f32, f32),
    thickness_radius: f32,
) -> bool {
    if blocker.polygon.len() >= 3 {
        polygon_intersects_segment(&blocker.polygon, start, end, thickness_radius)
    } else {
        let left = blocker.x - thickness_radius;
        let right = blocker.x + blocker.width + thickness_radius;
        let top = blocker.y - thickness_radius;
        let bottom = blocker.y + blocker.height + thickness_radius;
        point_in_rect(start, left, top, right, bottom)
            || point_in_rect(end, left, top, right, bottom)
            || segment_intersects_segment(start, end, (left, top), (right, top))
            || segment_intersects_segment(start, end, (right, top), (right, bottom))
            || segment_intersects_segment(start, end, (right, bottom), (left, bottom))
            || segment_intersects_segment(start, end, (left, bottom), (left, top))
    }
}

fn blocker_surface_y(blocker: &ParticleBlocker, x: f32) -> Option<f32> {
    if blocker.polygon.len() >= 3 {
        polygon_surface_y(&blocker.polygon, x)
    } else if x >= blocker.x && x <= blocker.x + blocker.width {
        Some(blocker.y)
    } else {
        None
    }
}

fn point_in_polygon(points: &[(f32, f32)], x: f32, y: f32) -> bool {
    let mut inside = false;
    let mut previous = points.len() - 1;
    for current in 0..points.len() {
        let (x1, y1) = points[current];
        let (x2, y2) = points[previous];
        let intersects = ((y1 > y) != (y2 > y))
            && (x < (x2 - x1) * (y - y1) / ((y2 - y1).abs().max(f32::EPSILON)) + x1);
        if intersects {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn polygon_intersects_circle(points: &[(f32, f32)], x: f32, y: f32, radius: f32) -> bool {
    if point_in_polygon(points, x, y) {
        return true;
    }
    let radius_sq = radius * radius;
    for index in 0..points.len() {
        let a = points[index];
        let b = points[(index + 1) % points.len()];
        if distance_sq_to_segment((x, y), a, b) <= radius_sq {
            return true;
        }
    }
    false
}

fn polygon_intersects_segment(
    points: &[(f32, f32)],
    start: (f32, f32),
    end: (f32, f32),
    thickness_radius: f32,
) -> bool {
    if point_in_polygon(points, start.0, start.1) || point_in_polygon(points, end.0, end.1) {
        return true;
    }
    let thickness_sq = thickness_radius * thickness_radius;
    for index in 0..points.len() {
        let a = points[index];
        let b = points[(index + 1) % points.len()];
        if segment_intersects_segment(start, end, a, b) {
            return true;
        }
        if distance_sq_between_segments(start, end, a, b) <= thickness_sq {
            return true;
        }
    }
    false
}

fn point_in_rect(point: (f32, f32), left: f32, top: f32, right: f32, bottom: f32) -> bool {
    point.0 >= left && point.0 <= right && point.1 >= top && point.1 <= bottom
}

fn orientation(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    ((b.1 - a.1) * (c.0 - b.0)) - ((b.0 - a.0) * (c.1 - b.1))
}

fn on_segment(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    b.0 >= a.0.min(c.0)
        && b.0 <= a.0.max(c.0)
        && b.1 >= a.1.min(c.1)
        && b.1 <= a.1.max(c.1)
}

fn segment_intersects_segment(
    p1: (f32, f32),
    q1: (f32, f32),
    p2: (f32, f32),
    q2: (f32, f32),
) -> bool {
    let o1 = orientation(p1, q1, p2);
    let o2 = orientation(p1, q1, q2);
    let o3 = orientation(p2, q2, p1);
    let o4 = orientation(p2, q2, q1);

    if ((o1 > 0.0 && o2 < 0.0) || (o1 < 0.0 && o2 > 0.0))
        && ((o3 > 0.0 && o4 < 0.0) || (o3 < 0.0 && o4 > 0.0))
    {
        return true;
    }

    if o1.abs() <= f32::EPSILON && on_segment(p1, p2, q1) {
        return true;
    }
    if o2.abs() <= f32::EPSILON && on_segment(p1, q2, q1) {
        return true;
    }
    if o3.abs() <= f32::EPSILON && on_segment(p2, p1, q2) {
        return true;
    }
    if o4.abs() <= f32::EPSILON && on_segment(p2, q1, q2) {
        return true;
    }

    false
}

fn distance_sq_between_segments(
    a1: (f32, f32),
    a2: (f32, f32),
    b1: (f32, f32),
    b2: (f32, f32),
) -> f32 {
    let d1 = distance_sq_to_segment(a1, b1, b2);
    let d2 = distance_sq_to_segment(a2, b1, b2);
    let d3 = distance_sq_to_segment(b1, a1, a2);
    let d4 = distance_sq_to_segment(b2, a1, a2);
    d1.min(d2).min(d3).min(d4)
}

fn distance_sq_to_segment(point: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let ab_x = b.0 - a.0;
    let ab_y = b.1 - a.1;
    let ap_x = point.0 - a.0;
    let ap_y = point.1 - a.1;
    let ab_len_sq = (ab_x * ab_x) + (ab_y * ab_y);
    if ab_len_sq <= f32::EPSILON {
        let dx = point.0 - a.0;
        let dy = point.1 - a.1;
        return (dx * dx) + (dy * dy);
    }
    let t = ((ap_x * ab_x) + (ap_y * ab_y)) / ab_len_sq;
    let t = t.clamp(0.0, 1.0);
    let closest_x = a.0 + (ab_x * t);
    let closest_y = a.1 + (ab_y * t);
    let dx = point.0 - closest_x;
    let dy = point.1 - closest_y;
    (dx * dx) + (dy * dy)
}

fn polygon_surface_y(points: &[(f32, f32)], x: f32) -> Option<f32> {
    let mut hits = Vec::new();
    for index in 0..points.len() {
        let (x1, y1) = points[index];
        let (x2, y2) = points[(index + 1) % points.len()];
        let min_x = x1.min(x2);
        let max_x = x1.max(x2);
        if x < min_x || x > max_x {
            continue;
        }
        if (x2 - x1).abs() <= f32::EPSILON {
            hits.push(y1.min(y2));
            continue;
        }
        let t = (x - x1) / (x2 - x1);
        if !(0.0..=1.0).contains(&t) {
            continue;
        }
        hits.push(y1 + ((y2 - y1) * t));
    }
    hits.into_iter().reduce(f32::min)
}

fn default_emitter_origin_x(preset: &SceneEmitterPreset) -> f32 {
    match preset {
        SceneEmitterPreset::Rain => 0.55,
        SceneEmitterPreset::Snow => 0.5,
        SceneEmitterPreset::Dust => 0.5,
        SceneEmitterPreset::Embers => 0.5,
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

fn resolve_emitter_min_speed(emitter: &backlayer_types::SceneEmitterNode) -> f32 {
    emitter.min_speed.unwrap_or(match emitter.preset {
        SceneEmitterPreset::Embers => 48.0,
        SceneEmitterPreset::Rain => 320.0,
        SceneEmitterPreset::Dust => 14.0,
        SceneEmitterPreset::Snow => 20.0,
    })
}

fn resolve_emitter_max_speed(emitter: &backlayer_types::SceneEmitterNode) -> f32 {
    emitter
        .max_speed
        .unwrap_or(match emitter.preset {
            SceneEmitterPreset::Embers => 110.0,
            SceneEmitterPreset::Rain => 620.0,
            SceneEmitterPreset::Dust => 42.0,
            SceneEmitterPreset::Snow => 58.0,
        })
        .max(resolve_emitter_min_speed(emitter))
}

fn resolve_emitter_min_life(emitter: &backlayer_types::SceneEmitterNode) -> f32 {
    emitter.min_life.unwrap_or(match emitter.preset {
        SceneEmitterPreset::Embers => 2.8,
        SceneEmitterPreset::Rain => 1.7,
        SceneEmitterPreset::Dust => 4.5,
        SceneEmitterPreset::Snow => 6.0,
    })
}

fn resolve_emitter_max_life(emitter: &backlayer_types::SceneEmitterNode) -> f32 {
    emitter
        .max_life
        .unwrap_or(match emitter.preset {
            SceneEmitterPreset::Embers => 5.0,
            SceneEmitterPreset::Rain => 2.5,
            SceneEmitterPreset::Dust => 9.0,
            SceneEmitterPreset::Snow => 9.0,
        })
        .max(resolve_emitter_min_life(emitter))
}

fn default_effect_color_hex(effect: &SceneEffectKind) -> &'static str {
    match effect {
        SceneEffectKind::Glow => "#ffc785",
        SceneEffectKind::Vignette => "#070d14",
        SceneEffectKind::Scanlines => "#ffd69b",
        SceneEffectKind::Fog => "#dbe8ff",
    }
}

fn emitter_origin_pixels(
    emitter: &backlayer_types::SceneEmitterNode,
    canvas_size: (u32, u32),
) -> (f32, f32) {
    let origin_x = emitter
        .origin_x
        .unwrap_or(default_emitter_origin_x(&emitter.preset))
        .clamp(0.0, 1.0);
    let origin_y = emitter
        .origin_y
        .unwrap_or(default_emitter_origin_y(&emitter.preset))
        .clamp(0.0, 1.0);
    (
        origin_x * canvas_size.0.max(1) as f32,
        origin_y * canvas_size.1.max(1) as f32,
    )
}

fn emitter_spawn_region(
    emitter: &backlayer_types::SceneEmitterNode,
    canvas_size: (u32, u32),
) -> (f32, f32) {
    (
        canvas_size.0 as f32
            * emitter
                .region_width
                .unwrap_or(default_emitter_region_width(&emitter.preset)),
        canvas_size.1 as f32
            * emitter
                .region_height
                .unwrap_or(default_emitter_region_height(&emitter.preset)),
    )
}

fn sample_emitter_position(
    emitter: &backlayer_types::SceneEmitterNode,
    canvas_size: (u32, u32),
    seed: &mut u64,
    origin_x: f32,
    origin_y: f32,
) -> (f32, f32) {
    let random = |seed: &mut u64| -> f32 { (next_u32(seed) as f32) / (u32::MAX as f32) };
    match emitter
        .shape
        .clone()
        .unwrap_or_else(|| default_emitter_shape(&emitter.preset))
    {
        SceneEmitterShape::Point => (origin_x, origin_y),
        SceneEmitterShape::Box => {
            let (spawn_width, spawn_height) = emitter_spawn_region(emitter, canvas_size);
            (
                origin_x + (random(seed) - 0.5) * spawn_width,
                origin_y + (random(seed) - 0.5) * spawn_height,
            )
        }
        SceneEmitterShape::Line => {
            let length = canvas_size.0 as f32
                * emitter
                    .line_length
                    .unwrap_or(default_emitter_line_length(&emitter.preset));
            let angle = emitter
                .line_angle_deg
                .unwrap_or(default_emitter_line_angle_deg(&emitter.preset))
                .to_radians();
            let offset = (random(seed) - 0.5) * length;
            (
                origin_x + angle.cos() * offset,
                origin_y + angle.sin() * offset,
            )
        }
        SceneEmitterShape::Circle => {
            let radius = emitter
                .region_radius
                .unwrap_or(default_emitter_region_radius(&emitter.preset))
                * canvas_size.0.min(canvas_size.1) as f32;
            let theta = random(seed) * std::f32::consts::TAU;
            let distance = random(seed).sqrt() * radius;
            (
                origin_x + theta.cos() * distance,
                origin_y + theta.sin() * distance,
            )
        }
    }
}

fn emitter_direction_radians(emitter: &backlayer_types::SceneEmitterNode) -> f32 {
    emitter
        .direction_deg
        .unwrap_or(default_emitter_direction_deg(&emitter.preset))
        .to_radians()
}

fn emitter_particle_rotation_radians(emitter: &backlayer_types::SceneEmitterNode) -> f32 {
    emitter.particle_rotation_deg.unwrap_or(0.0).to_radians()
}

fn rendered_particle_angle_radians(
    emitter: &backlayer_types::SceneEmitterNode,
    vx: f32,
    vy: f32,
) -> f32 {
    let rotation = emitter_particle_rotation_radians(emitter);
    if matches!(emitter.preset, SceneEmitterPreset::Rain) {
        vy.atan2(vx) - (std::f32::consts::FRAC_PI_2) + rotation
    } else {
        rotation
    }
}

fn parse_emitter_color(emitter: &backlayer_types::SceneEmitterNode) -> [f32; 3] {
    let value = emitter
        .color_hex
        .as_deref()
        .unwrap_or(default_emitter_color_hex(&emitter.preset))
        .trim();
    let value = value.strip_prefix('#').unwrap_or(value);
    if value.len() != 6 || !value.as_bytes().iter().all(|byte| byte.is_ascii_hexdigit()) {
        return parse_color_components(default_emitter_color_hex(&emitter.preset));
    }
    parse_color_components(value)
}

fn parse_effect_color(effect: &backlayer_types::SceneEffectNode) -> [f32; 3] {
    let value = effect
        .color_hex
        .as_deref()
        .unwrap_or(default_effect_color_hex(&effect.effect))
        .trim();
    let value = value.strip_prefix('#').unwrap_or(value);
    if value.len() != 6 || !value.as_bytes().iter().all(|byte| byte.is_ascii_hexdigit()) {
        return parse_color_components(default_effect_color_hex(&effect.effect));
    }
    parse_color_components(value)
}

fn parse_color_components(value: &str) -> [f32; 3] {
    let value = value.strip_prefix('#').unwrap_or(value);
    let parse = |range: std::ops::Range<usize>| -> f32 {
        u8::from_str_radix(&value[range], 16).unwrap_or(255) as f32 / 255.0
    };
    [parse(0..2), parse(2..4), parse(4..6)]
}

fn evaluate_scalar_curve(points: &[SceneCurvePoint], x: f32, fallback: f32) -> f32 {
    if points.is_empty() {
        return fallback;
    }
    let x = x.clamp(0.0, 1.0);
    if x <= points[0].x {
        return points[0].y;
    }
    for index in 1..points.len() {
        let left = &points[index - 1];
        let right = &points[index];
        if x <= right.x {
            let t = (x - left.x) / (right.x - left.x).max(0.0001);
            return left.y + ((right.y - left.y) * t);
        }
    }
    points.last().map(|point| point.y).unwrap_or(fallback)
}

fn evaluate_color_curve(points: &[SceneColorStop], x: f32, fallback: [f32; 3]) -> [f32; 3] {
    if points.is_empty() {
        return fallback;
    }
    let x = x.clamp(0.0, 1.0);
    if x <= points[0].x {
        return parse_color_components(&points[0].color_hex);
    }
    for index in 1..points.len() {
        let left = &points[index - 1];
        let right = &points[index];
        if x <= right.x {
            let t = (x - left.x) / (right.x - left.x).max(0.0001);
            let start = parse_color_components(&left.color_hex);
            let end = parse_color_components(&right.color_hex);
            return [
                start[0] + ((end[0] - start[0]) * t),
                start[1] + ((end[1] - start[1]) * t),
                start[2] + ((end[2] - start[2]) * t),
            ];
        }
    }
    points
        .last()
        .map(|point| parse_color_components(&point.color_hex))
        .unwrap_or(fallback)
}

fn create_rgba_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    image: &RgbaImage,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: image.width().max(1),
            height: image.height().max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        image.as_raw(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(image.width().max(1) * 4),
            rows_per_image: Some(image.height().max(1)),
        },
        wgpu::Extent3d {
            width: image.width().max(1),
            height: image.height().max(1),
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn create_sprite_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    blend: wgpu::BlendState,
    label: &'static str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[bind_group_layout],
                push_constant_ranges: &[],
            }),
        ),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(blend),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

fn scene_sprite_layout(
    canvas_size: (u32, u32),
    image: &RgbaImage,
    sprite: &backlayer_types::SceneSpriteNode,
    time_seconds: f32,
) -> (f32, f32, f32, f32, f32) {
    let mut x = sprite.x;
    let mut y = sprite.y;
    let mut scale = sprite.scale.max(0.1);
    let mut opacity = sprite.opacity.clamp(0.0, 1.0);
    for behavior in &sprite.behaviors {
        let phase = time_seconds * behavior.speed + behavior.phase;
        match behavior.kind {
            SceneBehaviorKind::Drift => {
                x += phase.sin() * behavior.amount_x;
                y += (phase * 0.85).cos() * behavior.amount_y;
            }
            SceneBehaviorKind::Pulse => {
                scale += phase.sin() * behavior.amount;
                opacity *= 0.9 + ((phase.sin() + 1.0) * 0.05);
            }
            SceneBehaviorKind::Orbit => {
                x += phase.cos() * behavior.amount;
                y += phase.sin() * behavior.amount_y.max(behavior.amount * 0.6);
            }
        }
    }
    let fit = sprite.fit.clone().unwrap_or(ImageFitMode::Cover);
    let (target_width, target_height, target_x, target_y) = sprite_layout(
        canvas_size.0,
        canvas_size.1,
        image.width(),
        image.height(),
        &fit,
        scale,
        x,
        y,
    );
    (
        target_width as f32,
        target_height as f32,
        target_x as f32,
        target_y as f32,
        opacity,
    )
}

fn effect_kind_to_u32(effect: &SceneEffectKind) -> u32 {
    match effect {
        SceneEffectKind::Glow => 0,
        SceneEffectKind::Vignette => 1,
        SceneEffectKind::Scanlines => 2,
        SceneEffectKind::Fog => 3,
    }
}

fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

fn build_particle_instances(scene: &NativeSceneRuntime) -> Vec<ParticleInstance> {
    let mut instances = Vec::new();
    let blockers = build_particle_blockers(scene);
    for emitter in scene.document.nodes.iter().filter_map(|node| match node {
        SceneNode::Emitter(emitter) if emitter.enabled => Some(emitter),
        _ => None,
    }) {
        let Some(state) = scene
            .emitters
            .iter()
            .find(|state| state.node_id == emitter.id)
        else {
            continue;
        };
        for particle in &state.particles {
            let progress = (particle.life / particle.max_life).clamp(0.0, 1.0);
            let life_t = 1.0 - progress;
            let alpha_curve = evaluate_scalar_curve(&emitter.alpha_curve, progress, life_t);
            let alpha = (particle.alpha * alpha_curve).clamp(0.0, 1.0);
            let size_curve = evaluate_scalar_curve(&emitter.size_curve, progress, 1.0);
            let color =
                evaluate_color_curve(&emitter.color_curve, progress, parse_emitter_color(emitter));
            let base_radius = (particle.size * size_curve).max(1.0);
            let render_angle = rendered_particle_angle_radians(emitter, particle.vx, particle.vy);
            let (size_x, size_y, angle, shape, alpha_scale) = match emitter.preset {
                SceneEmitterPreset::Rain => (
                    base_radius * 1.2,
                    base_radius * 8.5,
                    render_angle,
                    0.0,
                    0.92,
                ),
                SceneEmitterPreset::Snow => (
                    base_radius * 2.0,
                    base_radius * 2.0,
                    render_angle,
                    1.0,
                    0.86,
                ),
                SceneEmitterPreset::Dust => (
                    base_radius * 2.2,
                    base_radius * 2.2,
                    render_angle,
                    1.0,
                    0.7,
                ),
                SceneEmitterPreset::Embers => (
                    base_radius * 2.0,
                    base_radius * 2.0,
                    render_angle,
                    1.0,
                    1.0,
                ),
            };
            let occluded = if matches!(emitter.preset, SceneEmitterPreset::Rain) {
                let dx = angle.cos() * size_y * 0.5;
                let dy = angle.sin() * size_y * 0.5;
                particle_segment_is_occluded(
                    &blockers,
                    (particle.x - dx, particle.y - dy),
                    (particle.x + dx, particle.y + dy),
                    size_x.max(1.5) * 0.5,
                )
            } else {
                let occlusion_radius = size_x.max(size_y) * 0.5;
                particle_is_occluded(&blockers, particle.x, particle.y, occlusion_radius)
            };
            if occluded {
                continue;
            }
            instances.push(ParticleInstance {
                center_x: particle.x,
                center_y: particle.y,
                size_x,
                size_y,
                angle,
                shape,
                color_r: color[0],
                color_g: color[1],
                color_b: color[2],
                color_a: alpha * alpha_scale,
            });
        }
    }
    instances
}

fn build_debug_particle_area_instances(
    scene: &NativeSceneRuntime,
    surface_size: (u32, u32),
) -> Vec<ParticleInstance> {
    let mut instances = Vec::new();
    for area in scene.document.nodes.iter().filter_map(|node| match node {
        SceneNode::ParticleArea(area) if area.enabled => Some(area),
        _ => None,
    }) {
        let color = if area.occluder && area.surface {
            [0.98, 0.88, 0.28, 0.95]
        } else if area.occluder {
            [0.25, 0.95, 0.45, 0.95]
        } else if area.surface {
            [0.28, 0.72, 0.98, 0.95]
        } else {
            [0.95, 0.95, 0.95, 0.85]
        };
        if area.shape == Some(backlayer_types::SceneParticleAreaShape::Polygon) {
            let points = area
                .points
                .iter()
                .map(|point| {
                    (
                        point.x * surface_size.0 as f32,
                        point.y * surface_size.1 as f32,
                    )
                })
                .collect::<Vec<_>>();
            for window in points.windows(2) {
                push_debug_segment(&mut instances, window[0], window[1], color);
            }
            if let (Some(first), Some(last)) = (points.first().copied(), points.last().copied()) {
                push_debug_segment(&mut instances, last, first, color);
            }
        } else {
            let left = area.region.x * surface_size.0 as f32;
            let top = area.region.y * surface_size.1 as f32;
            let right = (area.region.x + area.region.width) * surface_size.0 as f32;
            let bottom = (area.region.y + area.region.height) * surface_size.1 as f32;
            push_debug_segment(&mut instances, (left, top), (right, top), color);
            push_debug_segment(&mut instances, (right, top), (right, bottom), color);
            push_debug_segment(&mut instances, (right, bottom), (left, bottom), color);
            push_debug_segment(&mut instances, (left, bottom), (left, top), color);
        }
    }
    instances
}

fn push_debug_segment(
    instances: &mut Vec<ParticleInstance>,
    from: (f32, f32),
    to: (f32, f32),
    color: [f32; 4],
) {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let length = (dx * dx + dy * dy).sqrt().max(1.0);
    instances.push(ParticleInstance {
        center_x: (from.0 + to.0) * 0.5,
        center_y: (from.1 + to.1) * 0.5,
        size_x: length,
        size_y: 3.0,
        angle: dy.atan2(dx),
        shape: 0.0,
        color_r: color[0],
        color_g: color[1],
        color_b: color[2],
        color_a: color[3],
    });
}

fn sprite_layout(
    canvas_width: u32,
    canvas_height: u32,
    image_width: u32,
    image_height: u32,
    fit: &ImageFitMode,
    scale: f32,
    offset_x: f32,
    offset_y: f32,
) -> (u32, u32, i64, i64) {
    let scaled_source_width = ((image_width as f32) * scale).round().max(1.0) as u32;
    let scaled_source_height = ((image_height as f32) * scale).round().max(1.0) as u32;
    let source_aspect = scaled_source_width as f32 / scaled_source_height as f32;
    let canvas_aspect = canvas_width as f32 / canvas_height as f32;

    let (base_width, base_height) = match fit {
        ImageFitMode::Contain => {
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
        ImageFitMode::Stretch => (canvas_width, canvas_height),
        ImageFitMode::Center => (scaled_source_width, scaled_source_height),
        ImageFitMode::Cover => {
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
        ImageFitMode::Center => (base_width, base_height),
        _ => (
            ((base_width as f32) * scale).round().max(1.0) as u32,
            ((base_height as f32) * scale).round().max(1.0) as u32,
        ),
    };

    let x = ((canvas_width as i64 - target_width as i64) / 2) + offset_x.round() as i64;
    let y = ((canvas_height as i64 - target_height as i64) / 2) + offset_y.round() as i64;
    (target_width, target_height, x, y)
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

#[derive(Debug, Clone)]
struct SceneLayer {
    image: PathBuf,
    x: i64,
    y: i64,
    width: Option<u32>,
    height: Option<u32>,
    scale: Option<f32>,
    opacity: f32,
}

#[derive(Debug, Clone)]
struct ScenePackage {
    entries: HashMap<String, Vec<u8>>,
}

impl ScenePackage {
    fn parse(path: &Path) -> Result<Self> {
        let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
        if bytes.len() < 16 || &bytes[4..12] != b"PKGV0001" {
            return Err(anyhow!(
                "{} is not a supported scene.pkg bundle",
                path.display()
            ));
        }

        let mut cursor = 16usize;
        let mut metadata = Vec::new();
        while cursor + 12 <= bytes.len() {
            let name_len = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
            let name_len = usize::try_from(name_len).unwrap_or(0);
            if !(1..=260).contains(&name_len) || cursor + 12 + name_len > bytes.len() {
                break;
            }

            let name_bytes = &bytes[cursor + 4..cursor + 4 + name_len];
            if !name_bytes.iter().all(|byte| {
                matches!(
                    byte,
                    b'a'..=b'z'
                        | b'A'..=b'Z'
                        | b'0'..=b'9'
                        | b'/'
                        | b'.'
                        | b'_'
                        | b'-'
                )
            }) {
                break;
            }

            let name = String::from_utf8(name_bytes.to_vec())
                .context("scene.pkg entry name was not valid UTF-8")?;
            let offset = u32::from_le_bytes(
                bytes[cursor + 4 + name_len..cursor + 8 + name_len]
                    .try_into()
                    .unwrap(),
            );
            let size = u32::from_le_bytes(
                bytes[cursor + 8 + name_len..cursor + 12 + name_len]
                    .try_into()
                    .unwrap(),
            );
            metadata.push((name, offset as usize, size as usize));
            cursor += 12 + name_len;
        }

        if metadata.is_empty() {
            return Err(anyhow!(
                "{} did not contain a readable package index",
                path.display()
            ));
        }

        let payload_base = cursor;
        let mut entries = HashMap::new();
        for (name, offset, size) in metadata {
            let start = payload_base.saturating_add(offset);
            let end = start.saturating_add(size);
            if end > bytes.len() {
                continue;
            }
            entries.insert(name, bytes[start..end].to_vec());
        }

        Ok(Self { entries })
    }

    fn json(&self, name: &str) -> Option<Value> {
        let bytes = self.entries.get(name)?;
        serde_json::from_slice(bytes).ok()
    }

    fn find_by_stem(&self, stem: &str, extensions: &[&str]) -> Option<(&str, &[u8])> {
        self.entries.iter().find_map(|(name, bytes)| {
            let path = Path::new(name);
            let entry_stem = path.file_stem()?.to_str()?;
            let extension = path.extension()?.to_str()?;
            (entry_stem.eq_ignore_ascii_case(stem)
                && extensions
                    .iter()
                    .any(|candidate| extension.eq_ignore_ascii_case(candidate)))
            .then_some((name.as_str(), bytes.as_slice()))
        })
    }
}

fn compose_scene_pkg_target(entrypoint: &Path) -> Result<Option<PathBuf>> {
    let package = ScenePackage::parse(entrypoint)?;
    let Some(scene) = package.json("scene.json") else {
        debug!(source = %entrypoint.display(), "scene pkg did not contain scene.json");
        return Ok(None);
    };

    let layers = extract_scene_pkg_layers(&scene, &package)?;
    debug!(source = %entrypoint.display(), layer_count = layers.len(), "scene pkg layers resolved");
    if layers.len() < 2 {
        return Ok(None);
    }

    let canvas = compose_layers(&layers, canvas_size_from_scene(&scene, &layers));
    Ok(Some(write_scene_png(canvas)?))
}

fn extract_scene_pkg_image_target(entrypoint: &Path) -> Result<Option<PathBuf>> {
    let package = ScenePackage::parse(entrypoint)?;
    let Some(scene) = package.json("scene.json") else {
        return Ok(None);
    };

    let mut layers = extract_scene_pkg_layers(&scene, &package)?;
    if let Some(layer) = layers.drain(..).next() {
        return Ok(Some(layer.image));
    }

    for (name, bytes) in package.entries.iter() {
        if is_image_name(name) {
            if let Ok(path) = persist_image_bytes_to_png(bytes, name) {
                return Ok(Some(path));
            }
        }
        if name.ends_with(".tex") {
            if let Some(extracted) = extract_embedded_image_bytes(bytes) {
                if let Ok(path) = persist_image_bytes_to_png(&extracted, name) {
                    return Ok(Some(path));
                }
            }
            if let Some(path) = decode_tex_to_png(bytes, name)? {
                return Ok(Some(path));
            }
        }
    }

    Ok(None)
}

fn extract_scene_pkg_layers(scene: &Value, package: &ScenePackage) -> Result<Vec<SceneLayer>> {
    let Some(objects) = scene.get("objects").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };

    let mut layers = Vec::new();
    for object in objects {
        if object.get("visible").and_then(Value::as_bool) == Some(false) {
            debug!("skipping hidden scene object");
            continue;
        }

        let Some(model_path) = object.get("image").and_then(Value::as_str) else {
            debug!(name = ?object.get("name"), "scene object has no model image");
            continue;
        };
        let Some(model) = package.json(model_path) else {
            debug!(%model_path, "scene model json missing from package");
            continue;
        };
        let Some(material_path) = model.get("material").and_then(Value::as_str) else {
            debug!(%model_path, "scene model had no material reference");
            continue;
        };
        let Some(material) = package.json(material_path) else {
            debug!(%material_path, "scene material json missing from package");
            continue;
        };
        let Some(texture_name) = material
            .get("passes")
            .and_then(Value::as_array)
            .and_then(|passes| passes.first())
            .and_then(|pass| pass.get("textures"))
            .and_then(Value::as_array)
            .and_then(|textures| textures.first())
            .and_then(Value::as_str)
        else {
            debug!(%material_path, "scene material had no texture reference");
            continue;
        };

        let Some(image_path) = resolve_scene_pkg_texture(package, texture_name)? else {
            debug!(%texture_name, "scene texture could not be resolved");
            continue;
        };

        debug!(
            name = ?object.get("name"),
            %model_path,
            %material_path,
            %texture_name,
            image = %image_path.display(),
            "scene layer resolved"
        );

        let size = parse_vec3(
            object
                .get("size")
                .and_then(Value::as_str)
                .unwrap_or("0 0 0"),
        );
        let origin = parse_vec3(
            object
                .get("origin")
                .and_then(Value::as_str)
                .unwrap_or("0 0 0"),
        );
        let scale = parse_vec3(
            object
                .get("scale")
                .and_then(Value::as_str)
                .unwrap_or("1 1 1"),
        );

        let width = (size.0 * scale.0).round().max(1.0) as u32;
        let height = (size.1 * scale.1).round().max(1.0) as u32;
        let x = (origin.0 - (width as f32 / 2.0)).round() as i64;
        let y = (origin.1 - (height as f32 / 2.0)).round() as i64;

        layers.push(SceneLayer {
            image: image_path,
            x,
            y,
            width: Some(width),
            height: Some(height),
            scale: None,
            opacity: 1.0,
        });
    }

    Ok(layers)
}

fn resolve_scene_pkg_texture(
    package: &ScenePackage,
    texture_name: &str,
) -> Result<Option<PathBuf>> {
    if let Some((name, bytes)) =
        package.find_by_stem(texture_name, &["png", "jpg", "jpeg", "webp", "gif"])
    {
        return persist_image_bytes_to_png(bytes, name).map(Some);
    }
    if let Some((name, bytes)) = package.find_by_stem(texture_name, &["tex"]) {
        if let Some(extracted) = extract_embedded_image_bytes(bytes) {
            if let Ok(path) = persist_image_bytes_to_png(&extracted, name) {
                return Ok(Some(path));
            }
        }
        if let Some(path) = decode_tex_to_png(bytes, name)? {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

#[derive(Debug, Clone, Copy)]
struct TexMetadata {
    encoded_width: u32,
    encoded_height: u32,
    width: u32,
    height: u32,
}

fn decode_tex_to_png(bytes: &[u8], key: &str) -> Result<Option<PathBuf>> {
    let Some(metadata) = parse_tex_metadata(bytes) else {
        return Ok(None);
    };
    let Some(texb_offset) = bytes.windows(9).position(|chunk| chunk == b"TEXB0002\0") else {
        return Ok(None);
    };

    let data_offsets = [
        texb_offset + 32,
        texb_offset + 36,
        texb_offset + 40,
        texb_offset + 44,
        texb_offset + 48,
    ];
    let dimension_candidates = [
        (metadata.width, metadata.height),
        (metadata.encoded_width, metadata.encoded_height),
    ];
    let format_candidates = [
        DdsImageFormat::BC1RgbaUnorm,
        DdsImageFormat::BC3RgbaUnorm,
        DdsImageFormat::BC7RgbaUnorm,
    ];

    for (width, height) in dimension_candidates {
        if width == 0 || height == 0 {
            continue;
        }

        for data_offset in data_offsets {
            if data_offset >= bytes.len() {
                continue;
            }

            for image_format in format_candidates {
                let surface = Surface {
                    width,
                    height,
                    depth: 1,
                    layers: 1,
                    mipmaps: 1,
                    image_format,
                    data: &bytes[data_offset..],
                };

                let Ok(decoded) = surface.decode_rgba8() else {
                    debug!(
                        texture = key,
                        width,
                        height,
                        data_offset,
                        format = ?image_format,
                        "tex decode failed"
                    );
                    continue;
                };
                let Ok(image) = decoded.into_image() else {
                    debug!(
                        texture = key,
                        width,
                        height,
                        data_offset,
                        format = ?image_format,
                        "tex image conversion failed"
                    );
                    continue;
                };
                let path = temp_image_path(key);
                image
                    .save(&path)
                    .with_context(|| format!("failed to write {}", path.display()))?;
                debug!(
                    texture = key,
                    width,
                    height,
                    data_offset,
                    format = ?image_format,
                    path = %path.display(),
                    "tex decode succeeded"
                );
                return Ok(Some(path));
            }
        }
    }

    Ok(None)
}

fn parse_tex_metadata(bytes: &[u8]) -> Option<TexMetadata> {
    let texi_offset = bytes.windows(9).position(|chunk| chunk == b"TEXI0001\0")?;
    let field_offset = texi_offset + 9;
    if field_offset + 24 > bytes.len() {
        return None;
    }

    let read_u32 = |offset: usize| -> Option<u32> {
        Some(u32::from_le_bytes(
            bytes.get(offset..offset + 4)?.try_into().ok()?,
        ))
    };

    Some(TexMetadata {
        encoded_width: read_u32(field_offset + 8)?,
        encoded_height: read_u32(field_offset + 12)?,
        width: read_u32(field_offset + 16)?,
        height: read_u32(field_offset + 20)?,
    })
}

fn persist_image_bytes_to_png(bytes: &[u8], key: &str) -> Result<PathBuf> {
    let image = image::load_from_memory(bytes)
        .with_context(|| format!("failed to decode image bytes for {key}"))?;
    let path = temp_image_path(key);
    image
        .save(&path)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path)
}

fn temp_image_path(key: &str) -> PathBuf {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    let hash = hasher.finish();
    std::env::temp_dir().join(format!(
        "backlayer-scene-extract-{}-{hash:x}.png",
        std::process::id()
    ))
}

fn extract_embedded_image_bytes(bytes: &[u8]) -> Option<Vec<u8>> {
    extract_png(bytes)
        .or_else(|| extract_jpeg(bytes))
        .or_else(|| extract_gif(bytes))
        .or_else(|| extract_webp(bytes))
}

fn extract_png(bytes: &[u8]) -> Option<Vec<u8>> {
    let start = bytes
        .windows(8)
        .position(|chunk| chunk == b"\x89PNG\r\n\x1a\n")?;
    let end_marker = bytes[start..]
        .windows(8)
        .position(|chunk| chunk == b"IEND\xaeB`\x82")?;
    let end = start + end_marker + 8;
    Some(bytes[start..end].to_vec())
}

fn extract_jpeg(bytes: &[u8]) -> Option<Vec<u8>> {
    let start = bytes
        .windows(3)
        .position(|chunk| chunk == [0xFF, 0xD8, 0xFF])?;
    let end_marker = bytes[start..]
        .windows(2)
        .rposition(|chunk| chunk == [0xFF, 0xD9])?;
    let end = start + end_marker + 2;
    Some(bytes[start..end].to_vec())
}

fn extract_gif(bytes: &[u8]) -> Option<Vec<u8>> {
    let start = bytes
        .windows(6)
        .position(|chunk| chunk == b"GIF87a" || chunk == b"GIF89a")?;
    let slice = bytes[start..].to_vec();
    image::load_from_memory(&slice).ok()?;
    Some(slice)
}

fn extract_webp(bytes: &[u8]) -> Option<Vec<u8>> {
    for (index, chunk) in bytes.windows(12).enumerate() {
        if &chunk[0..4] == b"RIFF" && &chunk[8..12] == b"WEBP" {
            let size = u32::from_le_bytes(chunk[4..8].try_into().ok()?);
            let total = 8usize.saturating_add(size as usize);
            if index + total <= bytes.len() {
                let slice = bytes[index..index + total].to_vec();
                image::load_from_memory(&slice).ok()?;
                return Some(slice);
            }
        }
    }
    None
}

fn parse_vec3(value: &str) -> (f32, f32, f32) {
    let mut parts = value
        .split_whitespace()
        .filter_map(|part| part.parse::<f32>().ok());
    (
        parts.next().unwrap_or(0.0),
        parts.next().unwrap_or(0.0),
        parts.next().unwrap_or(0.0),
    )
}

fn is_image_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".webp")
        || lower.ends_with(".gif")
}

fn extract_scene_layers(value: &Value, root: &Path) -> Vec<SceneLayer> {
    let Some(items) = value.get("layers").and_then(Value::as_array) else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(|item| parse_scene_layer(item, root))
        .collect()
}

fn parse_scene_layer(value: &Value, root: &Path) -> Option<SceneLayer> {
    let object = value.as_object()?;
    let image = object
        .get("image")
        .and_then(Value::as_str)
        .or_else(|| object.get("texture").and_then(Value::as_str))
        .or_else(|| object.get("file").and_then(Value::as_str))
        .and_then(|candidate| resolve_image_candidate(root, candidate))?;
    Some(SceneLayer {
        image,
        x: object.get("x").and_then(number_to_i64).unwrap_or(0),
        y: object.get("y").and_then(number_to_i64).unwrap_or(0),
        width: object.get("width").and_then(number_to_u32),
        height: object.get("height").and_then(number_to_u32),
        scale: object.get("scale").and_then(number_to_f32),
        opacity: object
            .get("opacity")
            .or_else(|| object.get("alpha"))
            .and_then(number_to_f32)
            .unwrap_or(1.0)
            .clamp(0.0, 1.0),
    })
}

fn number_to_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
}

fn number_to_u32(value: &Value) -> Option<u32> {
    value
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .or_else(|| value.as_i64().and_then(|value| u32::try_from(value).ok()))
}

fn number_to_f32(value: &Value) -> Option<f32> {
    value.as_f64().map(|value| value as f32)
}

fn find_image_in_json(value: &Value, root: &Path) -> Option<PathBuf> {
    match value {
        Value::String(text) => resolve_image_candidate(root, text),
        Value::Array(items) => items.iter().find_map(|item| find_image_in_json(item, root)),
        Value::Object(map) => map.values().find_map(|item| find_image_in_json(item, root)),
        _ => None,
    }
}

fn resolve_image_candidate(root: &Path, candidate: &str) -> Option<PathBuf> {
    let lower = candidate.to_ascii_lowercase();
    if !(lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".webp")
        || lower.ends_with(".gif"))
    {
        return None;
    }

    let path = root.join(candidate);
    path.is_file().then_some(path)
}

fn find_first_sibling_image(entrypoint: &Path) -> Option<PathBuf> {
    let root = entrypoint.parent()?;
    let mut images = Vec::new();

    for entry in fs::read_dir(root).ok()? {
        let path = entry.ok()?.path();
        if !path.is_file() || path == entrypoint {
            continue;
        }

        let Some(extension) = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
        else {
            continue;
        };

        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let stem = stem.to_ascii_lowercase();
        if ["preview", "thumbnail", "thumb", "screenshot"].contains(&stem.as_str()) {
            continue;
        }

        if ["png", "jpg", "jpeg", "webp", "gif"].contains(&extension.as_str()) {
            images.push(path);
        }
    }

    images.sort();
    images.into_iter().next()
}

fn canvas_size(value: &Value, layers: &[SceneLayer]) -> (u32, u32) {
    let explicit_width = value.get("width").and_then(number_to_u32);
    let explicit_height = value.get("height").and_then(number_to_u32);
    if let (Some(width), Some(height)) = (explicit_width, explicit_height) {
        return (width.max(1), height.max(1));
    }

    let mut max_width = 1u32;
    let mut max_height = 1u32;

    for layer in layers {
        if let Ok(image) = image::open(&layer.image) {
            let (width, height) = scaled_dimensions(&image, layer);
            let right = (layer.x.max(0) as u32).saturating_add(width);
            let bottom = (layer.y.max(0) as u32).saturating_add(height);
            max_width = max_width.max(right);
            max_height = max_height.max(bottom);
        }
    }

    (max_width.max(1), max_height.max(1))
}

fn canvas_size_from_scene(value: &Value, layers: &[SceneLayer]) -> (u32, u32) {
    if let Some(projection) = value
        .get("general")
        .and_then(|general| general.get("orthogonalprojection"))
    {
        let explicit_width = projection.get("width").and_then(number_to_u32);
        let explicit_height = projection.get("height").and_then(number_to_u32);
        if let (Some(width), Some(height)) = (explicit_width, explicit_height) {
            return (width.max(1), height.max(1));
        }
    }

    canvas_size(value, layers)
}

fn compose_layers(layers: &[SceneLayer], canvas_size: (u32, u32)) -> RgbaImage {
    let mut canvas = RgbaImage::from_pixel(canvas_size.0, canvas_size.1, Rgba([0, 0, 0, 0]));

    for layer in layers {
        let Ok(image) = image::open(&layer.image) else {
            continue;
        };
        let mut rgba = image.to_rgba8();
        let (target_width, target_height) = scaled_dimensions(&image, layer);
        if rgba.width() != target_width || rgba.height() != target_height {
            rgba = imageops::resize(
                &rgba,
                target_width.max(1),
                target_height.max(1),
                imageops::FilterType::Triangle,
            );
        }
        if layer.opacity < 1.0 {
            for pixel in rgba.pixels_mut() {
                pixel.0[3] = ((pixel.0[3] as f32) * layer.opacity)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
        }

        imageops::overlay(&mut canvas, &rgba, layer.x, layer.y);
    }

    canvas
}

fn scaled_dimensions(image: &DynamicImage, layer: &SceneLayer) -> (u32, u32) {
    let base_width = image.width();
    let base_height = image.height();

    match (layer.width, layer.height, layer.scale) {
        (Some(width), Some(height), _) => (width.max(1), height.max(1)),
        (Some(width), None, _) => {
            let ratio = width as f32 / base_width.max(1) as f32;
            (
                width.max(1),
                ((base_height as f32) * ratio).round().max(1.0) as u32,
            )
        }
        (None, Some(height), _) => {
            let ratio = height as f32 / base_height.max(1) as f32;
            (
                ((base_width as f32) * ratio).round().max(1.0) as u32,
                height.max(1),
            )
        }
        (None, None, Some(scale)) => (
            ((base_width as f32) * scale).round().max(1.0) as u32,
            ((base_height as f32) * scale).round().max(1.0) as u32,
        ),
        _ => (base_width.max(1), base_height.max(1)),
    }
}

fn write_scene_png(image: RgbaImage) -> Result<PathBuf> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    image.as_raw().hash(&mut hasher);
    let hash = hasher.finish();
    let path = std::env::temp_dir().join(format!(
        "backlayer-scene-composite-{}-{hash:x}.png",
        std::process::id()
    ));
    image
        .save(&path)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use backlayer_types::{SceneEffectKind, SceneEmitterPreset, SceneNode};
    use image::{Rgba, RgbaImage};

    use super::{
        compose_scene_target, extract_scene_image_target, find_first_sibling_image,
        load_native_scene_runtime, parse_tex_metadata, resolve_runtime_target,
    };

    #[test]
    fn resolves_first_scene_json_image_reference() {
        let root = temp_root("json-image");
        fs::write(
            root.join("scene.json"),
            r#"{"layers":[{"image":"bg.png"}]}"#,
        )
        .expect("scene json should write");
        fs::write(root.join("bg.png"), b"png").expect("image should write");

        let resolved = extract_scene_image_target(&root.join("scene.json"))
            .expect("extract should succeed")
            .expect("scene image should resolve");
        assert_eq!(resolved, root.join("bg.png"));

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn resolves_first_sibling_image_when_scene_json_is_not_present() {
        let root = temp_root("sibling-image");
        let pkg = root.join("scene.pkg");
        fs::write(&pkg, b"pkg").expect("scene pkg should write");
        fs::write(root.join("a-preview.png"), b"png").expect("image should write");

        let resolved = find_first_sibling_image(&pkg).expect("sibling image should resolve");
        assert_eq!(resolved, root.join("a-preview.png"));

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn preview_fallback_is_used_when_no_scene_image_exists() {
        let root = temp_root("preview");
        let pkg = root.join("scene.pkg");
        let preview = root.join("preview.png");
        fs::write(&pkg, b"pkg").expect("scene pkg should write");
        fs::write(&preview, b"png").expect("preview should write");

        let resolved =
            resolve_runtime_target(&pkg, Some(&preview)).expect("resolution should succeed");
        assert_eq!(resolved.mode, "preview_fallback");
        assert_eq!(resolved.path, preview);

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn composes_multiple_scene_layers_into_a_single_image() {
        let root = temp_root("layers");
        fs::write(
            root.join("scene.json"),
            r##"{
              "width": 4,
              "height": 4,
              "layers": [
                { "image": "bg.png", "x": 0, "y": 0 },
                { "image": "fg.png", "x": 1, "y": 1, "opacity": 1.0 }
              ]
            }"##,
        )
        .expect("scene json should write");

        let bg = RgbaImage::from_pixel(4, 4, Rgba([255, 0, 0, 255]));
        bg.save(root.join("bg.png")).expect("bg should write");
        let fg = RgbaImage::from_pixel(2, 2, Rgba([0, 255, 0, 255]));
        fg.save(root.join("fg.png")).expect("fg should write");

        let composited =
            compose_scene_target(&root.join("scene.json")).expect("composition should succeed");
        let composited = composited.expect("scene should compose");
        let image = image::open(&composited)
            .expect("image should open")
            .to_rgba8();

        assert_eq!(image.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(image.get_pixel(1, 1).0, [0, 255, 0, 255]);

        fs::remove_dir_all(root).ok();
        fs::remove_file(composited).ok();
    }

    #[test]
    fn composes_scene_pkg_layers_into_a_single_image() {
        let root = temp_root("pkg-layers");
        let pkg = root.join("scene.pkg");

        let base = RgbaImage::from_pixel(4, 4, Rgba([255, 0, 0, 255]));
        let front = RgbaImage::from_pixel(2, 2, Rgba([0, 255, 0, 255]));
        let mut base_png = Vec::new();
        let mut front_png = Vec::new();
        image::DynamicImage::ImageRgba8(base)
            .write_to(
                &mut std::io::Cursor::new(&mut base_png),
                image::ImageFormat::Png,
            )
            .expect("base png should encode");
        image::DynamicImage::ImageRgba8(front)
            .write_to(
                &mut std::io::Cursor::new(&mut front_png),
                image::ImageFormat::Png,
            )
            .expect("front png should encode");

        let pkg_bytes = build_scene_pkg(&[
            (
                "scene.json",
                br#"{
                  "general": { "orthogonalprojection": { "width": 4, "height": 4 } },
                  "objects": [
                    { "image": "models/bg.json", "origin": "2 2 0", "size": "4 4 0", "scale": "1 1 1", "visible": true },
                    { "image": "models/fg.json", "origin": "2 2 0", "size": "2 2 0", "scale": "1 1 1", "visible": true }
                  ]
                }"#.as_slice(),
            ),
            ("models/bg.json", br#"{ "material": "materials/bg.json" }"#.as_slice()),
            ("materials/bg.json", br#"{ "passes": [{ "textures": ["bg"] }] }"#.as_slice()),
            ("materials/bg.tex", wrapped_tex_bytes(&base_png).as_slice()),
            ("models/fg.json", br#"{ "material": "materials/fg.json" }"#.as_slice()),
            ("materials/fg.json", br#"{ "passes": [{ "textures": ["fg"] }] }"#.as_slice()),
            ("materials/fg.tex", wrapped_tex_bytes(&front_png).as_slice()),
        ]);
        fs::write(&pkg, pkg_bytes).expect("scene pkg should write");

        let composited = compose_scene_target(&pkg).expect("composition should succeed");
        let composited = composited.expect("scene pkg should compose");
        let image = image::open(&composited)
            .expect("image should open")
            .to_rgba8();

        assert_eq!(image.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(image.get_pixel(1, 1).0, [0, 255, 0, 255]);

        fs::remove_dir_all(root).ok();
        fs::remove_file(composited).ok();
    }

    #[test]
    fn loads_native_scene_v2_runtime() {
        let root = temp_root("native-scene-v2");
        let base = RgbaImage::from_pixel(16, 16, Rgba([255, 0, 0, 255]));
        base.save(root.join("base.png")).expect("base should write");
        let overlay = RgbaImage::from_pixel(8, 8, Rgba([0, 0, 255, 180]));
        overlay
            .save(root.join("overlay.png"))
            .expect("overlay should write");
        fs::write(
            root.join("scene.json"),
            r##"{
              "schema":"backlayer_scene_v2",
              "version":2,
              "width":16,
              "height":16,
              "images":[
                {"key":"base","path":"base.png"},
                {"key":"overlay","path":"overlay.png"}
              ],
              "nodes":[
                {"kind":"sprite","id":"sprite-base","name":"Base","enabled":true,"image_key":"base","fit":"cover","blend":"alpha","x":0.0,"y":0.0,"scale":1.0,"rotation_deg":0.0,"opacity":1.0,"behaviors":[]},
                {"kind":"sprite","id":"sprite-overlay","name":"Overlay","enabled":true,"image_key":"overlay","fit":"contain","blend":"screen","x":2.0,"y":-2.0,"scale":0.8,"rotation_deg":0.0,"opacity":0.7,"behaviors":[{"kind":"pulse","speed":1.2,"amount_x":0.0,"amount_y":0.0,"amount":0.08,"phase":0.0}]},
                {"kind":"effect","id":"effect-fog","name":"Fog","enabled":true,"effect":"fog","color_hex":"#dbe8ff","opacity":0.35,"intensity":0.8,"speed":0.6},
                {"kind":"emitter","id":"emit-snow","name":"Snow","enabled":true,"preset":"snow","emission_rate":24.0,"max_particles":32,"opacity":0.6,"size":3.0,"speed":80.0,"spread":40.0,"gravity_x":0.0,"gravity_y":10.0,"drag":0.2}
              ]
            }"##,
        )
        .expect("scene json should write");

        let runtime = load_native_scene_runtime(&root.join("scene.json"))
            .expect("native scene load should succeed")
            .expect("native scene should load");
        assert_eq!(runtime.canvas_size, (16, 16));
        assert_eq!(runtime.document.schema, "backlayer_scene_v2");
        assert_eq!(runtime.images.len(), 2);
        assert_eq!(runtime.emitters.len(), 1);
        assert!(runtime.images.contains_key("overlay"));
        assert!(runtime.document.nodes.iter().any(|node| matches!(
            node,
            SceneNode::Effect(effect) if effect.effect == SceneEffectKind::Fog
        )));
        assert!(runtime.document.nodes.iter().any(|node| matches!(
            node,
            SceneNode::Emitter(emitter) if emitter.preset == SceneEmitterPreset::Snow
        )));

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn parses_basic_tex_metadata() {
        let bytes = wrapped_tex_bytes(&[1, 2, 3, 4]);
        let metadata = parse_tex_metadata(&bytes).expect("metadata should parse");

        assert_eq!(metadata.encoded_width, 4);
        assert_eq!(metadata.encoded_height, 4);
        assert_eq!(metadata.width, 4);
        assert_eq!(metadata.height, 4);
    }

    fn temp_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "backlayer-scene-runner-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp root should exist");
        root
    }

    fn build_scene_pkg(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut header = Vec::new();
        header.extend_from_slice(&8u32.to_le_bytes());
        header.extend_from_slice(b"PKGV0001");
        header.extend_from_slice(&16u32.to_le_bytes());

        let mut offset = 0u32;
        for (name, data) in entries {
            header.extend_from_slice(&(name.len() as u32).to_le_bytes());
            header.extend_from_slice(name.as_bytes());
            header.extend_from_slice(&offset.to_le_bytes());
            header.extend_from_slice(&(data.len() as u32).to_le_bytes());
            offset = offset.saturating_add(data.len() as u32);
        }

        let mut bytes = header;
        for (_, data) in entries {
            bytes.extend_from_slice(data);
        }
        bytes
    }

    fn wrapped_tex_bytes(png: &[u8]) -> Vec<u8> {
        let mut bytes = b"TEXV0005\0TEXI0001\0".to_vec();
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(b"TEXB0002\0");
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(png);
        bytes
    }
}
