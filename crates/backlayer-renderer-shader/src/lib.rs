use std::fs;

use backlayer_types::AssetMetadata;
use backlayer_wayland::LayerSurfaceSession;
use naga::front::wgsl;
use pollster::block_on;
use thiserror::Error;

const ANIMATED_PROBE_SHADER: &str = r#"
struct ProbeUniforms {
  time_seconds: f32,
  width: f32,
  height: f32,
  _padding: f32,
}

@group(0) @binding(0)
var<uniform> probe: ProbeUniforms;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
  var positions = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -3.0),
    vec2<f32>(3.0, 1.0),
    vec2<f32>(-1.0, 1.0),
  );

  let xy = positions[vertex_index];
  return vec4<f32>(xy, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
  let resolution = vec2<f32>(probe.width, probe.height);
  let uv = position.xy / resolution;
  let centered = uv - vec2<f32>(0.5, 0.5);
  let aspect = probe.width / max(probe.height, 1.0);
  let time = probe.time_seconds;

  let wave = sin(centered.x * 10.0 * aspect + time * 1.8) * 0.5 + 0.5;
  let rings = sin(length(centered * vec2<f32>(aspect, 1.0)) * 24.0 - time * 3.2) * 0.5 + 0.5;
  let glow = smoothstep(0.7, 1.0, max(wave, rings));
  let base = vec3<f32>(0.03, 0.05, 0.09);
  let accent = vec3<f32>(0.16, 0.80, 0.78) * glow;

  return vec4<f32>(base + accent, 1.0);
}
"#;

#[derive(Debug, Error)]
pub enum ShaderRendererError {
    #[error("shader renderer received non-shader asset kind")]
    WrongKind,
    #[error("shader asset does not exist: {0}")]
    MissingFile(String),
    #[error("shader source must be a `.wgsl` file")]
    UnsupportedExtension,
    #[error("failed to read shader source: {0}")]
    Read(#[from] std::io::Error),
    #[error("shader source is invalid WGSL: {0}")]
    Parse(String),
    #[error("failed to find a compatible GPU adapter: {0}")]
    Adapter(String),
    #[error("failed to request a GPU device: {0}")]
    Device(String),
    #[error("failed to create or configure a GPU surface: {0}")]
    Surface(String),
    #[error("failed to acquire a surface frame: {0}")]
    Frame(String),
}

pub struct ShaderRuntime {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    bind_group: Option<wgpu::BindGroup>,
    uniform_buffer: Option<wgpu::Buffer>,
    info: wgpu::AdapterInfo,
    width: u32,
    height: u32,
    asset_id: String,
    animated: bool,
    started_at: std::time::Instant,
}

#[derive(Debug, Default, Clone)]
pub struct ShaderRenderer;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ProbeUniforms {
    time_seconds: f32,
    width: f32,
    height: f32,
    _padding: f32,
}

impl ShaderRenderer {
    pub fn name(&self) -> &'static str {
        "shader"
    }

    pub fn preferred_backend(&self) -> wgpu::Backends {
        wgpu::Backends::VULKAN
    }

    pub fn validate_asset(&self, asset: &AssetMetadata) -> Result<(), ShaderRendererError> {
        let source = self.load_source(asset)?;
        wgsl::parse_str(&source)
            .map_err(|error| ShaderRendererError::Parse(error.emit_to_string(&source)))?;
        Ok(())
    }

    fn load_source(&self, asset: &AssetMetadata) -> Result<String, ShaderRendererError> {
        if asset.kind != backlayer_types::WallpaperKind::Shader {
            return Err(ShaderRendererError::WrongKind);
        }

        if !asset.entrypoint.is_file() {
            return Err(ShaderRendererError::MissingFile(
                asset.entrypoint.display().to_string(),
            ));
        }

        if asset.entrypoint.extension().and_then(|ext| ext.to_str()) != Some("wgsl") {
            return Err(ShaderRendererError::UnsupportedExtension);
        }

        fs::read_to_string(&asset.entrypoint).map_err(ShaderRendererError::Read)
    }

    pub fn render_asset_once(
        &self,
        asset: &AssetMetadata,
        session: &mut LayerSurfaceSession,
    ) -> Result<String, ShaderRendererError> {
        let mut runtime = self.create_runtime(asset, session)?;
        runtime.render_frame()?;
        Ok(runtime.detail())
    }

    pub fn create_runtime(
        &self,
        asset: &AssetMetadata,
        session: &LayerSurfaceSession,
    ) -> Result<ShaderRuntime, ShaderRendererError> {
        let source = self.load_source(asset)?;
        self.create_runtime_from_source(asset.id.clone(), source, session, asset.animated)
    }

    pub fn create_animated_probe_runtime(
        &self,
        asset_id: impl Into<String>,
        session: &LayerSurfaceSession,
    ) -> Result<ShaderRuntime, ShaderRendererError> {
        self.create_runtime_from_source(
            asset_id.into(),
            ANIMATED_PROBE_SHADER.to_string(),
            session,
            true,
        )
    }

    fn create_runtime_from_source(
        &self,
        asset_id: String,
        source: String,
        session: &LayerSurfaceSession,
        animated: bool,
    ) -> Result<ShaderRuntime, ShaderRendererError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: self.preferred_backend(),
            ..Default::default()
        });
        let surface = unsafe { session.create_wgpu_surface(&instance) }
            .map_err(|error| ShaderRendererError::Surface(error.to_string()))?;
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .map_err(|error| ShaderRendererError::Adapter(error.to_string()))?;
        let info = adapter.get_info();
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("backlayer-shader-runtime"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::default(),
        }))
        .map_err(|error| ShaderRendererError::Device(error.to_string()))?;
        let (width, height) = session.dimensions();
        let config = surface
            .get_default_config(&adapter, width, height)
            .ok_or_else(|| {
                ShaderRendererError::Surface(
                    "surface does not expose a default configuration".into(),
                )
            })?;
        surface.configure(&device, &config);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("backlayer-shader-module"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let (pipeline_layout, bind_group, uniform_buffer) = if animated {
            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("backlayer-shader-bind-group-layout"),
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
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("backlayer-shader-pipeline-layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });
            let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("backlayer-shader-uniform-buffer"),
                size: std::mem::size_of::<ProbeUniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("backlayer-shader-bind-group"),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
            });
            (
                Some(pipeline_layout),
                Some(bind_group),
                Some(uniform_buffer),
            )
        } else {
            (None, None, None)
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("backlayer-shader-pipeline"),
            layout: pipeline_layout.as_ref(),
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

        Ok(ShaderRuntime {
            surface,
            device,
            queue,
            pipeline,
            bind_group,
            uniform_buffer,
            info,
            width,
            height,
            asset_id,
            animated,
            started_at: std::time::Instant::now(),
        })
    }
}

impl ShaderRuntime {
    pub fn animated(&self) -> bool {
        self.animated
    }

    pub fn render_frame(&mut self) -> Result<(), ShaderRendererError> {
        if self.animated {
            let uniforms = ProbeUniforms {
                time_seconds: self.started_at.elapsed().as_secs_f32(),
                width: self.width as f32,
                height: self.height as f32,
                _padding: 0.0,
            };
            if let Some(uniform_buffer) = &self.uniform_buffer {
                self.queue
                    .write_buffer(uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
            }
        }
        let frame = self
            .surface
            .get_current_texture()
            .map_err(|error| ShaderRendererError::Frame(error.to_string()))?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("backlayer-shader-frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("backlayer-shader-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.12,
                            b: 0.18,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            if let Some(bind_group) = &self.bind_group {
                pass.set_bind_group(0, bind_group, &[]);
            }
            pass.draw(0..3, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }

    pub fn detail(&self) -> String {
        format!(
            "{} on {:?} via {} (shader frame submitted {}x{} from{} {})",
            self.info.name,
            self.info.backend,
            self.info.driver,
            self.width,
            self.height,
            if self.animated { " animated" } else { "" },
            self.asset_id
        )
    }
}
