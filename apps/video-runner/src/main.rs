use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Child, ChildStdout, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow};
use backlayer_hyprland::HyprlandClient;
use backlayer_wayland::{LayerShellRuntime, LayerSurfaceSession};
use pollster::block_on;
use tracing::info;

const POLICY_CHECK_INTERVAL: Duration = Duration::from_millis(250);

const VIDEO_SHADER: &str = r#"
struct VideoUniforms {
  surface_width: f32,
  surface_height: f32,
  frame_width: f32,
  frame_height: f32,
  fit_mode: u32,
  _padding0: u32,
  _padding1: u32,
  _padding2: u32,
}

@group(0) @binding(0)
var video_texture: texture_2d<f32>;

@group(0) @binding(1)
var video_sampler: sampler;

@group(0) @binding(2)
var<uniform> video_uniforms: VideoUniforms;

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
  let surface = vec2<f32>(max(video_uniforms.surface_width, 1.0), max(video_uniforms.surface_height, 1.0));
  let source = vec2<f32>(max(video_uniforms.frame_width, 1.0), max(video_uniforms.frame_height, 1.0));

  var drawn = source;
  let scale = max(surface.x / source.x, surface.y / source.y);
  drawn = source * scale;

  let offset = (surface - drawn) * 0.5;
  let uv = (in.position.xy - offset) / drawn;
  if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
  }

  return textureSample(video_texture, video_sampler, uv);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct VideoUniforms {
    surface_width: f32,
    surface_height: f32,
    frame_width: f32,
    frame_height: f32,
    fit_mode: u32,
    _padding0: u32,
    _padding1: u32,
    _padding2: u32,
}

struct DecodedFrame {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

struct VideoFrameStream {
    child: Child,
    stdout: ChildStdout,
    width: u32,
    height: u32,
    frame_size: usize,
}

struct VideoSurfaceRuntime {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform_buffer: wgpu::Buffer,
    config: wgpu::SurfaceConfiguration,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("video_runner=info,backlayer=info")
        .compact()
        .init();

    let output_name = std::env::args()
        .nth(1)
        .context("missing output name argument")?;
    let fps = std::env::args()
        .nth(2)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(30)
        .max(1);
    let pause_on_fullscreen = std::env::args().nth(3).as_deref() == Some("1");
    let pause_on_battery = std::env::args().nth(4).as_deref() == Some("1");
    let asset_id = std::env::args()
        .nth(5)
        .unwrap_or_else(|| "video-runner".to_string());
    let original_entrypoint = std::env::args()
        .nth(6)
        .map(PathBuf::from)
        .context("missing original video entrypoint")?;

    let runtime = LayerShellRuntime::new();
    let mut session = runtime
        .start_session_on_output(Some(&output_name))
        .with_context(|| format!("failed to start layer-shell session for {output_name}"))?;
    let mut video_surface = VideoSurfaceRuntime::new(&session)?;
    let mut stream = VideoFrameStream::open(&original_entrypoint)?;

    info!(
        output = %output_name,
        fps,
        pause_on_fullscreen,
        pause_on_battery,
        asset_id = %asset_id,
        source = %original_entrypoint.display(),
        detail = %"ffmpeg cli decode video playback active",
        "video runner started"
    );

    let frame_interval = Duration::from_millis((1000 / fps).max(1));
    let hyprland = HyprlandClient::new();
    let power = PowerStateProbe::default();
    let mut next_frame_at = Instant::now();

    loop {
        let now = Instant::now();
        session
            .dispatch_pending()
            .map_err(|error| anyhow!("wayland dispatch failed: {error}"))?;

        let paused_for_fullscreen =
            pause_on_fullscreen && hyprland.fullscreen_active().unwrap_or(false);
        let paused_for_battery = pause_on_battery && power.on_battery().unwrap_or(false);
        let paused = paused_for_fullscreen || paused_for_battery;

        if !paused && now >= next_frame_at {
            let frame = stream.next_frame().with_context(|| {
                format!(
                    "video decode stream failed for {}",
                    original_entrypoint.display()
                )
            })?;
            video_surface.render_frame(&frame)?;
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

impl VideoFrameStream {
    fn open(path: &Path) -> Result<Self> {
        let (width, height) = probe_video_dimensions(path)?;
        let frame_size = width as usize * height as usize * 4;

        let mut child = Command::new("ffmpeg")
            .arg("-v")
            .arg("error")
            .arg("-nostdin")
            .arg("-stream_loop")
            .arg("-1")
            .arg("-i")
            .arg(path)
            .arg("-an")
            .arg("-sn")
            .arg("-vf")
            .arg("fps=30,format=rgba")
            .arg("-f")
            .arg("rawvideo")
            .arg("-pix_fmt")
            .arg("rgba")
            .arg("pipe:1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("failed to spawn ffmpeg for {}", path.display()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("ffmpeg stdout pipe was not available"))?;

        Ok(Self {
            child,
            stdout,
            width,
            height,
            frame_size,
        })
    }

    fn next_frame(&mut self) -> Result<DecodedFrame> {
        let mut rgba = vec![0_u8; self.frame_size];
        self.stdout
            .read_exact(&mut rgba)
            .map_err(|error| match self.child.try_wait() {
                Ok(Some(status)) => anyhow!("ffmpeg exited unexpectedly with status {status}: {error}"),
                Ok(None) => anyhow!("failed to read frame bytes from ffmpeg: {error}"),
                Err(wait_error) => anyhow!("failed to read frame bytes from ffmpeg: {error}; and failed to inspect ffmpeg status: {wait_error}"),
            })?;

        Ok(DecodedFrame {
            width: self.width,
            height: self.height,
            rgba,
        })
    }
}

fn probe_video_dimensions(path: &Path) -> Result<(u32, u32)> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("v:0")
        .arg("-show_entries")
        .arg("stream=width,height")
        .arg("-of")
        .arg("csv=p=0:s=x")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .with_context(|| format!("failed to spawn ffprobe for {}", path.display()))?;

    if !output.status.success() {
        return Err(anyhow!(
            "ffprobe failed for {} with status {}",
            path.display(),
            output.status
        ));
    }

    let text = String::from_utf8(output.stdout)
        .context("ffprobe returned non-utf8 width/height output")?;
    let line = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .ok_or_else(|| {
            anyhow!(
                "ffprobe returned no video dimensions for {}",
                path.display()
            )
        })?;
    let (width, height) = line
        .trim()
        .split_once('x')
        .ok_or_else(|| anyhow!("ffprobe returned unexpected dimensions format: {line}"))?;

    Ok((
        width
            .parse()
            .with_context(|| format!("invalid ffprobe width: {width}"))?,
        height
            .parse()
            .with_context(|| format!("invalid ffprobe height: {height}"))?,
    ))
}

impl VideoSurfaceRuntime {
    fn new(session: &LayerSurfaceSession) -> Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = unsafe { session.create_wgpu_surface(&instance) }
            .map_err(|error| anyhow!("failed to create GPU surface: {error}"))?;
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .map_err(|error| anyhow!("failed to find adapter: {error}"))?;
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("backlayer-video-runner"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::default(),
        }))
        .map_err(|error| anyhow!("failed to request device: {error}"))?;

        let (width, height) = session.dimensions();
        let config = surface
            .get_default_config(&adapter, width, height)
            .ok_or_else(|| anyhow!("surface does not expose a default configuration"))?;
        surface.configure(&device, &config);

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("backlayer-video-uniforms"),
            size: std::mem::size_of::<VideoUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("backlayer-video-bind-group-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
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
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("backlayer-video-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("backlayer-video-shader"),
            source: wgpu::ShaderSource::Wgsl(VIDEO_SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("backlayer-video-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("backlayer-video-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        Ok(Self {
            surface,
            device,
            queue,
            pipeline,
            bind_group_layout,
            sampler,
            uniform_buffer,
            config,
        })
    }

    fn render_frame(&mut self, frame: &DecodedFrame) -> Result<()> {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("backlayer-video-frame"),
            size: wgpu::Extent3d {
                width: frame.width,
                height: frame.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            texture.as_image_copy(),
            &frame.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(frame.width * 4),
                rows_per_image: Some(frame.height),
            },
            wgpu::Extent3d {
                width: frame.width,
                height: frame.height,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let uniforms = VideoUniforms {
            surface_width: self.config.width as f32,
            surface_height: self.config.height as f32,
            frame_width: frame.width as f32,
            frame_height: frame.height as f32,
            fit_mode: 0,
            _padding0: 0,
            _padding1: 0,
            _padding2: 0,
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("backlayer-video-bind-group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let frame_texture = self
            .surface
            .get_current_texture()
            .map_err(|error| anyhow!("failed to acquire surface frame: {error}"))?;
        let surface_view = frame_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("backlayer-video-frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("backlayer-video-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        frame_texture.present();
        Ok(())
    }
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
