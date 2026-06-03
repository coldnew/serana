use std::time::Duration;

use crossterm::event::KeyEvent;
use display_protocol::InputEvent;

/// Terminal event
#[derive(Debug, Clone)]
pub enum Event {
    /// Key press (display-protocol InputEvent)
    Input(InputEvent),
    /// Tick interval
    Tick,
}

/// Event handler using display-tui polling
pub struct EventHandler {
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(_tick_rate: Duration) -> Self {
        Self {
            tick_rate: Duration::from_millis(16),
        }
    }

    /// Get next event using display-tui input polling
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> crate::core::Result<Event> {
        match display_tui::poll_input(16) {
            Some(event) => Ok(Event::Input(event)),
            None => Ok(Event::Tick),
        }
    }
}
