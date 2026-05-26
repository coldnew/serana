use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use mora_core::app::mora::MoraEditor;
use std::io;
use std::path::Path;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "mora")]
#[command(about = "Mora - a modal TUI editor")]
struct Cli {
    file: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let size = terminal.size()?;
    let editor_height = size.height.saturating_sub(3) as usize;
    let mut editor = if let Some(ref path) = cli.file {
        MoraEditor::open(Path::new(path), editor_height)
            .map_err(|e| anyhow::anyhow!("Failed to open {}: {}", path, e))?
    } else {
        MoraEditor::new(editor_height)
    };

    let result = (|| -> anyhow::Result<()> {
        loop {
            terminal.draw(|frame| {
                let widget = mora_core::app::mora::ui::EditorWidget::new(&editor);
                frame.render_widget(widget, frame.area());
            })?;

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        break;
                    }
                    editor.handle_key(key);
                }
            }

            if event::poll(Duration::from_millis(0))? {
                if let Event::Resize(_w, h) = event::read()? {
                    editor.set_height(h.saturating_sub(3) as usize);
                }
            }

            if editor.quit_requested() {
                break;
            }
        }
        Ok(())
    })();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
