use clap::{Parser, Subcommand};
use mora_bin::mora::editor_core::MoraCore;
use mora_bin::mora::display::backend::{DisplayBackend, InputEvent};
use mora_bin::mora::display::tui::TuiBackend;
use mora_bin::mora::display::wgpu_backend::WgpuBackend;
use mora_bin::mora::display::style::MoraStyle;
use mora_compile::{CompileOptions, CompileTarget};
use display_protocol::{Cell, Color, Grid, Style, WireMessage, PROTOCOL_VERSION, DisplayCmd, InputEvent as ProtoInputEvent, KeyEvent, KeyCode, KeyModifiers, MouseEventKind};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::panic;
use std::path::Path;
use mora_bin::mora::display::event::{MoraKeyEvent, MoraKeyCode};
use mora_bin::mora::display::backend::MouseKind;

#[derive(Parser)]
#[command(name = "mora")]
#[command(about = "Mora - a modal editor with Lisp scripting")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// File to open in the editor (default mode)
    file: Option<String>,

    /// Use GPU-accelerated GUI backend (wgpu)
    #[arg(long)]
    gui: bool,

    /// Run as headless server (accept TCP connections)
    #[arg(long)]
    server: bool,

    /// TCP port for server mode
    #[arg(long, default_value = "7890")]
    port: u16,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a .mora file to binary or shared library
    Compile {
        /// Input .mora file
        input: String,

        /// Output path (optional)
        #[arg(short, long)]
        output: Option<String>,

        /// Compile to shared library instead of binary
        #[arg(short, long)]
        shared: bool,

        /// Optimization level (0-3)
        #[arg(short = 'O', long, default_value = "2")]
        opt_level: u8,

        /// Enable debug info
        #[arg(long)]
        debug: bool,
    },

    /// Evaluate mora-lisp code and print result
    Eval {
        /// Code to evaluate (or - for stdin)
        code: String,
    },

    /// Run a .mora script (non-interactive)
    Script {
        /// Script file to run
        file: String,

        /// Arguments to pass to the script
        args: Vec<String>,
    },

    /// Connect to a mora server
    Connect {
        /// Server address (host:port)
        #[arg(default_value = "127.0.0.1:7890")]
        addr: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Compile {
            input,
            output,
            shared,
            opt_level,
            debug,
        }) => {
            let options = CompileOptions {
                target: if shared {
                    CompileTarget::SharedLib
                } else {
                    CompileTarget::Binary
                },
                output,
                opt_level,
                debug,
            };

            let result = mora_compile::compile_file(&input, &options)?;
            println!("Compiled to: {}", result);
            Ok(())
        }

        Some(Commands::Eval { code }) => {
            let code = if code == "-" {
                let mut buf = String::new();
                io::stdin().read_to_string(&mut buf)?;
                buf
            } else {
                code
            };

            let mut lisp = mora_lisp::MoraLisp::new();
            match lisp.eval(&code) {
                Ok(result) => {
                    match &result {
                        mora_lisp::types::Value::Nil => {}
                        _ => println!("{}", result),
                    }
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Some(Commands::Script { file, args }) => {
            let code = std::fs::read_to_string(&file)
                .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", file, e))?;

            let mut lisp = mora_lisp::MoraLisp::new();

            // Set command line args
            let args_vec: Vec<mora_lisp::types::Value> = args
                .into_iter()
                .map(mora_lisp::types::Value::string)
                .collect();
            lisp.ns_mut()
                .current()
                .lock()
                .intern("*command-line-args*", mora_lisp::types::Value::vector(args_vec));

            match lisp.eval(&code) {
                Ok(_) => Ok(()),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        None => {
            if cli.server {
                run_server(cli.file, cli.port)
            } else {
                run_editor(cli.file, cli.gui)
            }
        }

        Some(Commands::Connect { addr }) => {
            run_client(&addr)
        }
    }
}

fn run_editor(file: Option<String>, gui: bool) -> anyhow::Result<()> {
    if gui {
        run_editor_wgpu(file)
    } else {
        run_editor_tui(file)
    }
}

fn init_lisp_state(core: &MoraCore) {
    let mut state = mora_bin::mora::lisp_ext::EditorState::new();
    state.lines = core.editor.buffer.lines.clone();
    state.cursor_row = core.editor.buffer.cursor.row;
    state.cursor_col = core.editor.buffer.cursor.col;
    state.modified = core.editor.buffer.modified;
    state.file_path = core.editor.buffer.path.as_ref().map(|p| p.to_string_lossy().to_string());
    state.mode = format!("{:?}", core.editor.mode()).to_lowercase();
    state.window_count = core.editor.windows.len().max(1);
    mora_bin::mora::lisp_ext::set_editor_state(state);
}

/// Convert a display-protocol Grid to a ratatui Buffer for TUI rendering.
fn grid_to_ratatui_buf(grid: &Grid) -> ratatui::buffer::Buffer {
    let area = ratatui::layout::Rect::new(0, 0, grid.width, grid.height);
    let mut buf = ratatui::buffer::Buffer::empty(area);
    for y in 0..grid.height {
        for x in 0..grid.width {
            let cell = grid.get(x, y);
            let ratatui_cell = buf.get_mut(x, y);
            ratatui_cell.set_symbol(&cell.ch.to_string());
            ratatui_cell.set_fg(ratatui::style::Color::from(cell.style.fg.unwrap_or(Color::WHITE)));
            ratatui_cell.set_bg(ratatui::style::Color::from(cell.style.bg.unwrap_or(Color::BLACK)));
        }
    }
    buf
}

fn run_editor_tui(file: Option<String>) -> anyhow::Result<()> {
    let mut backend = TuiBackend::new();
    backend.init().map_err(|e| anyhow::anyhow!(e))?;

    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        let _ = crossterm::execute!(io::stdout(), crossterm::cursor::Show);
        default_hook(info);
    }));

    let (w, h) = backend.size();
    let mut core = match file {
        Some(ref path) => MoraCore::open(Path::new(path), w, h)
            .map_err(|e| anyhow::anyhow!(e))?,
        None => MoraCore::new(w, h),
    };

    init_lisp_state(&core);
    core.editor.lisp_bridge.load_init_file();

    let result = (|| -> anyhow::Result<()> {
        loop {
            let frame = core.render_frame();
            let ratatui_buf = grid_to_ratatui_buf(&frame.grid);

            backend
                .draw(|buf, _area| {
                    // Copy our rendered grid into the terminal buffer
                    for y in 0..frame.grid.height.min(buf.area().height) {
                        for x in 0..frame.grid.width.min(buf.area().width) {
                            let src = ratatui_buf.get(x, y);
                            let dst = buf.get_mut(x, y);
                            *dst = src.clone();
                        }
                    }
                })
                .map_err(|e| anyhow::anyhow!(e))?;

            match backend.poll_event(50) {
                Some(InputEvent::Key(key)) => {
                    if core.handle_mora_input(InputEvent::Key(key)) {
                        break;
                    }
                }
                Some(event) => {
                    if core.handle_mora_input(event) {
                        break;
                    }
                }
                _ => {}
            }

            if core.quit_requested() {
                break;
            }
        }
        Ok(())
    })();

    backend.cleanup().map_err(|e| anyhow::anyhow!(e))?;
    result
}

fn run_editor_wgpu(file: Option<String>) -> anyhow::Result<()> {
    use mora_bin::mora::display::backend::CellBuffer;

    let mut backend = WgpuBackend::new();
    backend.init().map_err(|e| anyhow::anyhow!(e))?;

    let (w, h) = backend.size();
    let mut core = match file {
        Some(ref path) => MoraCore::open(Path::new(path), w, h)
            .map_err(|e| anyhow::anyhow!(e))?,
        None => MoraCore::new(w, h),
    };

    init_lisp_state(&core);
    core.editor.lisp_bridge.load_init_file();

    let result = (|| -> anyhow::Result<()> {
        loop {
            let frame = core.render_frame();

            // Convert FrameUpdate grid to CellBuffer for wgpu backend
            let mut cell_buf = CellBuffer::new(frame.grid.width, frame.grid.height);
            for y in 0..frame.grid.height {
                for x in 0..frame.grid.width {
                    let cell = frame.grid.get(x, y);
                    let style = MoraStyle {
                        fg: cell.style.fg,
                        bg: cell.style.bg,
                        bold: cell.style.bold,
                        italic: cell.style.italic,
                        underline: cell.style.underline,
                        strikethrough: cell.style.strikethrough,
                        dim: cell.style.dim,
                        reverse: cell.style.reverse,
                        blink: cell.style.blink,
                    };
                    cell_buf.set_cell(x, y, cell.ch, style);
                }
            }

            backend.render_buffer(&cell_buf).map_err(|e| anyhow::anyhow!(e))?;

            match backend.poll_event(50) {
                Some(InputEvent::Key(key)) => {
                    if core.handle_mora_input(InputEvent::Key(key)) {
                        break;
                    }
                }
                Some(event) => {
                    if core.handle_mora_input(event) {
                        break;
                    }
                }
                _ => {}
            }

            if core.quit_requested() {
                break;
            }
        }
        Ok(())
    })();

    backend.cleanup().map_err(|e| anyhow::anyhow!(e))?;
    result
}

fn run_server(file: Option<String>, port: u16) -> anyhow::Result<()> {
    let width: u16 = 120;
    let height: u16 = 40;
    let addr = format!("127.0.0.1:{}", port);

    let mut core = match &file {
        Some(path) => MoraCore::open(Path::new(path), width, height)
            .map_err(|e| anyhow::anyhow!(e))?,
        None => MoraCore::new(width, height),
    };

    init_lisp_state(&core);
    core.editor.lisp_bridge.load_init_file();

    eprintln!("mora-server listening on {}", addr);
    let listener = TcpListener::bind(&addr)?;

    for stream in listener.incoming() {
        let mut stream = stream?;
        eprintln!("Client connected: {}", stream.peer_addr()?);

        let mut reader = BufReader::new(stream.try_clone()?);

        let hello = WireMessage::ServerHello {
            version: PROTOCOL_VERSION,
            width,
            height,
        };
        stream.write_all(&hello.to_json_line().unwrap())?;
        stream.flush()?;

        let frame = core.render_frame();
        stream.write_all(&WireMessage::Frame(frame).to_json_line().unwrap())?;
        stream.flush()?;

        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    eprintln!("Client disconnected");
                    break;
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    match WireMessage::from_json_line(trimmed.as_bytes()) {
                        Ok(WireMessage::Input(event)) => {
                            let cmds = core.handle_input(event);
                            for cmd in &cmds {
                                match cmd {
                                    DisplayCmd::Quit => {
                                        let _ = stream.write_all(&WireMessage::Cmd(DisplayCmd::Quit).to_json_line().unwrap());
                                        let _ = stream.flush();
                                        return Ok(());
                                    }
                                    other => {
                                        let _ = stream.write_all(&WireMessage::Cmd(other.clone()).to_json_line().unwrap());
                                    }
                                }
                            }
                            let _ = stream.flush();
                            let frame = core.render_frame();
                            stream.write_all(&WireMessage::Frame(frame).to_json_line().unwrap())?;
                            stream.flush()?;
                        }
                        Ok(WireMessage::ClientHello { version }) => {
                            eprintln!("Client protocol version: {}", version);
                        }
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Protocol error: {} (raw: {:?})", e, trimmed);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Read error: {}", e);
                    break;
                }
            }
        }
        eprintln!("Waiting for next client...");
    }
    Ok(())
}

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
    KeyEvent::new(code, KeyModifiers::EMPTY)
}

fn run_client(addr: &str) -> anyhow::Result<()> {
    let mut stream = TcpStream::connect(addr)?;
    eprintln!("Connected to {}", addr);

    let mut reader = BufReader::new(stream.try_clone()?);

    let mut line = String::new();
    reader.read_line(&mut line)?;
    let hello = WireMessage::from_json_line(line.trim().as_bytes())
        .map_err(|e| anyhow::anyhow!("Protocol error: {}", e))?;

    let (width, height) = match &hello {
        WireMessage::ServerHello { version, width, height } => {
            eprintln!("Server protocol v{}, size {}x{}", version, width, height);
            (*width, *height)
        }
        _ => anyhow::bail!("Expected ServerHello, got {:?}", hello),
    };

    let client_hello = WireMessage::ClientHello { version: PROTOCOL_VERSION };
    stream.write_all(&client_hello.to_json_line().unwrap())?;
    stream.flush()?;

    let mut backend = TuiBackend::new();
    backend.init().map_err(|e| anyhow::anyhow!(e))?;

    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        let _ = crossterm::execute!(io::stdout(), crossterm::cursor::Show);
        default_hook(info);
    }));

    stream.set_nonblocking(true)?;

    let result = (|| -> anyhow::Result<()> {
        let mut current_frame: Option<FrameUpdate> = None;

        loop {
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => {
                        eprintln!("Server disconnected");
                        return Ok(());
                    }
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() { continue; }
                        match WireMessage::from_json_line(trimmed.as_bytes()) {
                            Ok(WireMessage::Frame(frame)) => {
                                current_frame = Some(frame);
                            }
                            Ok(WireMessage::Cmd(DisplayCmd::Quit)) => return Ok(()),
                            Ok(WireMessage::Cmd(DisplayCmd::Bell)) => { eprint!("\x07"); }
                            Ok(_) => {}
                            Err(_) => {}
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock
                        || e.kind() == io::ErrorKind::ConnectionAborted => {
                        break;
                    }
                    Err(e) => {
                        eprintln!("Read error: {}", e);
                        return Ok(());
                    }
                }
            }

            if let Some(ref frame) = current_frame {
                let ratatui_buf = grid_to_ratatui_buf(&frame.grid);
                let w = frame.grid.width;
                let h = frame.grid.height;
                backend.draw(|buf, _area| {
                    for y in 0..h.min(buf.area().height) {
                        for x in 0..w.min(buf.area().width) {
                            let src = ratatui_buf.get(x, y);
                            let dst = buf.get_mut(x, y);
                            *dst = src.clone();
                        }
                    }
                }).map_err(|e| anyhow::anyhow!(e))?;
            }

            match backend.poll_event(16) {
                Some(InputEvent::Key(key)) => {
                    let proto_key = mora_key_to_proto(key);
                    let msg = WireMessage::Input(ProtoInputEvent::Key(proto_key));
                    stream.write_all(&msg.to_json_line().unwrap())?;
                    stream.flush()?;
                }
                Some(InputEvent::Resize(w, h)) => {
                    let msg = WireMessage::Input(ProtoInputEvent::Resize { width: w, height: h });
                    stream.write_all(&msg.to_json_line().unwrap())?;
                    stream.flush()?;
                }
                Some(InputEvent::Mouse { x, y, kind }) => {
                    let proto_kind = match kind {
                        MouseKind::Press => MouseEventKind::Press,
                        MouseKind::Release => MouseEventKind::Release,
                        MouseKind::Drag => MouseEventKind::Drag,
                        MouseKind::ScrollUp => MouseEventKind::ScrollUp,
                        MouseKind::ScrollDown => MouseEventKind::ScrollDown,
                    };
                    let msg = WireMessage::Input(ProtoInputEvent::Mouse {
                        x, y, kind: proto_kind, modifiers: KeyModifiers::EMPTY,
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
