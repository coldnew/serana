mod common;

use display_protocol::UiNode;
use display_protocol_widgets::*;

fn main() {
    section(
        "card and separator",
        "+ Project +\n| serana-new |\n| ref/pi parity |\n------------",
        UiNode::column(vec![
            Card::new(vec![
                UiNode::text("serana-new"),
                UiNode::text("ref/pi parity"),
            ])
            .title("Project")
            .width(42)
            .build(),
            Separator::new().build(),
            Separator::new().dashed().build(),
            Separator::new().dotted().build(),
        ])
        .gap(1),
    );

    section(
        "accordion",
        "> Runtime\n  model: gpt-5\n  approval: on-request\n> Collapsed",
        UiNode::column(vec![
            Accordion::new("Runtime")
                .expanded(true)
                .children(vec![
                    UiNode::text("model: gpt-5"),
                    UiNode::text("approval: on-request"),
                ])
                .build(),
            Accordion::new("Collapsed").build(),
        ]),
    );

    section(
        "tabs and status bar",
        "[ app.rs ] [ slash_commands.rs* ] [ config.rs ]\nNORMAL | app.rs                 Rust | UTF-8",
        UiNode::column(vec![
            Tabs::new(vec!["app.rs", "slash_commands.rs", "config.rs"])
                .active(1)
                .modified(1, true)
                .build(),
            StatusBar::new()
                .left(vec![
                    UiNode::text("NORMAL").bold(),
                    UiNode::text(" | "),
                    UiNode::text("app.rs"),
                ])
                .right(vec![UiNode::text("Rust"), UiNode::text(" | "), UiNode::text("UTF-8")])
                .build(),
        ])
        .gap(1),
    );

    section(
        "table and tree",
        "Name       Kind\napp.rs     file\nsrc        folder\n\nsrc\n  tui\n    app.rs",
        UiNode::row(vec![
            Table::new(vec!["Name", "Kind"])
                .row(vec!["app.rs", "file"])
                .row(vec!["src", "folder"])
                .build(),
            TreeView::new(vec![
                TreeBranch::new("src")
                    .expanded(true)
                    .children(vec![TreeBranch::new("tui")
                        .expanded(true)
                        .children(vec![TreeBranch::new("app.rs")])]),
                TreeBranch::new("Cargo.toml"),
            ])
            .selected(Some(2))
            .build(),
        ])
        .gap(4),
    );

    section(
        "scroll and split pane",
        "+ Files --------+ | + Preview --------+\n| app.rs        | | | selected file   |\n| config.rs     | | | contents        |",
        SplitPane::horizontal(
            ScrollPane::new(List::dashed(vec![
                UiNode::text("app.rs"),
                UiNode::text("config.rs"),
                UiNode::text("slash_commands.rs"),
                UiNode::text("main.rs"),
            ]))
            .viewport(24, 6)
            .content_size(None, Some(20)),
            Card::new(vec![UiNode::text("selected file"), UiNode::text("contents")])
                .title("Preview"),
        )
        .ratio(0.4)
        .build(),
    );

    section(
        "popup and dialog",
        "Popup at (8, 3); modal confirm overlay centered by backend",
        UiNode::column(vec![
            Popup::new(
                Card::new(vec![UiNode::text("Quick action")])
                    .title("Menu")
                    .build(),
            )
            .at(8, 3)
            .z_index(30)
            .build(),
            Dialog::new("Delete session")
                .body(vec![UiNode::text("This cannot be undone.")])
                .action(Button::new("Cancel").secondary().build())
                .action(Button::new("Delete").danger().width(12).build())
                .width(42)
                .height(9)
                .build(),
        ]),
    );

    section(
        "canvas placeholder",
        "TUI reserves a 40x10 blank area; WGPU reads frame_id=minimap",
        Canvas::new(40, 10)
            .frame("minimap")
            .bg(palette::Color::new(12, 16, 24))
            .build(),
    );
}

fn section(title: &str, preview: &str, node: UiNode) {
    println!("\n=== {title} ===");
    println!("Reference shape:\n{preview}");
    common::print_node(&node, 80, 40);
}
