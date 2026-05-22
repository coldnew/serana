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
    pub fn new(_tick_rate: Duration) -> Self {
        // Ignore the tick_rate parameter and use fast polling
        Self {
            tick_rate: Duration::from_millis(16),
        } // ~60fps
    }

    /// Get next event
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> serana_core::Result<Event> {
        // Fast poll for immediate responsiveness
        match event::poll(self.tick_rate) {
            Ok(true) => {
                if let Ok(e) = event::read() {
                    match e {
                        CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                            return Ok(Event::Key(key));
                        }
                        CrosstermEvent::Resize(w, h) => return Ok(Event::Resize(w, h)),
                        _ => {}
                    }
                }
                // Event was consumed but not relevant, return tick
                Ok(Event::Tick)
            }
            Ok(false) => Ok(Event::Tick),
            Err(e) => {
                // If poll fails, sleep briefly to avoid busy loop
                std::thread::sleep(Duration::from_millis(10));
                eprintln!("Event poll error: {}", e);
                Ok(Event::Tick)
            }
        }
    }
}
