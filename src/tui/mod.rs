//! TUI frontend for serana
//! 
//! Provides a terminal user interface using ratatui for interactive
//! coding agent sessions.

pub mod app;
pub mod event;
pub mod ui;
pub mod components;
pub mod theme;

use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use self::app::App;
use self::event::{Event, EventHandler};
use crate::Result;

/// Run the TUI application
pub fn run(workspace: std::path::PathBuf) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and event handler
    let mut app = App::new(workspace);
    let events = EventHandler::new(Duration::from_millis(250));

    // Main loop
    let res = run_app(&mut terminal, &mut app, events);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    mut events: EventHandler,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        match events.next()? {
            Event::Key(key_event) => {
                if !app.handle_key_event(key_event)? {
                    return Ok(());
                }
            }
            Event::Resize(width, height) => {
                app.handle_resize(width, height);
            }
            Event::Tick => {
                app.tick();
            }
        }
    }
}
