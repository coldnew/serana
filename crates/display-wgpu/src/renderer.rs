use display_protocol::{Color, ScreenBuffer, ScreenCell};
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};

use crate::atlas::GlyphAtlas;

// ── GPU data structures ──

/// Per-cell instance data sent to the GPU.
/// Must match the WGSL `InstanceInput` struct layout.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CellInstance {
    /// Pixel position (x, y) of the cell's top-left corner.
    position: [f32; 2],
    /// Foreground color packed as RGBA8 (u32).
    fg_color: u32,
    /// Background color packed as RGBA8 (u32).
    bg_color: u32,
    /// Character code (index into atlas grid).
    char_code: u32,
    /// Style flags: bit0=reverse, bit1=bold, bit2=italic, bit3=underline,
    /// bit4=strikethrough, bit5=dim, bit6=blink.
    flags: u32,
}

/// Uniform buffer contents.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    screen_size: [f32; 2],
}

// Compile-time layout checks.
const _: () = assert!(std::mem::size_of::<CellInstance>() == 24);
const _: () = assert!(std::mem::size_of::<Uniforms>() == 8);

// ── Color packing ──

fn pack_color(c: Color) -> u32 {
    (c.r as u32) << 24 | (c.g as u32) << 16 | (c.b as u32) << 8 | 0xFF
}

fn style_flags(cell: &ScreenCell) -> u32 {
    let mut flags = 0u32;
    if cell.reverse { flags |= 1 << 0; }
    if cell.bold { flags |= 1 << 1; }
    if cell.italic { flags |= 1 << 2; }
    if cell.underline { flags |= 1 << 3; }
    if cell.strikethrough { flags |= 1 << 4; }
    if cell.dim { flags |= 1 << 5; }
    if cell.blink { flags |= 1 << 6; }
    flags
}

// ── WgpuRenderer ──

/// GPU-accelerated renderer for `ScreenBuffer`.
///
/// Renders character cells using instanced quads with a glyph atlas.
pub struct WgpuRenderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub pixel_width: u32,
    pub pixel_height: u32,

    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    atlas_texture: wgpu::Texture,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,

    atlas: GlyphAtlas,
    cell_width: f32,
    cell_height: f32,
    grid_cols: u16,
    grid_rows: u16,
}

impl WgpuRenderer {
    /// Create a new renderer for the given surface and window size.
    ///
    /// `font_bytes` should be a TrueType/OpenType font (monospace recommended).
    pub async fn new(
        surface: wgpu::Surface<'static>,
        window: &winit::window::Window,
        font_bytes: &[u8],
    ) -> Self {
        let size = window.inner_size();
        let pixel_width = size.width.max(1);
        let pixel_height = size.height.max(1);

        // ── wgpu init ──
        let instance = wgpu::Instance::new(
            wgpu::InstanceDescriptor::new_without_display_handle()
        );

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("No suitable GPU adapter found");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("display-wgpu"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("Failed to create GPU device");

        // ── surface config ──
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: pixel_width,
            height: pixel_height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            desired_maximum_frame_latency: 2,
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        // ── glyph atlas ──
        let atlas = GlyphAtlas::new(font_bytes, 14.0);
        let atlas_size = atlas.size();

        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph-atlas"),
            size: wgpu::Extent3d {
                width: atlas_size,
                height: atlas_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("atlas-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        // Upload initial atlas data.
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            atlas.pixels(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(atlas_size),
                rows_per_image: Some(atlas_size),
            },
            wgpu::Extent3d {
                width: atlas_size,
                height: atlas_size,
                depth_or_array_layers: 1,
            },
        );

        // ── uniform buffer ──
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniforms"),
            contents: bytemuck::bytes_of(&Uniforms {
                screen_size: [pixel_width as f32, pixel_height as f32],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // ── bind group ──
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bind-group-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bind-group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        // ── pipeline ──
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cell-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cell-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<CellInstance>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        // cell_pos: vec2<f32>
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        // fg_color: u32
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Uint32,
                            offset: 8,
                            shader_location: 1,
                        },
                        // bg_color: u32
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Uint32,
                            offset: 12,
                            shader_location: 2,
                        },
                        // char_code: u32
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Uint32,
                            offset: 16,
                            shader_location: 3,
                        },
                        // flags: u32
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Uint32,
                            offset: 20,
                            shader_location: 4,
                        },
                    ],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview_mask: None,
            cache: None,
        });

        // ── vertex buffer (re-allocated per frame if needed) ──
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance-buffer"),
            size: 4096 * std::mem::size_of::<CellInstance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ── cell dimensions ──
        let glyph_size = atlas.glyph_size() as f32;
        let cell_width = glyph_size;
        let cell_height = glyph_size;
        let grid_cols = (pixel_width as f32 / cell_width) as u16;
        let grid_rows = (pixel_height as f32 / cell_height) as u16;

        Self {
            device,
            queue,
            surface,
            surface_config,
            pixel_width,
            pixel_height,
            pipeline,
            bind_group,
            uniform_buffer,
            atlas_texture,
            vertex_buffer,
            vertex_count: 0,
            atlas,
            cell_width,
            cell_height,
            grid_cols,
            grid_rows,
        }
    }

    /// Grid dimensions (columns, rows).
    pub fn grid_size(&self) -> (u16, u16) {
        (self.grid_cols, self.grid_rows)
    }

    /// Handle window resize.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.pixel_width = width;
        self.pixel_height = height;
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);

        self.grid_cols = (width as f32 / self.cell_width) as u16;
        self.grid_rows = (height as f32 / self.cell_height) as u16;

        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&Uniforms {
                screen_size: [width as f32, height as f32],
            }),
        );
    }

    /// Upload atlas to GPU if dirty.
    fn flush_atlas(&mut self) {
        if !self.atlas.is_dirty() {
            return;
        }
        let size = self.atlas.size();
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            self.atlas.pixels(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size),
                rows_per_image: Some(size),
            },
            wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
        );
        self.atlas.mark_clean();
    }

    /// Build instance data from a ScreenBuffer.
    fn build_instances(&mut self, buffer: &ScreenBuffer) -> Vec<CellInstance> {
        let cols = buffer.width.min(self.grid_cols);
        let rows = buffer.height.min(self.grid_rows);
        let mut instances = Vec::with_capacity((cols as usize) * (rows as usize));

        for y in 0..rows {
            for x in 0..cols {
                let cell = buffer.get(x, y);
                let _info = self.atlas.ensure_char(cell.ch);

                instances.push(CellInstance {
                    position: [
                        x as f32 * self.cell_width,
                        y as f32 * self.cell_height,
                    ],
                    fg_color: pack_color(cell.fg),
                    bg_color: pack_color(cell.bg),
                    char_code: cell.ch as u32,
                    flags: style_flags(&cell),
                });
            }
        }
        instances
    }

    /// Render a ScreenBuffer to the surface and present.
    pub fn render_screen_buffer(&mut self, buffer: &ScreenBuffer) {
        self.flush_atlas();
        let instances = self.build_instances(buffer);
        if instances.is_empty() {
            return;
        }

        // Upload instance data.
        let byte_data = bytemuck::cast_slice(&instances);
        if byte_data.len() <= self.vertex_buffer.size() as usize {
            self.queue.write_buffer(&self.vertex_buffer, 0, byte_data);
        } else {
            // Reallocate larger buffer.
            self.vertex_buffer = self.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("instance-buffer"),
                    contents: byte_data,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                },
            );
        }
        self.vertex_count = instances.len() as u32;

        // Acquire surface texture.
        let output = self.surface.get_current_texture();
        let output = match output {
            wgpu::CurrentSurfaceTexture::Success(t) => t,
            wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                self.resize(self.pixel_width, self.pixel_height);
                return;
            }
            _ => return, // Timeout, Occluded, Validation — skip frame
        };

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("render-encoder") }
        );

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.draw(0..4, 0..self.vertex_count);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}
