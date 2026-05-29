use clap::Parser;
use display_protocol::{
    Cell, Color, CursorState, CursorStyle, DisplayCmd, FrameUpdate, Grid, InputEvent, KeyEvent,
    KeyCode, KeyModifiers, MouseEventKind, WireMessage, PROTOCOL_VERSION,
};
use mora_bin::mora::display::backend::{CellBuffer, DisplayBackend, MouseKind};
use mora_bin::mora::display::event::{MoraKeyEvent, MoraKeyCode, MoraKeyModifiers};
use mora_bin::mora::display::style::MoraStyle;
use mora_bin::mora::display::tui::TuiBackend;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::panic;

#[derive(Parser)]
#[command(name = "mora-client")]
#[command(about = "Mora remote client - connect to a mora server")]
struct Cli {
    /// Server address (host:port)
    #[arg(default_value = "127.0.0.1:7890")]
    addr: String,
}

/// Convert a mora KeyEvent (from backend) to a display-protocol KeyEvent
fn mora_key_to_proto(key: MoraKeyEvent) -> KeyEvent {
    let code = match key.code {
        MoraKeyCode::Char(c) => KeyCode::Char(c),
        MoraKeyCode::Enter => KeyCode::Enter,
        MoraKeyCode::Tab => KeyCode::Tab,
        MoraKeyCode::Backspace => KeyCode::Backspace,
        MoraKeyCode::Delete => KeyCode::Delete,
        MoraKeyCode::Esc => KeyCode::Esc,
        MoraKeyCode::Left => KeyCode::Left,
        MoraKeyCode::Right => KeyCode::Right,
        MoraKeyCode::Up => KeyCode::Up,
        MoraKeyCode::Down => KeyCode::Down,
        MoraKeyCode::Home => KeyCode::Home,
        MoraKeyCode::End => KeyCode::End,
        MoraKeyCode::PageUp => KeyCode::PageUp,
        MoraKeyCode::PageDown => KeyCode::PageDown,
        MoraKeyCode::F(n) => KeyCode::F(n),
        MoraKeyCode::Insert => KeyCode::Insert,
        MoraKeyCode::BackTab => KeyCode::BackTab,
    };
    let mods = KeyModifiers {
        ctrl: key.modifiers.ctrl,
        alt: key.modifiers.alt,
        shift: key.modifiers.shift,
        super_key: key.modifiers.super_key,
    };
    KeyEvent::new(code, mods)
}

/// Convert a display-protocol Grid to a ratatui Buffer for TUI rendering
fn grid_to_ratatui_buf(grid: &Grid) -> ratatui::buffer::Buffer {
    let area = ratatui::layout::Rect::new(0, 0, grid.width, grid.height);
    let mut buf = ratatui::buffer::Buffer::empty(area);
    for y in 0..grid.height {
        for x in 0..grid.width {
            let cell = grid.get(x, y);
            let ratatui_cell = buf.get_mut(x, y);
            ratatui_cell.set_symbol(&cell.ch.to_string());
            ratatui_cell.set_fg(display_tui::conversions::color_to_ratatui(
                cell.style.fg.unwrap_or(Color::WHITE),
            ));
            ratatui_cell.set_bg(display_tui::conversions::color_to_ratatui(
                cell.style.bg.unwrap_or(Color::BLACK),
            ));
        }
    }
    buf
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let mut stream = TcpStream::connect(&cli.addr)?;
    eprintln!("Connected to {}", cli.addr);

    let mut reader = BufReader::new(stream.try_clone()?);

    // Read ServerHello
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let hello = WireMessage::from_json_line(line.trim().as_bytes())
        .map_err(|e| anyhow::anyhow!("Protocol error: {}", e))?;

    let (mut width, mut height) = match &hello {
        WireMessage::ServerHello {
            version,
            width,
            height,
        } => {
            eprintln!("Server protocol v{}, size {}x{}", version, width, height);
            (*width, *height)
        }
        _ => anyhow::bail!("Expected ServerHello, got {:?}", hello),
    };

    // Send ClientHello
    let client_hello = WireMessage::ClientHello {
        version: PROTOCOL_VERSION,
    };
    stream.write_all(&client_hello.to_json_line().unwrap())?;
    stream.flush()?;

    // Initialize TUI backend
    let mut backend = TuiBackend::new();
    backend.init().map_err(|e| anyhow::anyhow!(e))?;

    // Panic hook to restore terminal
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen
        );
        let _ = crossterm::execute!(std::io::stdout(), crossterm::cursor::Show);
        default_hook(info);
    }));

    // Set stream to non-blocking for event polling
    stream.set_nonblocking(true)?;

    let result = (|| -> anyhow::Result<()> {
        let mut current_frame: Option<FrameUpdate> = None;

        loop {
            // Try to read frames from server (non-blocking)
            line.clear();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => {
                        eprintln!("Server disconnected");
                        return Ok(());
                    }
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        match WireMessage::from_json_line(trimmed.as_bytes()) {
                            Ok(WireMessage::Frame(frame)) => {
                                current_frame = Some(frame);
                            }
                            Ok(WireMessage::Cmd(cmd)) => match cmd {
                                DisplayCmd::Quit => return Ok(()),
                                DisplayCmd::SetTitle(title) => {
                                    // Could set terminal title via ANSI escape
                                }
                                DisplayCmd::Bell => {
                                    eprint!("\x07");
                                }
                                _ => {}
                            },
                            Ok(_) => {}
                            Err(_) => {}
                        }
                    }
                    Err(ref e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            || e.kind() == std::io::ErrorKind::ConnectionAborted =>
                    {
                        break;
                    }
                    Err(e) => {
                        eprintln!("Read error: {}", e);
                        return Ok(());
                    }
                }
            }

            // Render current frame
            if let Some(ref frame) = current_frame {
                let ratatui_buf = grid_to_ratatui_buf(&frame.grid);
                let w = frame.grid.width;
                let h = frame.grid.height;

                backend
                    .draw(|buf, _area| {
                        for y in 0..h.min(buf.area().height) {
                            for x in 0..w.min(buf.area().width) {
                                let src = ratatui_buf.get(x, y);
                                let dst = buf.get_mut(x, y);
                                *dst = src.clone();
                            }
                        }
                    })
                    .map_err(|e| anyhow::anyhow!(e))?;
            }

            // Poll local input
            match backend.poll_event(16) {
                Some(mora_bin::mora::display::backend::InputEvent::Key(key)) => {
                    let proto_key = mora_key_to_proto(key);
                    let msg = WireMessage::Input(InputEvent::Key(proto_key));
                    stream.write_all(&msg.to_json_line().unwrap())?;
                    stream.flush()?;
                }
                Some(mora_bin::mora::display::backend::InputEvent::Resize(w, h)) => {
                    width = w;
                    height = h;
                    let msg = WireMessage::Input(InputEvent::Resize {
                        width: w,
                        height: h,
                    });
                    stream.write_all(&msg.to_json_line().unwrap())?;
                    stream.flush()?;
                }
                Some(mora_bin::mora::display::backend::InputEvent::Mouse { x, y, kind }) => {
                    let proto_kind = match kind {
                        MouseKind::Press => MouseEventKind::Press,
                        MouseKind::Release => MouseEventKind::Release,
                        MouseKind::Drag => MouseEventKind::Drag,
                        MouseKind::ScrollUp => MouseEventKind::ScrollUp,
                        MouseKind::ScrollDown => MouseEventKind::ScrollDown,
                    };
                    let msg = WireMessage::Input(InputEvent::Mouse {
                        x,
                        y,
                        kind: proto_kind,
                        modifiers: KeyModifiers::EMPTY,
                    });
                    stream.write_all(&msg.to_json_line().unwrap())?;
                    stream.flush()?;
                }
                _ => {}
            }
        }
    })();

    backend.cleanup().map_err(|e| anyhow::anyhow!(e))?;
    result
}
