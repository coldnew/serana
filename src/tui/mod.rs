//! TUI frontend for serana
//!
//! Custom inline TUI inspired by oh-my-pi, with vertical flow rendering and no
//! alternate-screen takeover.

pub mod app;
pub mod component;
pub mod components;
pub mod event;
pub mod style;
pub mod terminal;
pub mod tool_execution;
pub mod tui;
pub mod ui;

use std::path::PathBuf;
use std::time::Duration;

use self::app::App;
use self::event::Event;
use self::tui::Tui;
use crate::Result;

/// Run the TUI application.
pub fn run(workspace: PathBuf, model: String, provider: String) -> Result<()> {
    let mut tui = Tui::new()?;
    let mut app = App::with_model(workspace, model, provider);
    let events = event::EventHandler::new(Duration::from_millis(16));

    tui.clear_screen()?;
    tui.hide_cursor()?;

    let result = run_app(&mut tui, &mut app, events);

    tui.show_cursor()?;
    tui.restore()?;
    result
}


fn run_app(tui: &mut Tui, app: &mut App, mut events: event::EventHandler) -> Result<()> {
    loop {
        ui::draw(tui, app)?;
        tui.render()?;

        match events.next()? {
            Event::Key(key_event) => {
                if !app.handle_key_event(key_event)? {
                    return Ok(());
                }
                tui.request_render();
            }
            Event::Resize(width, height) => {
                app.handle_resize(width, height);
                tui.request_render();
            }
            Event::Tick => {
                app.tick();
                tui.request_render();
            }
        }
    }
}
