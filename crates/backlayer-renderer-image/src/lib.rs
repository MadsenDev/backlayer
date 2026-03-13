use std::fs;

use backlayer_types::{AssetMetadata, ImageFitMode};
use backlayer_wayland::LayerSurfaceSession;
use image::ImageReader;
use pollster::block_on;
use thiserror::Error;

const IMAGE_SHADER: &str = r#"
struct ImageUniforms {
  surface_width: f32,
  surface_height: f32,
  image_width: f32,
  image_height: f32,
  fit_mode: u32,
  _padding0: u32,
  _padding1: u32,
  _padding2: u32,
}

@group(0) @binding(0)
var wallpaper_texture: texture_2d<f32>;

@group(0) @binding(1)
var wallpaper_sampler: sampler;

@group(0) @binding(2)
var<uniform> image_uniforms: ImageUniforms;

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
  var positions = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -3.0),
    vec2<f32>(3.0, 1.0),
    vec2<f32>(-1.0, 1.0),
  );
  var uvs = array<vec2<f32>, 3>(
    vec2<f32>(0.0, 2.0),
    vec2<f32>(2.0, 0.0),
    vec2<f32>(0.0, 0.0),
  );

  var out: VertexOutput;
  out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
  out.uv = uvs[vertex_index];
  return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  let surface = vec2<f32>(max(image_uniforms.surface_width, 1.0), max(image_uniforms.surface_height, 1.0));
  let source = vec2<f32>(max(image_uniforms.image_width, 1.0), max(image_uniforms.image_height, 1.0));
  let fit = image_uniforms.fit_mode;

  if (fit == 2u) {
    return textureSample(wallpaper_texture, wallpaper_sampler, in.position.xy / surface);
  }

  var drawn = source;
  if (fit == 0u) {
    let scale = max(surface.x / source.x, surface.y / source.y);
    drawn = source * scale;
  } else if (fit == 1u) {
    let scale = min(surface.x / source.x, surface.y / source.y);
    drawn = source * scale;
  }

  let offset = (surface - drawn) * 0.5;
  let uv = (in.position.xy - offset) / drawn;
  if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
  }

  return textureSample(wallpaper_texture, wallpaper_sampler, uv);
}
"#;

#[derive(Debug, Error)]
pub enum ImageRendererError {
    #[error("image renderer received non-image asset kind")]
    WrongKind,
    #[error("image asset does not exist: {0}")]
    MissingFile(String),
    #[error("unsupported image extension: {0}")]
    UnsupportedExtension(String),
    #[error("failed to read image asset: {0}")]
    Read(#[from] std::io::Error),
    #[error("failed to decode image asset: {0}")]
    Decode(String),
    #[error("failed to find a compatible GPU adapter: {0}")]
    Adapter(String),
    #[error("failed to request a GPU device: {0}")]
    Device(String),
    #[error("failed to create or configure a GPU surface: {0}")]
    Surface(String),
    #[error("failed to acquire a surface frame: {0}")]
    Frame(String),
}

#[derive(Debug, Clone)]
struct DecodedImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

#[derive(Debug, Default, Clone)]
pub struct ImageRenderer;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ImageUniforms {
    surface_width: f32,
    surface_height: f32,
    image_width: f32,
    image_height: f32,
    fit_mode: u32,
    _padding0: u32,
    _padding1: u32,
    _padding2: u32,
}

impl ImageRenderer {
    pub fn name(&self) -> &'static str {
        "image"
    }

    pub fn validate_asset(&self, asset: &AssetMetadata) -> Result<(), ImageRendererError> {
        self.decode_image(asset).map(|_| ())
    }

    fn decode_image(&self, asset: &AssetMetadata) -> Result<DecodedImage, ImageRendererError> {
        if asset.kind != backlayer_types::WallpaperKind::Image {
            return Err(ImageRendererError::WrongKind);
        }

        if !asset.entrypoint.is_file() {
            return Err(ImageRendererError::MissingFile(
                asset.entrypoint.display().to_string(),
            ));
        }

        let extension = asset
            .entrypoint
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .ok_or_else(|| {
                ImageRendererError::UnsupportedExtension(asset.entrypoint.display().to_string())
            })?;

        match extension.as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "ppm" => {}
            _ => return Err(ImageRendererError::UnsupportedExtension(extension)),
        }

        let bytes = fs::read(&asset.entrypoint)?;
        let image = ImageReader::new(std::io::Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|error| ImageRendererError::Decode(error.to_string()))?
            .decode()
            .map_err(|error| ImageRendererError::Decode(error.to_string()))?;
        let rgba = image.to_rgba8();

        Ok(DecodedImage {
            width: rgba.width(),
            height: rgba.height(),
            rgba: rgba.into_raw(),
        })
    }

    fn fit_mode(asset: &AssetMetadata) -> ImageFitMode {
        asset.image_fit.clone().unwrap_or(ImageFitMode::Cover)
    }

    pub fn render_asset_once(
        &self,
        asset: &AssetMetadata,
        session: &mut LayerSurfaceSession,
    ) -> Result<String, ImageRendererError> {
        let decoded = self.decode_image(asset)?;
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = unsafe { session.create_wgpu_surface(&instance) }
            .map_err(|error| ImageRendererError::Surface(error.to_string()))?;
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .map_err(|error| ImageRendererError::Adapter(error.to_string()))?;
        let info = adapter.get_info();
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("backlayer-image-probe"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::default(),
        }))
        .map_err(|error| ImageRendererError::Device(error.to_string()))?;

        let (width, height) = session.dimensions();
        let config = surface
            .get_default_config(&adapter, width, height)
            .ok_or_else(|| {
                ImageRendererError::Surface(
                    "surface does not expose a default configuration".into(),
                )
            })?;
        surface.configure(&device, &config);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("backlayer-image-texture"),
            size: wgpu::Extent3d {
                width: decoded.width,
                height: decoded.height,
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
            &decoded.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(decoded.width * 4),
                rows_per_image: Some(decoded.height),
            },
            wgpu::Extent3d {
                width: decoded.width,
                height: decoded.height,
                depth_or_array_layers: 1,
            },
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("backlayer-image-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("backlayer-image-shader"),
            source: wgpu::ShaderSource::Wgsl(IMAGE_SHADER.into()),
        });
        let uniforms = ImageUniforms {
            surface_width: width as f32,
            surface_height: height as f32,
            image_width: decoded.width as f32,
            image_height: decoded.height as f32,
            fit_mode: match Self::fit_mode(asset) {
                ImageFitMode::Cover => 0,
                ImageFitMode::Contain => 1,
                ImageFitMode::Stretch => 2,
                ImageFitMode::Center => 3,
            },
            _padding0: 0,
            _padding1: 0,
            _padding2: 0,
        };
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("backlayer-image-uniform-buffer"),
            size: std::mem::size_of::<ImageUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("backlayer-image-bind-group-layout"),
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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("backlayer-image-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("backlayer-image-pipeline"),
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
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("backlayer-image-bind-group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let frame = surface
            .get_current_texture()
            .map_err(|error| ImageRendererError::Frame(error.to_string()))?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("backlayer-image-frame"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("backlayer-image-render-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit(Some(encoder.finish()));
        frame.present();

        Ok(format!(
            "{} on {:?} via {} (image frame submitted {}x{} from {} source {}x{} fit {:?})",
            info.name,
            info.backend,
            info.driver,
            width,
            height,
            asset.id,
            decoded.width,
            decoded.height,
            Self::fit_mode(asset),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use backlayer_types::{
        AssetMetadata, AssetSourceKind, CompatibilityInfo, ImageFitMode, WallpaperKind,
    };

    use super::ImageRenderer;

    const PNG_1X1_RGBA: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8,
        0xcf, 0xc0, 0xf0, 0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99, 0x3d, 0x1d, 0x00, 0x00,
        0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    #[test]
    fn validate_asset_decodes_png_fixture() {
        let renderer = ImageRenderer::default();
        let path = temp_fixture_path("backlayer-image-renderer-test.png");
        fs::write(&path, PNG_1X1_RGBA).expect("fixture should write");

        let asset = AssetMetadata {
            id: "test.image".into(),
            name: "Test Image".into(),
            kind: WallpaperKind::Image,
            animated: false,
            image_fit: Some(ImageFitMode::Contain),
            source_kind: AssetSourceKind::Native,
            preview_image: None,
            compatibility: CompatibilityInfo::default(),
            import_metadata: None,
            entrypoint: path.clone(),
        };

        renderer
            .validate_asset(&asset)
            .expect("png fixture should validate");

        fs::remove_file(path).ok();
    }

    fn temp_fixture_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("{}-{}", std::process::id(), name))
    }
}
