//! Event handling for TUI

use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind};

/// Terminal event
#[derive(Debug, Clone)]
pub enum Event {
    /// Key press
    Key(KeyEvent),
    /// Terminal resize
    Resize(u16, u16),
    /// Tick interval
    Tick,
}

/// Event handler using synchronous polling
pub struct EventHandler {
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    /// Get next event
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> crate::Result<Event> {
        loop {
            match event::poll(self.tick_rate) {
                Ok(true) => {
                    if let Ok(e) = event::read() {
                        match e {
                            CrosstermEvent::Key(key)
                                if key.kind == KeyEventKind::Press =>
                            {
                                return Ok(Event::Key(key));
                            }
                            CrosstermEvent::Resize(w, h) => return Ok(Event::Resize(w, h)),
                            _ => {}
                        }
                    }
                }
                Ok(false) => return Ok(Event::Tick),
                Err(e) => {
                    // If poll fails, return tick to keep the loop running
                    // but log the error for debugging
                    eprintln!("Event poll error: {}", e);
                    return Ok(Event::Tick);
                }
            }
        }
    }
}
