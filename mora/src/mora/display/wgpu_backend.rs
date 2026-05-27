use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use wgpu::util::DeviceExt;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::platform::pump_events::EventLoopExtPumpEvents;
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};

use super::backend::{DisplayBackend, InputEvent, CellBuffer};
use super::event::{MoraKeyCode, MoraKeyEvent, MoraKeyModifiers};
use super::style::{MoraColor, MoraStyle};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct RectVertex {
    position: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    screen_size: [f32; 2],
}

struct CellData {
    ch: char,
    style: MoraStyle,
}

struct SharedState {
    window: Option<Arc<Window>>,
    events: Vec<InputEvent>,
    modifiers: ModifiersState,
}

pub struct WgpuBackend {
    instance: Option<wgpu::Instance>,
    adapter: Option<wgpu::Adapter>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,

    rect_pipeline: Option<wgpu::RenderPipeline>,
    uniform_buffer: Option<wgpu::Buffer>,
    uniform_bind_group: Option<wgpu::BindGroup>,

    font_system: Option<glyphon::FontSystem>,
    swash_cache: Option<glyphon::SwashCache>,
    cache: Option<glyphon::Cache>,
    atlas: Option<glyphon::TextAtlas>,
    text_renderer: Option<glyphon::TextRenderer>,
    viewport: Option<glyphon::Viewport>,

    cells: Vec<Vec<CellData>>,
    cols: u16,
    rows: u16,
    cell_width: f32,
    cell_height: f32,
    font_size: f32,

    cursor_visible: bool,
    cursor_x: u16,
    cursor_y: u16,

    event_loop: Option<EventLoop<()>>,
    shared: Rc<RefCell<SharedState>>,

    needs_render: bool,
    needs_text_reshape: bool,
    dirty_cells: Vec<bool>,
    rect_vertices: Vec<RectVertex>,
    rect_vertex_buffer: Option<wgpu::Buffer>,
    text_hash: u64,
}

fn color_to_linear(c: MoraColor) -> [f32; 4] {
    fn srgb_to_linear(c: u8) -> f32 {
        let s = c as f32 / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    }
    [
        srgb_to_linear(c.r),
        srgb_to_linear(c.g),
        srgb_to_linear(c.b),
        1.0,
    ]
}

fn resolve_colors(style: MoraStyle) -> (Option<MoraColor>, Option<MoraColor>) {
    if style.reverse {
        (style.bg, style.fg)
    } else {
        (style.fg, style.bg)
    }
}

fn hash_cells(cells: &[Vec<CellData>]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for row in cells {
        for cell in row {
            cell.ch.hash(&mut hasher);
            cell.style.fg.map(|c| (c.r, c.g, c.b)).hash(&mut hasher);
            cell.style.bg.map(|c| (c.r, c.g, c.b)).hash(&mut hasher);
            cell.style.bold.hash(&mut hasher);
            cell.style.italic.hash(&mut hasher);
            cell.style.underline.hash(&mut hasher);
            cell.style.reverse.hash(&mut hasher);
            cell.style.dim.hash(&mut hasher);
        }
    }
    hasher.finish()
}

impl WgpuBackend {
    pub fn new() -> Self {
        Self {
            instance: None,
            adapter: None,
            device: None,
            queue: None,
            surface: None,
            surface_config: None,
            rect_pipeline: None,
            uniform_buffer: None,
            uniform_bind_group: None,
            font_system: None,
            swash_cache: None,
            cache: None,
            atlas: None,
            text_renderer: None,
            viewport: None,
            cells: Vec::new(),
            cols: 80,
            rows: 24,
            cell_width: 8.0,
            cell_height: 16.0,
            font_size: 14.0,
            cursor_visible: true,
            cursor_x: 0,
            cursor_y: 0,
            event_loop: None,
            shared: Rc::new(RefCell::new(SharedState {
                window: None,
                events: Vec::new(),
                modifiers: ModifiersState::empty(),
            })),
            needs_render: true,
            needs_text_reshape: true,
            dirty_cells: Vec::new(),
            rect_vertices: Vec::new(),
            rect_vertex_buffer: None,
            text_hash: 0,
        }
    }

    fn init_cells(&mut self) {
        let total = self.rows as usize * self.cols as usize;
        self.cells = (0..self.rows as usize)
            .map(|_| {
                (0..self.cols as usize)
                    .map(|_| CellData {
                        ch: ' ',
                        style: MoraStyle::default(),
                    })
                    .collect()
            })
            .collect();
        self.dirty_cells = vec![true; total];
        self.rect_vertices = vec![
            RectVertex {
                position: [0.0, 0.0],
                color: [0.0, 0.0, 0.0, 1.0]
            };
            total * 6
        ];
        self.needs_text_reshape = true;
    }

    fn build_rect_pipeline(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
    ) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rect Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/rect.wgsl").into()),
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Rect Uniform BGL"),
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Rect Pipeline Layout"),
            bind_group_layouts: &[Some(&uniform_bind_group_layout)],
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rect Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<RectVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 8,
                            shader_location: 1,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        (pipeline, uniform_bind_group_layout)
    }

    fn measure_font(font_system: &mut glyphon::FontSystem, font_size: f32) -> (f32, f32) {
        let mut buf = glyphon::Buffer::new(
            font_system,
            glyphon::Metrics::new(font_size, font_size * 1.2),
        );
        buf.set_size(font_system, Some(f32::MAX), Some(f32::MAX));
        buf.set_text(
            font_system,
            "M",
            &glyphon::Attrs::new().family(glyphon::Family::Monospace),
            glyphon::Shaping::Advanced,
            None,
        );
        buf.shape_until_scroll(font_system, false);

        let width = buf
            .layout_runs()
            .next()
            .map(|r| r.line_w)
            .unwrap_or(font_size * 0.6);
        let height = font_size * 1.2;
        (width, height)
    }

    fn render_frame(&mut self) {
        let device = match &self.device {
            Some(d) => d,
            None => return,
        };
        let queue = match &self.queue {
            Some(q) => q,
            None => return,
        };
        let surface = match &self.surface {
            Some(s) => s,
            None => return,
        };
        let rect_pipeline = match &self.rect_pipeline {
            Some(p) => p,
            None => return,
        };
        let uniform_bg = match &self.uniform_bind_group {
            Some(bg) => bg,
            None => return,
        };

        let surface_texture = surface.get_current_texture();
        let frame = match surface_texture {
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => return,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                let shared = self.shared.borrow();
                if let Some(window) = &shared.window {
                    let size = window.inner_size();
                    drop(shared);
                    self.reconfigure_surface(size.width, size.height);
                }
                return;
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        // Update only dirty cells in the persistent vertex buffer
        let mut any_dirty = false;
        for row in 0..self.rows as usize {
            for col in 0..self.cols as usize {
                let idx = row * self.cols as usize + col;
                if !self.dirty_cells[idx] {
                    continue;
                }
                any_dirty = true;
                self.dirty_cells[idx] = false;

                let cell = &self.cells[row][col];
                let x = col as f32 * self.cell_width;
                let y = row as f32 * self.cell_height;
                let (_fg, bg) = resolve_colors(cell.style);
                let bg_color = bg.unwrap_or(MoraColor::new(30, 30, 30));
                let c = color_to_linear(bg_color);
                let x2 = x + self.cell_width;
                let y2 = y + self.cell_height;

                let vi = idx * 6;
                self.rect_vertices[vi] = RectVertex { position: [x, y], color: c };
                self.rect_vertices[vi + 1] = RectVertex { position: [x2, y], color: c };
                self.rect_vertices[vi + 2] = RectVertex { position: [x, y2], color: c };
                self.rect_vertices[vi + 3] = RectVertex { position: [x2, y], color: c };
                self.rect_vertices[vi + 4] = RectVertex { position: [x2, y2], color: c };
                self.rect_vertices[vi + 5] = RectVertex { position: [x, y2], color: c };
            }
        }

        // Upload dirty region or full buffer
        let vertex_data = bytemuck::cast_slice(&self.rect_vertices);
        if any_dirty {
            match &self.rect_vertex_buffer {
                Some(buf) => queue.write_buffer(buf, 0, vertex_data),
                None => {
                    self.rect_vertex_buffer = Some(device.create_buffer_init(
                        &wgpu::util::BufferInitDescriptor {
                            label: Some("Rect Vertices"),
                            contents: vertex_data,
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        },
                    ));
                }
            }
        }

        // Render rect pass
        if let Some(vbuf) = &self.rect_vertex_buffer {
            let total_verts = (self.rows as usize * self.cols as usize * 6) as u32;
            let cursor_verts = if self.cursor_visible { 6 } else { 0 };

            // Append cursor vertices
            let mut cursor_data = [RectVertex {
                position: [0.0, 0.0],
                color: [0.0, 0.0, 0.0, 1.0],
            }; 6];
            if self.cursor_visible {
                let cx = self.cursor_x as f32 * self.cell_width;
                let cy = self.cursor_y as f32 * self.cell_height;
                let cx2 = cx + self.cell_width;
                let cy2 = cy + self.cell_height;
                let cc = color_to_linear(MoraColor::new(200, 200, 200));
                cursor_data = [
                    RectVertex { position: [cx, cy], color: cc },
                    RectVertex { position: [cx2, cy], color: cc },
                    RectVertex { position: [cx, cy2], color: cc },
                    RectVertex { position: [cx2, cy], color: cc },
                    RectVertex { position: [cx2, cy2], color: cc },
                    RectVertex { position: [cx, cy2], color: cc },
                ];
            }

            // We need a temp buffer for cursor since it changes every frame
            let cursor_buf = if self.cursor_visible {
                Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Cursor Vertices"),
                    contents: bytemuck::cast_slice(&cursor_data),
                    usage: wgpu::BufferUsages::VERTEX,
                }))
            } else {
                None
            };

            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Rect Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.118,
                                g: 0.118,
                                b: 0.118,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(rect_pipeline);
                pass.set_bind_group(0, uniform_bg, &[]);
                pass.set_vertex_buffer(0, vbuf.slice(..));
                pass.draw(0..total_verts, 0..1);

                if let Some(cbuf) = &cursor_buf {
                    pass.set_vertex_buffer(0, cbuf.slice(..));
                    pass.draw(0..cursor_verts, 0..1);
                }
            }
        }

        // Text rendering — only reshape if content changed
        let current_hash = hash_cells(&self.cells);
        if current_hash != self.text_hash {
            self.text_hash = current_hash;
            self.needs_text_reshape = true;
        }

        if self.needs_text_reshape {
            if let (Some(font_system), Some(atlas), Some(text_renderer), Some(swash_cache), Some(viewport)) = (
                &mut self.font_system,
                &mut self.atlas,
                &mut self.text_renderer,
                &mut self.swash_cache,
                &mut self.viewport,
            ) {
                // Build text spans from cells
                let mut spans: Vec<(String, MoraColor)> = Vec::new();

                for row in 0..self.rows as usize {
                    let mut line_text = String::new();
                    let mut line_fg = MoraColor::new(220, 220, 220);
                    let mut started = false;

                    for col in 0..self.cols as usize {
                        let cell = &self.cells[row][col];
                        let (fg, _bg) = resolve_colors(cell.style);
                        let fg = fg.unwrap_or(MoraColor::new(220, 220, 220));

                        if !started {
                            line_fg = fg;
                            started = true;
                        }

                        if fg != line_fg && !line_text.is_empty() {
                            spans.push((std::mem::take(&mut line_text), line_fg));
                            line_fg = fg;
                        }
                        line_text.push(cell.ch);
                    }

                    if !line_text.is_empty() {
                        spans.push((line_text, line_fg));
                    }
                    if row < self.rows as usize - 1 {
                        if let Some(last) = spans.last_mut() {
                            last.0.push('\n');
                        }
                    }
                }

                let mut buffer = glyphon::Buffer::new(
                    font_system,
                    glyphon::Metrics::new(self.font_size, self.cell_height),
                );
                buffer.set_size(
                    font_system,
                    Some(self.cols as f32 * self.cell_width),
                    Some(self.rows as f32 * self.cell_height),
                );

                let full_text: String = spans.iter().map(|(s, _)| s.as_str()).collect();
                let attrs_list: Vec<(std::ops::Range<usize>, glyphon::Attrs)> = spans
                    .iter()
                    .scan(0, |offset, (text, fg)| {
                        let start = *offset;
                        let end = start + text.len();
                        *offset = end;
                        let attrs = glyphon::Attrs::new()
                            .family(glyphon::Family::Monospace)
                            .color(glyphon::Color::rgba(fg.r, fg.g, fg.b, 255));
                        Some((start..end, attrs))
                    })
                    .collect();

                let default_attrs = glyphon::Attrs::new().family(glyphon::Family::Monospace);
                buffer.set_rich_text(
                    font_system,
                    attrs_list.iter().map(|(range, attrs)| {
                        (&full_text[range.clone()], attrs.clone())
                    }),
                    &default_attrs,
                    glyphon::Shaping::Advanced,
                    None,
                );
                buffer.shape_until_scroll(font_system, false);

                let width = self.cols as f32 * self.cell_width;
                let height = self.rows as f32 * self.cell_height;
                let scale = {
                    let shared = self.shared.borrow();
                    shared.window.as_ref().map(|w| w.scale_factor() as f32).unwrap_or(1.0)
                };

                let text_area = glyphon::TextArea {
                    buffer: &buffer,
                    left: 0.0,
                    top: 0.0,
                    scale,
                    bounds: glyphon::TextBounds {
                        left: 0,
                        top: 0,
                        right: width as i32,
                        bottom: height as i32,
                    },
                    default_color: glyphon::Color::rgba(220, 220, 220, 255),
                    custom_glyphs: &[],
                };

                let (win_w, win_h) = {
                    let shared = self.shared.borrow();
                    shared.window.as_ref().map(|w| {
                        let s = w.inner_size();
                        (s.width, s.height)
                    }).unwrap_or((800, 600))
                };

                viewport.update(queue, glyphon::Resolution {
                    width: win_w,
                    height: win_h,
                });

                if let Err(e) = text_renderer.prepare(
                    device,
                    queue,
                    font_system,
                    atlas,
                    &viewport,
                    [text_area],
                    swash_cache,
                ) {
                    eprintln!("Text prepare error: {e}");
                } else {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Text Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            depth_slice: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    text_renderer.render(atlas, &viewport, &mut pass).ok();
                }

                self.needs_text_reshape = false;
            }
        } else {
            // Text unchanged — just render the already-prepared text
            if let (Some(atlas), Some(text_renderer), Some(viewport)) = (
                &self.atlas,
                &self.text_renderer,
                &self.viewport,
            ) {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Text Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                text_renderer.render(atlas, &viewport, &mut pass).ok();
            }
        }

        queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        self.needs_render = false;
    }

    fn reconfigure_surface(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        let device = match &self.device {
            Some(d) => d,
            None => return,
        };
        let surface = match &self.surface {
            Some(s) => s,
            None => return,
        };
        let config = match &mut self.surface_config {
            Some(c) => c,
            None => return,
        };
        config.width = width;
        config.height = height;
        surface.configure(device, config);

        let queue = match &self.queue {
            Some(q) => q,
            None => return,
        };
        let uniforms = Uniforms {
            screen_size: [width as f32, height as f32],
        };
        if let Some(ub) = &self.uniform_buffer {
            queue.write_buffer(ub, 0, bytemuck::cast_slice(&[uniforms]));
        }

        self.cell_width = width as f32 / self.cols as f32;
        self.cell_height = height as f32 / self.rows as f32;
        self.needs_render = true;
    }

    fn map_winit_key(event: &winit::event::KeyEvent, mods: ModifiersState) -> Option<MoraKeyEvent> {
        use winit::keyboard::{Key, NamedKey};

        let code = match &event.logical_key {
            Key::Named(named) => match named {
                NamedKey::Enter => MoraKeyCode::Enter,
                NamedKey::Tab => MoraKeyCode::Tab,
                NamedKey::Backspace => MoraKeyCode::Backspace,
                NamedKey::Delete => MoraKeyCode::Delete,
                NamedKey::Escape => MoraKeyCode::Esc,
                NamedKey::ArrowLeft => MoraKeyCode::Left,
                NamedKey::ArrowRight => MoraKeyCode::Right,
                NamedKey::ArrowUp => MoraKeyCode::Up,
                NamedKey::ArrowDown => MoraKeyCode::Down,
                NamedKey::Home => MoraKeyCode::Home,
                NamedKey::End => MoraKeyCode::End,
                NamedKey::PageUp => MoraKeyCode::PageUp,
                NamedKey::PageDown => MoraKeyCode::PageDown,
                NamedKey::Insert => MoraKeyCode::Insert,
                NamedKey::F1 => MoraKeyCode::F(1),
                NamedKey::F2 => MoraKeyCode::F(2),
                NamedKey::F3 => MoraKeyCode::F(3),
                NamedKey::F4 => MoraKeyCode::F(4),
                NamedKey::F5 => MoraKeyCode::F(5),
                NamedKey::F6 => MoraKeyCode::F(6),
                NamedKey::F7 => MoraKeyCode::F(7),
                NamedKey::F8 => MoraKeyCode::F(8),
                NamedKey::F9 => MoraKeyCode::F(9),
                NamedKey::F10 => MoraKeyCode::F(10),
                NamedKey::F11 => MoraKeyCode::F(11),
                NamedKey::F12 => MoraKeyCode::F(12),
                _ => return None,
            },
            Key::Character(c) => {
                if let Some(ch) = c.chars().next() {
                    MoraKeyCode::Char(ch)
                } else {
                    return None;
                }
            }
            _ => return None,
        };

        let mora_mods = MoraKeyModifiers {
            ctrl: mods.contains(ModifiersState::CONTROL),
            alt: mods.contains(ModifiersState::ALT),
            shift: mods.contains(ModifiersState::SHIFT),
            super_key: mods.contains(ModifiersState::SUPER),
        };

        Some(MoraKeyEvent::new(code, mora_mods))
    }

    fn ensure_surface(&mut self) {
        if self.surface.is_some() || self.device.is_none() {
            return;
        }

        // Pump events once to trigger window creation via resumed()
        if let Some(el) = self.event_loop.as_mut() {
            let mut app = WgpuApp {
                shared: self.shared.clone(),
            };
            el.pump_app_events(Some(Duration::from_millis(100)), &mut app);
        }

        let window = {
            let shared = self.shared.borrow();
            match &shared.window {
                Some(w) => w.clone(),
                None => return,
            }
        };

        let device = self.device.as_ref().unwrap();
        let size = window.inner_size();
        let surface = self.instance.as_ref().unwrap().create_surface(window).unwrap();
        let caps = surface.get_capabilities(self.adapter.as_ref().unwrap());
        let format = caps.formats.iter().find(|f| f.is_srgb()).copied().unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(device, &config);
        self.surface = Some(surface);
        self.surface_config = Some(config);
        self.reconfigure_surface(size.width, size.height);
    }
}

struct WgpuApp {
    shared: Rc<RefCell<SharedState>>,
}

impl ApplicationHandler for WgpuApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let mut shared = self.shared.borrow_mut();
        if shared.window.is_none() {
            let attrs = Window::default_attributes()
                .with_title("mora")
                .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 768.0));
            match event_loop.create_window(attrs) {
                Ok(window) => {
                    shared.window = Some(Arc::new(window));
                    shared.events.push(InputEvent::Resize(80, 24));
                }
                Err(e) => {
                    eprintln!("Failed to create window: {e}");
                    event_loop.exit();
                }
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let mut shared = self.shared.borrow_mut();
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::ModifiersChanged(mods) => {
                shared.modifiers = mods.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == winit::event::ElementState::Pressed {
                    if let Some(key) = WgpuBackend::map_winit_key(&event, shared.modifiers) {
                        shared.events.push(InputEvent::Key(key));
                    }
                }
            }
            WindowEvent::Focused(focused) => {
                let event = if focused {
                    InputEvent::FocusGained
                } else {
                    InputEvent::FocusLost
                };
                shared.events.push(event);
            }
            _ => {}
        }
    }
}

impl DisplayBackend for WgpuBackend {
    fn init(&mut self) -> Result<(), String> {
        let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
        self.event_loop = Some(event_loop);

        // Create wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        self.instance = Some(instance);

        // Pump event loop to create window
        if let Some(el) = self.event_loop.as_mut() {
            let mut app = WgpuApp {
                shared: self.shared.clone(),
            };
            el.pump_app_events(Some(Duration::from_secs(2)), &mut app);
        }

        // Get adapter (compatible with the window surface if available)
        let instance = self.instance.as_ref().unwrap();
        let window = {
            let shared = self.shared.borrow();
            shared.window.clone()
        };

        let adapter = if let Some(window) = &window {
            let surface = instance.create_surface(window.clone()).unwrap();
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }))
            .map_err(|e| format!("Failed to find GPU adapter: {e}"))?
        } else {
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            }))
            .map_err(|e| format!("Failed to find GPU adapter: {e}"))?
        };

        self.adapter = Some(adapter);

        let (device, queue) = pollster::block_on(self.adapter.as_ref().unwrap().request_device(
            &wgpu::DeviceDescriptor {
                label: Some("mora wgpu device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
        ))
        .map_err(|e| e.to_string())?;

        self.device = Some(device);
        self.queue = Some(queue);

        // Initialize font system
        let mut font_system = glyphon::FontSystem::new();
        let swash_cache = glyphon::SwashCache::new();

        let (cell_w, cell_h) = Self::measure_font(&mut font_system, self.font_size);
        self.cell_width = cell_w;
        self.cell_height = cell_h;

        self.cols = (1024.0 / self.cell_width) as u16;
        self.rows = (768.0 / self.cell_height) as u16;

        self.font_system = Some(font_system);
        self.swash_cache = Some(swash_cache);

        // Create glyphon cache, atlas, text renderer, viewport
        let device_ref = self.device.as_ref().unwrap();
        let cache = glyphon::Cache::new(device_ref);
        let mut atlas = glyphon::TextAtlas::new(
            device_ref,
            self.queue.as_ref().unwrap(),
            &cache,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        );
        let text_renderer = glyphon::TextRenderer::new(
            &mut atlas,
            device_ref,
            wgpu::MultisampleState::default(),
            None,
        );
        let viewport = glyphon::Viewport::new(device_ref, &cache);

        self.cache = Some(cache);
        self.atlas = Some(atlas);
        self.text_renderer = Some(text_renderer);
        self.viewport = Some(viewport);

        self.init_cells();

        // Create uniform buffer
        let uniforms = Uniforms {
            screen_size: [1024.0, 768.0],
        };
        let device_ref = self.device.as_ref().unwrap();
        let uniform_buffer = device_ref.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group_layout =
            device_ref.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Uniform BGL"),
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

        let uniform_bind_group = device_ref.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform BG"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Build rect pipeline
        let (rect_pipeline, _bgl) =
            Self::build_rect_pipeline(device_ref, wgpu::TextureFormat::Bgra8UnormSrgb);

        self.rect_pipeline = Some(rect_pipeline);
        self.uniform_buffer = Some(uniform_buffer);
        self.uniform_bind_group = Some(uniform_bind_group);

        // Create surface if window is available
        self.ensure_surface();

        Ok(())
    }

    fn cleanup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn size(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }

    fn clear(&mut self) {
        self.init_cells();
        self.needs_render = true;
        self.needs_text_reshape = true;
    }

    fn flush(&mut self) -> Result<(), String> {
        self.ensure_surface();

        if self.needs_render {
            self.render_frame();
        }

        // Request redraw from winit
        {
            let shared = self.shared.borrow();
            if let Some(window) = &shared.window {
                window.request_redraw();
            }
        }

        Ok(())
    }

    fn set_cell(&mut self, x: u16, y: u16, ch: char, style: MoraStyle) {
        if (x as usize) < self.cols as usize && (y as usize) < self.rows as usize {
            let cell = &mut self.cells[y as usize][x as usize];
            if cell.ch != ch || cell.style != style {
                *cell = CellData { ch, style };
                let idx = y as usize * self.cols as usize + x as usize;
                self.dirty_cells[idx] = true;
                self.needs_render = true;
            }
        }
    }

    fn set_line(&mut self, x: u16, y: u16, text: &str, style: MoraStyle) {
        for (i, ch) in text.chars().enumerate() {
            let cx = x + i as u16;
            if cx >= self.cols {
                break;
            }
            self.set_cell(cx, y, ch, style);
        }
    }

    fn poll_event(&mut self, timeout_ms: u64) -> Option<InputEvent> {
        // Check for already-queued events
        {
            let mut shared = self.shared.borrow_mut();
            if !shared.events.is_empty() {
                return Some(shared.events.remove(0));
            }
        }

        // Pump winit event loop
        if let Some(el) = self.event_loop.as_mut() {
            let mut app = WgpuApp {
                shared: self.shared.clone(),
            };
            let timeout = Some(Duration::from_millis(timeout_ms));
            el.pump_app_events(timeout, &mut app);
        }

        // Return queued event
        let mut shared = self.shared.borrow_mut();
        if !shared.events.is_empty() {
            return Some(shared.events.remove(0));
        }
        None
    }

    fn hide_cursor(&mut self) {
        self.cursor_visible = false;
        self.needs_render = true;
    }

    fn show_cursor(&mut self) {
        self.cursor_visible = true;
        self.needs_render = true;
    }

    fn set_cursor(&mut self, x: u16, y: u16) {
        self.cursor_x = x;
        self.cursor_y = y;
        self.needs_render = true;
    }

    fn render_buffer(&mut self, buf: &CellBuffer) -> Result<(), String> {
        let size_changed = self.cols != buf.width || self.rows != buf.height;
        self.cols = buf.width;
        self.rows = buf.height;
        self.cells = (0..buf.height)
            .map(|y| {
                (0..buf.width)
                    .map(|x| {
                        let cell = buf.get(x, y);
                        CellData { ch: cell.ch, style: cell.style }
                    })
                    .collect()
            })
            .collect();

        if size_changed {
            let total = self.rows as usize * self.cols as usize;
            self.dirty_cells = vec![true; total];
            self.rect_vertices = vec![
                RectVertex {
                    position: [0.0, 0.0],
                    color: [0.0, 0.0, 0.0, 1.0]
                };
                total * 6
            ];
            self.rect_vertex_buffer = None; // force buffer recreate
        } else {
            self.dirty_cells.iter_mut().for_each(|d| *d = true);
        }
        self.needs_render = true;
        self.needs_text_reshape = true;
        self.flush()
    }
}
