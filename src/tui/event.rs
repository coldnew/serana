//! Event handling for TUI

use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind};
use tokio::sync::mpsc;

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

/// Event handler
pub struct EventHandler {
    rx: mpsc::Receiver<Event>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(async move {
            loop {
                let event = match event::poll(tick_rate) {
                    Ok(true) => {
                        if let Ok(e) = event::read() {
                            match e {
                                CrosstermEvent::Key(key) => {
                                    // Only process key press events (not release)
                                    if key.kind == KeyEventKind::Press {
                                        Some(Event::Key(key))
                                    } else {
                                        None
                                    }
                                }
                                CrosstermEvent::Resize(w, h) => Some(Event::Resize(w, h)),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    }
                    Ok(false) => Some(Event::Tick),
                    Err(_) => None,
                };

                if let Some(e) = event {
                    if tx.send(e).await.is_err() {
                        break;
                    }
                }
            }
        });

        Self { rx }
    }

    /// Get next event
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> crate::Result<Event> {
        self.rx.blocking_recv().ok_or_else(|| anyhow::anyhow!("Event channel closed"))
    }
}
