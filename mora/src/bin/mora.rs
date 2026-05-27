use clap::{Parser, Subcommand};
use mora_bin::mora::MoraEditor;
use mora_bin::mora::display::backend::{DisplayBackend, InputEvent};
use mora_bin::mora::display::tui::TuiBackend;
use mora_bin::mora::display::wgpu_backend::WgpuBackend;
use mora_bin::mora::display::style::MoraStyle;
use mora_compile::{CompileOptions, CompileTarget};
use ratatui::widgets::Widget;
use std::io::{self, Read};
use std::panic;
use std::path::Path;

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
            // Default: open editor
            run_editor(cli.file, cli.gui)
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

fn init_lisp_state(editor: &MoraEditor) {
    let mut state = mora_bin::mora::lisp_ext::EditorState::new();
    state.lines = editor.buffer.lines.clone();
    state.cursor_row = editor.buffer.cursor.row;
    state.cursor_col = editor.buffer.cursor.col;
    state.modified = editor.buffer.modified;
    state.file_path = editor.buffer.path.as_ref().map(|p| p.to_string_lossy().to_string());
    state.mode = format!("{:?}", editor.mode()).to_lowercase();
    state.window_count = editor.windows.len().max(1);
    mora_bin::mora::lisp_ext::set_editor_state(state);
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

    let (_, h) = backend.size();
    let editor_height = h.saturating_sub(3) as usize;
    let mut editor = if let Some(ref path) = file {
        MoraEditor::open(Path::new(path), editor_height)
            .map_err(|e| anyhow::anyhow!("Failed to open {}: {}", path, e))?
    } else {
        MoraEditor::new(editor_height)
    };

    init_lisp_state(&editor);
    editor.lisp_bridge.load_init_file();

    let result = (|| -> anyhow::Result<()> {
        loop {
            let editor_ref = &editor;
            backend
                .draw(|buf, area| {
                    let widget = mora_bin::mora::ui::EditorWidget::new(editor_ref);
                    widget.render(area, buf);
                })
                .map_err(|e| anyhow::anyhow!(e))?;

            match backend.poll_event(50) {
                Some(InputEvent::Key(key)) => {
                    if key.modifiers.ctrl && key.code == mora_bin::mora::display::event::MoraKeyCode::Char('c') {
                        break;
                    }
                    editor.handle_key(key);
                }
                Some(InputEvent::Resize(_nw, nh)) => {
                    editor.set_height(nh.saturating_sub(3) as usize);
                }
                _ => {}
            }

            if editor.quit_requested() {
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
    use ratatui::layout::Rect;

    let mut backend = WgpuBackend::new();
    backend.init().map_err(|e| anyhow::anyhow!(e))?;

    let (w, h) = backend.size();
    let editor_height = h.saturating_sub(3) as usize;
    let mut editor = if let Some(ref path) = file {
        MoraEditor::open(Path::new(path), editor_height)
            .map_err(|e| anyhow::anyhow!("Failed to open {}: {}", path, e))?
    } else {
        MoraEditor::new(editor_height)
    };

    init_lisp_state(&editor);
    editor.lisp_bridge.load_init_file();

    let result = (|| -> anyhow::Result<()> {
        loop {
            let (w, h) = backend.size();
            let area = Rect::new(0, 0, w, h);

            let editor_ref = &editor;
            let mut ratatui_buf = ratatui::buffer::Buffer::empty(area);
            let widget = mora_bin::mora::ui::EditorWidget::new(editor_ref);
            widget.render(area, &mut ratatui_buf);

            let mut cell_buf = CellBuffer::new(w, h);
            for y in 0..h {
                for x in 0..w {
                    let cell = ratatui_buf.get(x, y);
                    let fg = cell.fg.into();
                    let bg = cell.bg.into();
                    let style = MoraStyle { fg: Some(fg), bg: Some(bg), ..MoraStyle::default() };
                    cell_buf.set_cell(x, y, cell.symbol().chars().next().unwrap_or(' '), style);
                }
            }

            backend.render_buffer(&cell_buf).map_err(|e| anyhow::anyhow!(e))?;

            match backend.poll_event(50) {
                Some(InputEvent::Key(key)) => {
                    if key.modifiers.ctrl && key.code == mora_bin::mora::display::event::MoraKeyCode::Char('c') {
                        break;
                    }
                    editor.handle_key(key);
                }
                Some(InputEvent::Resize(_nw, nh)) => {
                    editor.set_height(nh.saturating_sub(3) as usize);
                }
                _ => {}
            }

            if editor.quit_requested() {
                break;
            }
        }
        Ok(())
    })();

    backend.cleanup().map_err(|e| anyhow::anyhow!(e))?;
    result
}
