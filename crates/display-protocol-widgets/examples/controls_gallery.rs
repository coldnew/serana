use display_protocol::UiNode;
use display_protocol_widgets::*;

fn main() {
    section(
        "button variants",
        "[ Primary ] [ Secondary ] [ Outline ] [ Ghost ] [ Danger ]",
        UiNode::row(vec![
            Button::new("Primary").primary().build(),
            Button::new("Secondary").secondary().build(),
            Button::new("Outline").outline().build(),
            Button::new("Ghost").ghost().build(),
            Button::new("Danger").danger().build(),
        ])
        .gap(1),
    );

    section(
        "button states",
        "[ Focused ] [ Pressed ] [ Disabled ]",
        UiNode::row(vec![
            Button::new("Focused").focused(true).build(),
            Button::new("Pressed").pressed(true).build(),
            Button::new("Disabled").disabled(true).build(),
        ])
        .gap(1),
    );

    section(
        "checkbox and toggle states",
        "[ ] Unchecked\n[x] Checked\n[ ] Disabled\n[off] Feature A\n[on] Feature B",
        UiNode::column(vec![
            Checkbox::new("Unchecked").build(),
            Checkbox::new("Checked").checked(true).build(),
            Checkbox::new("Disabled").disabled(true).build(),
            Toggle::new("Feature A").build(),
            Toggle::new("Feature B").on(true).build(),
        ])
        .gap(1),
    );

    section(
        "text inputs",
        "[normal value          ]\n[focused value         ]\n[placeholder text      ]",
        UiNode::column(vec![
            TextInput::new("normal value").width(24).build(),
            TextInput::new("focused value")
                .width(24)
                .focused(true)
                .cursor_pos(7)
                .build(),
            TextInput::new("")
                .width(24)
                .placeholder("placeholder text")
                .build(),
        ])
        .gap(1),
    );

    section(
        "form with errors",
        "Email *\n[agent@example.test]\nToken *\n[        ]\nToken is required\n[ Cancel ] [ Save ]",
        Form::new()
            .field(
                FormField::new(
                    "Email",
                    TextInput::new("agent@example.test").width(28).build(),
                )
                .required(true)
                .build(),
            )
            .field(
                FormField::new("Token", TextInput::new("").width(28).focused(true).build())
                    .required(true)
                    .error("Token is required")
                    .build(),
            )
            .action(Button::new("Cancel").secondary().build())
            .action(Button::new("Save").primary().build())
            .build(),
    );

    section(
        "progress bars",
        "[##########----------] 50%\n[################----] 80%",
        UiNode::column(vec![
            ProgressBar::new(0.5).width(30).build(),
            ProgressBar::with_range(80.0, 100.0)
                .width(30)
                .show_percent(true)
                .build(),
        ])
        .gap(1),
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
