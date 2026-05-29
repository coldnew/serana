use std::io;

pub use display_tui::TuiTerminal;

pub type Backend = ratatui::backend::CrosstermBackend<io::Stdout>;

pub struct Tui {
    inner: TuiTerminal,
}

impl Tui {
    pub fn new() -> io::Result<Self> {
        let mut inner = TuiTerminal::new()?;
        inner.init_with_mouse()?;
        Ok(Self { inner })
    }
    pub fn inner(&mut self) -> &mut TuiTerminal {
        &mut self.inner
    }

    pub fn terminal(&mut self) -> &mut ratatui::Terminal<Backend> {
        self.inner.terminal()
    }

    pub fn restore(&mut self) -> io::Result<()> {
        self.inner.cleanup()
    }

    pub fn draw<F>(&mut self, render_fn: F) -> io::Result<()>
    where
        F: FnOnce(&mut ratatui::buffer::Buffer, ratatui::layout::Rect),
    {
        self.inner.draw(render_fn)
    }

    pub fn poll_input(&mut self, timeout_ms: u64) -> Option<display_protocol::InputEvent> {
        self.inner.poll_input(timeout_ms)
    }

    pub fn render_ui(&mut self, node: &display_protocol::UiNode) -> io::Result<()> {
        let (w, h) = self.inner.size();
        let buf = display_protocol::paint::paint(node, w, h);
        self.inner.render_screen_buffer(&buf)
    }

    pub fn render_frame(&mut self, frame: &display_protocol::FrameUpdate) -> io::Result<()> {
        self.inner.render_frame(frame)
    }
}