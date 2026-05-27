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

    pub fn render_frame(&mut self, frame: &display_protocol::FrameUpdate) -> io::Result<()> {
        self.inner.render_frame(frame)
    }
}
