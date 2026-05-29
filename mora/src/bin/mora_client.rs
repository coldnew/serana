use clap::Parser;
use display_protocol::{
    DisplayCmd, FrameUpdate, WireMessage, PROTOCOL_VERSION,
};
use display_tui::TuiTerminal;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

#[derive(Parser)]
#[command(name = "mora-client")]
#[command(about = "Mora remote client - connect to a mora server")]
struct Cli {
    /// Server address (host:port)
    #[arg(default_value = "127.0.0.1:7890")]
    addr: String,
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

    match &hello {
        WireMessage::ServerHello { version, .. } => {
            eprintln!("Server protocol v{}", version);
        }
        _ => anyhow::bail!("Expected ServerHello, got {:?}", hello),
    };

    // Send ClientHello
    let client_hello = WireMessage::ClientHello {
        version: PROTOCOL_VERSION,
    };
    stream.write_all(&client_hello.to_json_line().unwrap())?;
    stream.flush()?;

    // Initialize TUI terminal
    let mut tui = TuiTerminal::new().map_err(|e| anyhow::anyhow!(e))?;
    tui.init().map_err(|e| anyhow::anyhow!(e))?;
    display_tui::install_panic_hook();

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
                tui.render_frame(frame).map_err(|e| anyhow::anyhow!(e))?;
            }

            // Poll local input — returns display-protocol InputEvent directly
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
