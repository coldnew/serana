use clap::Parser;
use display_protocol::{DisplayCmd, FrameUpdate, InputEvent, WireMessage, PROTOCOL_VERSION};
use mora_bin::mora::editor_core::MoraCore;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::Path;

#[derive(Parser)]
#[command(name = "mora-server")]
#[command(about = "Mora headless server - accept TCP client connections")]
struct Cli {
    /// File to open in the editor
    file: Option<String>,

    /// TCP port to listen on
    #[arg(short, long, default_value = "7890")]
    port: u16,

    /// Initial terminal width
    #[arg(long, default_value = "120")]
    width: u16,

    /// Initial terminal height
    #[arg(long, default_value = "40")]
    height: u16,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let addr = format!("127.0.0.1:{}", cli.port);

    let mut core = match &cli.file {
        Some(path) => MoraCore::open(Path::new(path), cli.width, cli.height)
            .map_err(|e| anyhow::anyhow!(e))?,
        None => MoraCore::new(cli.width, cli.height),
    };

    // Load init file
    {
        use mora_bin::mora::lisp_ext::EditorState;
        let mut state = EditorState::new();
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
    core.editor.lisp_bridge.load_init_file();

    eprintln!("mora-server listening on {}", addr);
    let listener = TcpListener::bind(&addr)?;

    for stream in listener.incoming() {
        let mut stream = stream?;
        eprintln!("Client connected: {}", stream.peer_addr()?);

        let mut reader = BufReader::new(stream.try_clone()?);

        // Send hello
        let hello = WireMessage::ServerHello {
            version: PROTOCOL_VERSION,
            width: cli.width,
            height: cli.height,
        };
        stream.write_all(&hello.to_json_line().unwrap())?;
        stream.flush()?;

        // Send initial frame
        let frame = core.render_frame();
        let msg = WireMessage::Frame(frame);
        stream.write_all(&msg.to_json_line().unwrap())?;
        stream.flush()?;

        // Main loop: read input from client, send frames back
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
                                        let msg = WireMessage::Cmd(DisplayCmd::Quit);
                                        let _ = stream.write_all(&msg.to_json_line().unwrap());
                                        let _ = stream.flush();
                                        eprintln!("Quit requested");
                                        return Ok(());
                                    }
                                    other => {
                                        let msg = WireMessage::Cmd(other.clone());
                                        let _ = stream.write_all(&msg.to_json_line().unwrap());
                                    }
                                }
                            }
                            let _ = stream.flush();

                            // Send updated frame
                            let frame = core.render_frame();
                            let msg = WireMessage::Frame(frame);
                            stream.write_all(&msg.to_json_line().unwrap())?;
                            stream.flush()?;
                        }
                        Ok(WireMessage::ClientHello { version }) => {
                            eprintln!("Client protocol version: {}", version);
                        }
                        Ok(_) => {
                            // Ignore unexpected message types from client
                        }
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
