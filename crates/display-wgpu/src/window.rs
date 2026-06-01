use display_protocol::{InputEvent, UiNode, paint};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use crate::input::winit_to_input_event;
use crate::renderer::WgpuRenderer;

/// Configuration for the WGPU window.
pub struct WgpuConfig {
    pub width: u32,
    pub height: u32,
    pub title: String,
}

impl Default for WgpuConfig {
    fn default() -> Self {
        Self {
            width: 1024,
            height: 768,
            title: "display-wgpu".to_string(),
        }
    }
}

/// High-level windowed WGPU display.
///
/// Owns the window, renderer, and event loop. Drives rendering through
/// a user-provided callback.
///
/// # Example
///
/// ```ignore
/// use display_wgpu::{WgpuWindow, WgpuConfig};
/// use display_protocol::*;
///
/// let config = WgpuConfig {
///     title: "My App".into(),
///     ..Default::default()
/// };
///
/// let font = std::fs::read("fonts/Hack-Regular.ttf").unwrap();
///
/// WgpuWindow::new(config, &font).run(|events, ctx| {
///     // Build UI from input events
///     let mut ui = UiNode::text("Hello from WGPU!");
///     for ev in events {
///         // handle input...
///     }
///     ui
/// }).unwrap();
/// ```
pub struct WgpuWindow {
    config: WgpuConfig,
    font_bytes: Vec<u8>,
}

impl WgpuWindow {
    /// Create a new WGPU window with the given config and font.
    pub fn new(config: WgpuConfig, font_bytes: &[u8]) -> Self {
        Self {
            config,
            font_bytes: font_bytes.to_vec(),
        }
    }

    /// Run the event loop with a render callback.
    ///
    /// The callback receives a list of input events accumulated since the
    /// last frame, and a `RenderCtx` for querying window state.
    /// It returns a `UiNode` tree that will be painted and rendered.
    ///
    /// **This function does not return** (winit takes ownership of the thread).
    pub fn run<F>(self, render_fn: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(&[InputEvent], &RenderCtx) -> UiNode + 'static,
    {
        let event_loop = EventLoop::new()?;

        let mut app = WgpuApp {
            config: self.config,
            font_bytes: self.font_bytes,
            window: None,
            renderer: None,
            pending_events: Vec::new(),
            render_fn: Box::new(render_fn),
        };

        event_loop.run_app(&mut app)?;
        Ok(())
    }
}

/// Context passed to the render callback.
pub struct RenderCtx {
    pub grid_cols: u16,
    pub grid_rows: u16,
    pub pixel_width: u32,
    pub pixel_height: u32,
}

// ── Internal winit application ──

struct WgpuApp {
    config: WgpuConfig,
    font_bytes: Vec<u8>,
    window: Option<Arc<Window>>,
    renderer: Option<WgpuRenderer>,
    pending_events: Vec<InputEvent>,
    render_fn: Box<dyn FnMut(&[InputEvent], &RenderCtx) -> UiNode>,
}

impl ApplicationHandler for WgpuApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = Window::default_attributes()
            .with_title(&self.config.title)
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.config.width,
                self.config.height,
            ));

        let window = Arc::new(
            event_loop.create_window(attrs).expect("Failed to create window")
        );

        // Create surface.
        let instance = wgpu::Instance::new(
            wgpu::InstanceDescriptor::new_without_display_handle()
        );
        let surface = instance.create_surface(window.clone()).unwrap();

        // Create renderer (needs async, so we block on it).
        // Pass instance so it lives as long as the renderer (surface requires the instance to stay alive in wgpu 29.x).
        let renderer = pollster::block_on(WgpuRenderer::new(instance, surface, &window, &self.font_bytes));

        self.window = Some(window);
        self.renderer = Some(renderer);

        // Trigger first redraw.
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match &event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                self.render_frame();
                return;
            }
            _ => {}
        }

        // Accumulate input events.
        if let Some(input) = winit_to_input_event(&event) {
            self.pending_events.push(input);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}

impl WgpuApp {
    fn render_frame(&mut self) {
        let renderer = match &self.renderer {
            Some(r) => r,
            None => return,
        };

        let ctx = RenderCtx {
            grid_cols: renderer.grid_size().0,
            grid_rows: renderer.grid_size().1,
            pixel_width: renderer.pixel_width,
            pixel_height: renderer.pixel_height,
        };

        // Drain pending events and pass to user callback.
        let events: Vec<InputEvent> = self.pending_events.drain(..).collect();
        let ui = (self.render_fn)(&events, &ctx);

        // Paint UI tree into a ScreenBuffer.
        let buf = paint(&ui, ctx.grid_cols, ctx.grid_rows);

        // Render.
        if let Some(renderer) = &mut self.renderer {
            renderer.render_screen_buffer(&buf);
        }
    }
}
