use display_protocol::{Border, Padding, Style, UiNode};
use display_protocol_widgets::*;

fn main() {
    section(
        "box, text, spacer",
        "+ Box +\n| Heading |\n| body    |",
        BoxComponent::new(vec![
            Text::new("Heading").bold().color(palette::PRIMARY).build(),
            Spacer::height(1).build(),
            Text::new("body").dim().build(),
        ])
        .title("Box")
        .padding(Padding::new(0, 1, 0, 1))
        .border(Border::all(Some(Style::default().fg(palette::PRIMARY))))
        .build(),
    );

    section(
        "loaders",
        "| Loading model    Esc to cancel",
        UiNode::column(vec![
            Loader::new(0).label("Loading model").build(),
            CancellableLoader::new("Streaming response", 2)
                .cancel_hint("Ctrl+C to stop")
                .build(),
        ])
        .gap(1),
    );

    section(
        "select list",
        "  gpt-5          Best reasoning\n> gpt-5-mini     Fast default\n  local-model    Offline",
        SelectList::new(vec![
            SelectItem::new("gpt-5", "gpt-5").description("Best reasoning"),
            SelectItem::new("gpt-5-mini", "gpt-5-mini").description("Fast default"),
            SelectItem::new("local", "local-model").description("Offline"),
        ])
        .selected(1)
        .build(),
    );

    section(
        "settings list",
        "> Model                         gpt-5-mini\n  Approval mode                 on-request",
        SettingsList::new(vec![
            SettingItem::new("Model", "gpt-5-mini").description("Current LLM"),
            SettingItem::new("Approval mode", "on-request").description("Shell permissions"),
        ])
        .selected(0)
        .build(),
    );

    section(
        "markdown",
        "# Release notes\n- Added shared components\n> Backend agnostic",
        Markdown::new("# Release notes\n- Added shared components\n> Backend agnostic").build(),
    );

    section(
        "truncated text",
        "very-long-file-na...",
        TruncatedText::new("very-long-file-name-that-does-not-fit.rs", 20)
            .suffix("...")
            .build(),
    );

    section(
        "input and editor aliases",
        "[prompt value]\n1 fn main() {\n2     println!(\"hi\");\n3 }",
        UiNode::column(vec![
            input_component("prompt value")
                .placeholder("message")
                .focused(true)
                .build(),
            editor_component(vec![
                "fn main() {".to_string(),
                "    println!(\"hi\");".to_string(),
                "}".to_string(),
            ])
            .cursor(1, 4)
            .height(5)
            .focused(true)
            .build(),
        ])
        .gap(1),
    );

    section(
        "image placeholder",
        "Canvas image frame_id=preview-image, 32x8",
        Image::new("preview-image", 32, 8)
            .background(palette::Color::new(8, 12, 18))
            .build(),
    );
}

fn section(title: &str, preview: &str, node: UiNode) {
    println!("\n=== {title} ===");
    println!("Preview:\n{preview}");
    println!("Node kind: {}", node_kind(&node));
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
