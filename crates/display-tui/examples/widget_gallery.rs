use display_protocol::{InputEvent, KeyCode};
use display_protocol_widgets::{
    BoxSpec, CancellableLoaderSpec, InputSpec, LoaderSpec, MarkdownSpec, SelectItemSpec,
    SelectListSpec, SettingItemSpec, SettingsListSpec, SpacerSpec, TextSpec, TruncatedTextSpec,
    WidgetSpec,
};
use display_tui::{render_widget_spec_to_lines, TuiTerminal};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use std::{io, time::Duration};

fn main() -> io::Result<()> {
    let mut terminal = TuiTerminal::new()?;
    display_tui::install_panic_hook();
    terminal.init()?;

    let result = run_gallery(&mut terminal);
    terminal.cleanup()?;
    result
}

fn run_gallery(terminal: &mut TuiTerminal) -> io::Result<()> {
    let mut selected = 0usize;
    let sections = gallery_sections();

    loop {
        terminal.draw(|buf, area| render_gallery(buf, area, &sections, selected))?;
        if let Some(event) = terminal.poll_input(Duration::from_millis(100).as_millis() as u64) {
            match event {
                InputEvent::Key(key) => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => break,
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => {
                        selected = (selected + 1).min(sections.len().saturating_sub(1))
                    }
                    _ => {}
                },
                InputEvent::Resize { .. } => {}
                _ => {}
            }
        }
    }

    Ok(())
}

fn render_gallery(buf: &mut Buffer, area: Rect, sections: &[GallerySection], selected: usize) {
    let root = Block::default()
        .title(" display-tui widget gallery ")
        .title_bottom(" q/Esc quit · ↑/↓ select ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));
    let inner = root.inner(area);
    root.render(area, buf);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(20)])
        .split(inner);

    render_nav(buf, chunks[0], sections, selected);
    render_section(buf, chunks[1], &sections[selected]);
}

fn render_nav(buf: &mut Buffer, area: Rect, sections: &[GallerySection], selected: usize) {
    let lines = sections
        .iter()
        .enumerate()
        .map(|(index, section)| {
            let marker = if index == selected { "→" } else { " " };
            let style = if index == selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::from(vec![Span::styled(
                format!("{marker} {}", section.title),
                style,
            )])
        })
        .collect::<Vec<_>>();

    Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Components ")
                .borders(Borders::RIGHT),
        )
        .render(area, buf);
}

fn render_section(buf: &mut Buffer, area: Rect, section: &GallerySection) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(5)])
        .split(area);

    Paragraph::new(section.summary)
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: false })
        .render(chunks[0], buf);

    let render_width = chunks[1].width.saturating_sub(4).max(1);
    let mut lines = render_widget_spec_to_lines(&section.spec, render_width);
    display_tui::trim_rendered_lines(&mut lines);
    let lines = lines.into_iter().map(Line::from).collect::<Vec<_>>();

    Paragraph::new(lines)
        .block(
            Block::default()
                .title(section.title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .wrap(Wrap { trim: false })
        .render(chunks[1], buf);
}

struct GallerySection {
    title: &'static str,
    summary: &'static str,
    spec: WidgetSpec,
}

fn gallery_sections() -> Vec<GallerySection> {
    vec![
        GallerySection {
            title: "Text",
            summary: "Pi-style Text: wraps text, applies horizontal and vertical padding.",
            spec: TextSpec::new("Serana can share one widget protocol while each backend owns its renderer.")
                .padding(2, 1)
                .into(),
        },
        GallerySection {
            title: "Box",
            summary: "Box is a protocol container. TUI rendering pads child lines to the available width.",
            spec: BoxSpec::new(vec![
                TextSpec::new("Model: gpt-5").padding(0, 0).into(),
                SpacerSpec::new(1).into(),
                TextSpec::new("Approval: on-request").padding(0, 0).into(),
            ])
            .padding(2, 1)
            .into(),
        },
        GallerySection {
            title: "Input",
            summary: "Input carries prompt, value, cursor, and focus state; display-tui renders the cursor.",
            spec: InputSpec::new("explain display protocol widgets")
                .cursor(8)
                .focused(true)
                .into(),
        },
        GallerySection {
            title: "Loader",
            summary: "Loader protocol stores message and animation frame; display-tui chooses terminal glyphs.",
            spec: LoaderSpec::new("Loading conversation context").frame(3).into(),
        },
        GallerySection {
            title: "Cancellable Loader",
            summary: "CancellableLoader extends Loader state with a backend-neutral cancel hint.",
            spec: CancellableLoaderSpec::new("Calling tool")
                .frame(5)
                .cancel_hint("Esc to cancel")
                .into(),
        },
        GallerySection {
            title: "Select List",
            summary: "SelectList mirrors ref/pi: selected cursor, primary column, optional description column.",
            spec: SelectListSpec::new(vec![
                SelectItemSpec::new("model", "Model").description("Change active model"),
                SelectItemSpec::new("login", "Login").description("Authenticate provider"),
                SelectItemSpec::new("export", "Export").description("Write session to disk"),
                SelectItemSpec::new("copy", "Copy").description("Copy last assistant output"),
            ])
            .selected(1)
            .max_visible(4)
            .into(),
        },
        GallerySection {
            title: "Settings List",
            summary: "SettingsList carries labels, values, selected item, description, and hint text.",
            spec: SettingsListSpec::new(vec![
                SettingItemSpec::new("model", "Model", "gpt-5"),
                SettingItemSpec::new("reasoning", "Reasoning", "high"),
                SettingItemSpec::new("approval", "Approval Mode", "on-request")
                    .description("Ask before commands that need elevated permissions."),
            ])
            .selected(2)
            .into(),
        },
        GallerySection {
            title: "Markdown",
            summary: "Markdown is protocol text. TUI backend handles terminal markdown conventions.",
            spec: MarkdownSpec::new("# Release notes\n- Shared widget protocol\n> Rendered by display-tui")
                .padding(1, 1)
                .into(),
        },
        GallerySection {
            title: "Truncated Text",
            summary: "TruncatedText keeps long terminal rows bounded by backend width.",
            spec: TruncatedTextSpec::new(
                "very-long-workspace-path/ref/pi/packages/tui/src/components/select-list.ts",
            )
            .padding(1, 1)
            .into(),
        },
    ]
}
