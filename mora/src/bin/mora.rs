use clap::{Parser, Subcommand};
use display_protocol::{DisplayCmd, FrameUpdate, WireMessage, PROTOCOL_VERSION};
use display_tui::TuiTerminal;
use display_wgpu::{WgpuConfig, WgpuWindow};
use mora_bin::lisp;
use mora_bin::mora::editor_core::MoraCore;
use mora_bin::mora::font::DEFAULT_FONT;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

#[derive(Parser)]
#[command(name = "mora")]
#[command(about = "Mora - a modal editor with Lisp scripting")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// File to open in the editor (default mode)
    file: Option<String>,

    /// Force terminal UI, like emacs -nw
    #[arg(long)]
    nw: bool,
}

#[derive(Subcommand)]
enum Commands {
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
    /// Start an interactive REPL
    Repl,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Eval { code }) => {
            let code = if code == "-" {
                let mut buf = String::new();
                io::stdin().read_to_string(&mut buf)?;
                buf
            } else {
                code
            };

            let mut lisp = lisp::MoraLisp::new();
            match lisp.eval(&code) {
                Ok(result) => {
                    match &result {
                        lisp::types::Value::Nil => {}
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

            let mut lisp = lisp::MoraLisp::new();

            // Set command line args
            let args_vec: Vec<lisp::types::Value> =
                args.into_iter().map(lisp::types::Value::string).collect();
            lisp.ns_mut()
                .current()
                .lock()
                .intern("*command-line-args*", lisp::types::Value::vector(args_vec));

            match lisp.eval(&code) {
                Ok(_) => Ok(()),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        None => run_editor(cli.file, cli.nw),

        Some(Commands::Connect { addr }) => run_client(&addr),

        Some(Commands::Repl) => {
            println!("mora-lisp REPL (Ctrl-D to exit)");
            let mut repl = lisp::repl::Repl::new();
            repl.run().map_err(|e| anyhow::anyhow!("{}", e))
        }
    }
}

fn run_editor(file: Option<String>, force_nw: bool) -> anyhow::Result<()> {
    let use_gui = if force_nw {
        false
    } else {
        // Auto-detect: GUI if a display server is available (like Emacs).
        // Check for X11 (DISPLAY) or Wayland (WAYLAND_DISPLAY).
        std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok()
    };
    if use_gui {
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
    state.file_path = core
        .editor
        .buffer
        .path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string());
    state.mode = format!("{:?}", core.editor.mode()).to_lowercase();
    state.window_count = core.editor.windows.len().max(1);
    mora_bin::mora::lisp_ext::set_editor_state(state);
}

fn run_editor_tui(file: Option<String>) -> anyhow::Result<()> {
    let mut tui = TuiTerminal::new().map_err(|e| anyhow::anyhow!(e))?;
    tui.init().map_err(|e| anyhow::anyhow!(e))?;
    display_tui::install_panic_hook();

    let (w, h) = tui.size();
    let mut core = match file {
        Some(ref path) => MoraCore::open(Path::new(path), w, h).map_err(|e| anyhow::anyhow!(e))?,
        None => MoraCore::new(w, h),
    };

    init_lisp_state(&core);
    core.editor.lisp_bridge.load_init_file();
    core.show_menu_bar = false;

    let result = (|| -> anyhow::Result<()> {
        loop {
            let frame = core.render_ui_frame();
            tui.render_frame(&frame).map_err(|e| anyhow::anyhow!(e))?;

            match tui.poll_input(50) {
                Some(event) => {
                    let cmds = core.handle_input(event);
                    for cmd in &cmds {
                        if matches!(cmd, DisplayCmd::Quit) {
                            return Ok(());
                        }
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

    tui.cleanup().map_err(|e| anyhow::anyhow!(e))?;
    result
}

fn run_editor_wgpu(file: Option<String>) -> anyhow::Result<()> {
    let config = WgpuConfig {
        title: "mora".into(),
        width: 1024,
        height: 768,
    };

    // Create core with a default grid size; the callback will resize based on RenderCtx.
    let mut core = match file {
        Some(ref path) => {
            MoraCore::open(Path::new(path), 80, 24).map_err(|e| anyhow::anyhow!(e))?
        }
        None => MoraCore::new(80, 24),
    };

    init_lisp_state(&core);
    core.editor.lisp_bridge.load_init_file();
    core.show_menu_bar = true;

    WgpuWindow::new(config, DEFAULT_FONT)
        .run(move |events, ctx| {
            // Sync cell dimensions for mouse coordinate conversion.
            core.set_cell_size(ctx.cell_width, ctx.cell_height);
            // Resize core if grid dimensions changed.
            let (cols, rows) = (ctx.grid_cols, ctx.grid_rows);
            if cols != core.width() || rows != core.height() {
                core.resize(cols, rows);
            }

            // Process input events.
            for ev in events {
                let cmds = core.handle_input(ev.clone());
                for cmd in &cmds {
                    if matches!(cmd, DisplayCmd::Quit) {
                        std::process::exit(0);
                    }
                }
            }

            if core.quit_requested() {
                std::process::exit(0);
            }

            // Build UiNode tree directly — no FrameUpdate/Grid round-trip.
            core.build_ui_node(cols, rows)
        })
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    Ok(())
}

fn run_server(file: Option<String>, port: u16) -> anyhow::Result<()> {
    let width: u16 = 120;
    let height: u16 = 40;
    let addr = format!("127.0.0.1:{}", port);

    let mut core = match &file {
        Some(path) => {
            MoraCore::open(Path::new(path), width, height).map_err(|e| anyhow::anyhow!(e))?
        }
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

        let frame = core.render_ui_frame();
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
                                        let _ = stream.write_all(
                                            &WireMessage::Cmd(DisplayCmd::Quit)
                                                .to_json_line()
                                                .unwrap(),
                                        );
                                        let _ = stream.flush();
                                        return Ok(());
                                    }
                                    other => {
                                        let _ = stream.write_all(
                                            &WireMessage::Cmd(other.clone())
                                                .to_json_line()
                                                .unwrap(),
                                        );
                                    }
                                }
                            }
                            let _ = stream.flush();
                            let frame = core.render_ui_frame();
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

fn run_client(addr: &str) -> anyhow::Result<()> {
    let mut stream = TcpStream::connect(addr)?;
    eprintln!("Connected to {}", addr);

    let mut reader = BufReader::new(stream.try_clone()?);

    let mut line = String::new();
    reader.read_line(&mut line)?;
    let hello = WireMessage::from_json_line(line.trim().as_bytes())
        .map_err(|e| anyhow::anyhow!("Protocol error: {}", e))?;

    let (_width, _height) = match &hello {
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
    let client_hello = WireMessage::ClientHello {
        version: PROTOCOL_VERSION,
    };
    stream.write_all(&client_hello.to_json_line().unwrap())?;
    stream.flush()?;

    let mut tui = TuiTerminal::new().map_err(|e| anyhow::anyhow!(e))?;
    tui.init().map_err(|e| anyhow::anyhow!(e))?;
    display_tui::install_panic_hook();

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
                        if trimmed.is_empty() {
                            continue;
                        }
                        match WireMessage::from_json_line(trimmed.as_bytes()) {
                            Ok(WireMessage::Frame(frame)) => {
                                current_frame = Some(frame);
                            }
                            Ok(WireMessage::Cmd(DisplayCmd::Quit)) => return Ok(()),
                            Ok(WireMessage::Cmd(DisplayCmd::Bell)) => {
                                eprint!("\x07");
                            }
                            Ok(_) => {}
                            Err(_) => {}
                        }
                    }
                    Err(ref e)
                        if e.kind() == io::ErrorKind::WouldBlock
                            || e.kind() == io::ErrorKind::ConnectionAborted =>
                    {
                        break;
                    }
                    Err(e) => {
                        eprintln!("Read error: {}", e);
                        return Ok(());
                    }
                }
            }

            if let Some(ref frame) = current_frame {
                tui.render_frame(frame).map_err(|e| anyhow::anyhow!(e))?;
            }

            // poll_input returns display-protocol InputEvent directly
            if let Some(event) = tui.poll_input(16) {
                let msg = WireMessage::Input(event);
                stream.write_all(&msg.to_json_line().unwrap())?;
                stream.flush()?;
            }
        }
    })();

    tui.cleanup().map_err(|e| anyhow::anyhow!(e))?;
    result
}
