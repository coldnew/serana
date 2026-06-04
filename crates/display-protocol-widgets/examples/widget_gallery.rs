use display_protocol::{Border, MenuItem, Padding, Style, UiNode};
use display_protocol_widgets::*;

fn main() {
    show(
        "Buttons",
        "[ Save ]  [ Cancel ]  [ Delete ]  [ Disabled ]",
        UiNode::row(vec![
            Button::new("Save").primary().focused(true).build(),
            Button::new("Cancel").secondary().build(),
            Button::new("Delete").danger().build(),
            Button::new("Disabled").outline().disabled(true).build(),
        ])
        .gap(1),
    );

    show(
        "Badges",
        "[default] [primary] [success] [warning] [danger] [info]",
        UiNode::row(vec![
            Badge::new("default").build(),
            Badge::new("primary").style(BadgeStyle::Primary).build(),
            Badge::new("success").style(BadgeStyle::Success).build(),
            Badge::new("warning").style(BadgeStyle::Warning).build(),
            Badge::new("danger").style(BadgeStyle::Danger).build(),
            Badge::new("info").style(BadgeStyle::Info).build(),
        ])
        .gap(1),
    );

    show(
        "Alerts",
        "info/success/warning/error boxes with icon, title, and message",
        UiNode::column(vec![
            Alert::new(AlertLevel::Info, "Connected to display backend")
                .title("Info")
                .build(),
            Alert::new(AlertLevel::Success, "Configuration saved")
                .title("Success")
                .build(),
            Alert::new(AlertLevel::Warning, "Low disk space")
                .title("Warning")
                .build(),
            Alert::new(AlertLevel::Error, "Failed to start session")
                .title("Error")
                .build(),
        ])
        .gap(1),
    );

    show(
        "Inputs",
        "Name *\n[serana                 ]\nSearch\n[Type to search...      ]",
        Form::new()
            .field(
                FormField::new(
                    "Name",
                    TextInput::new("serana")
                        .focused(true)
                        .cursor_pos(3)
                        .width(24)
                        .build(),
                )
                .required(true)
                .build(),
            )
            .field(
                FormField::new(
                    "Search",
                    TextInput::new("")
                        .placeholder("Type to search...")
                        .width(24)
                        .build(),
                )
                .build(),
            )
            .action(Button::new("Submit").primary().build())
            .action(Button::new("Reset").ghost().build())
            .build(),
    );

    show(
        "Selection Controls",
        "[x] Enable tools\n[ ] Read-only mode\n[on] Stream responses\n[off] Telemetry",
        UiNode::column(vec![
            Checkbox::new("Enable tools").checked(true).build(),
            Checkbox::new("Read-only mode").build(),
            Toggle::new("Stream responses").on(true).build(),
            Toggle::new("Telemetry").disabled(true).build(),
        ])
        .gap(1),
    );

    show(
        "Data Display",
        "Tabs, table, tree, list, and progress bar",
        UiNode::column(vec![
            Tabs::new(vec!["Files", "Search", "Git"])
                .active(1)
                .modified(2, true)
                .build(),
            Table::new(vec!["Name", "State", "Owner"])
                .row(vec!["serana", "active", "agent"])
                .row(vec!["display", "ready", "protocol"])
                .build(),
            TreeView::new(vec![
                TreeBranch::new("src")
                    .expanded(true)
                    .children(vec![TreeBranch::new("main.rs"), TreeBranch::new("lib.rs")]),
                TreeBranch::new("Cargo.toml"),
            ])
            .selected(Some(1))
            .build(),
            List::bullets(vec![
                UiNode::text("Build declarative nodes"),
                UiNode::text("Render in TUI or WGPU backend"),
            ])
            .build(),
            ProgressBar::new(0.72).width(30).build(),
        ])
        .gap(1),
    );

    show(
        "Layout",
        "Card, accordion, separator, scroll pane, split pane, popup, dialog",
        UiNode::column(vec![
            Card::new(vec![UiNode::text("Session ready"), link("Open docs")])
                .title("Card")
                .width(40)
                .build(),
            Accordion::new("Advanced")
                .expanded(true)
                .child(UiNode::text("Temperature: 0.2"))
                .child(UiNode::text("Top-p: 0.9"))
                .build(),
            Separator::new().dashed().build(),
            ScrollPane::new(List::numbered(vec![
                UiNode::text("First visible line"),
                UiNode::text("Second visible line"),
                UiNode::text("More content below"),
            ]))
            .viewport(40, 4)
            .content_size(None, Some(30))
            .build(),
            SplitPane::horizontal(
                Card::new(vec![UiNode::text("Left pane")]).title("Files"),
                Card::new(vec![UiNode::text("Right pane")]).title("Preview"),
            )
            .ratio(0.35)
            .build(),
            Popup::new(Card::new(vec![UiNode::text("Tooltip text")]).build())
                .at(4, 3)
                .z_index(20)
                .build(),
            Dialog::new("Confirm")
                .body(vec![UiNode::text("Apply these changes?")])
                .actions(vec![
                    Button::new("Cancel").secondary().build(),
                    Button::new("Apply").primary().build(),
                ])
                .width(36)
                .height(8)
                .build(),
        ])
        .gap(1),
    );

    show(
        "Text Helpers",
        "`Ctrl+K` keycap, link, spinner, breadcrumb, span",
        UiNode::column(vec![
            UiNode::row(vec![kbd("Ctrl+K"), link("Command palette"), spinner(3)]).gap(1),
            breadcrumb(vec![UiNode::text("workspace"), UiNode::text("serana")]),
            UiNode::row(vec![
                Span::new("bold").bold().fg(palette::PRIMARY).build(),
                Span::new("underlined")
                    .underline()
                    .fg(palette::WARNING)
                    .build(),
                Span::new("dim").dim().fg(palette::MUTED).build(),
            ])
            .gap(1),
        ])
        .gap(1),
    );

    show_menu();

    show(
        "Custom Box Component",
        "+ Shared +\n| Pi-style component wrappers |",
        BoxComponent::new(vec![Text::new("Pi-style component wrappers").build()])
            .padding(Padding::new(0, 1, 0, 1))
            .border(Border::all(Some(Style::default().fg(palette::PRIMARY))))
            .title("Shared")
            .build(),
    );
}

fn show(title: &str, preview: &str, node: UiNode) {
    println!("\n=== {title} ===");
    println!("Preview:\n{preview}");
    println!("Node kind: {}", node_kind(&node));
}

fn show_menu() {
    let menu = Menu::new()
        .item(MenuItem::new("Copy").shortcut("Ctrl+C"))
        .item(MenuItem::new("Paste").shortcut("Ctrl+V"))
        .separator()
        .item(MenuItem::new("Delete").shortcut("Del").disabled())
        .at(10, 4)
        .build();

    println!("\n=== Menu ===");
    println!("Preview:\nCopy        Ctrl+C\nPaste       Ctrl+V\n-----------\nDelete      Del");
    println!(
        "Menu items: {}, anchor: {:?}",
        menu.items.len(),
        menu.anchor
    );
}

fn node_kind(node: &UiNode) -> &'static str {
    match node {
        UiNode::None => "None",
        UiNode::Text(_) => "Text",
        UiNode::Span(_) => "Span",
        UiNode::Box(_) => "Box",
        UiNode::Row(_) => "Row",
        UiNode::Column(_) => "Column",
        UiNode::Divider(_) => "Divider",
        UiNode::ProgressBar(_) => "ProgressBar",
        UiNode::List(_) => "List",
        UiNode::ListItem(_) => "ListItem",
        UiNode::Show { .. } => "Show",
        UiNode::For { .. } => "For",
        UiNode::Keyed { .. } => "Keyed",
        UiNode::Input(_) => "Input",
        UiNode::TextArea(_) => "TextArea",
        UiNode::TabBar(_) => "TabBar",
        UiNode::TreeView(_) => "TreeView",
        UiNode::SplitPane(_) => "SplitPane",
        UiNode::Canvas(_) => "Canvas",
        UiNode::Overlay(_) => "Overlay",
        UiNode::ScrollView(_) => "ScrollView",
        UiNode::StatusBar(_) => "StatusBar",
        UiNode::Table(_) => "Table",
    }
}
