use display_protocol::*;
use display_protocol_rsx::rsx;

#[test]
fn test_rsx_text_with_bold() {
    let node = rsx! {
        Text(bold) { "Hello" }
    };
    match &node {
        UiNode::Text(t) => {
            assert_eq!(t.content, "Hello");
            assert!(t.style.bold);
        }
        _ => panic!("expected Text"),
    }
}

#[test]
fn test_rsx_column_with_children() {
    let node = rsx! {
        Column(gap = 1) {
            Text(bold) { "first" }
            Text(dim) { "second" }
        }
    };
    match &node {
        UiNode::Column(f) => {
            assert_eq!(f.gap, 1);
            assert_eq!(f.children.len(), 2);
            match &f.children[0] {
                UiNode::Text(t) => assert_eq!(t.content, "first"),
                _ => panic!("expected Text"),
            }
        }
        _ => panic!("expected Column"),
    }
}

#[test]
fn test_rsx_box_with_border() {
    let node = rsx! {
        Box(title = "Panel", border) {
            Text() { "content" }
        }
    };
    match &node {
        UiNode::Box(b) => {
            assert_eq!(b.title, Some("Panel".to_string()));
            assert!(b.border.has_any());
            assert_eq!(b.children.len(), 1);
        }
        _ => panic!("expected Box"),
    }
}

#[test]
fn test_rsx_divider() {
    let node = rsx! {
        Divider(char = '=')
    };
    match &node {
        UiNode::Divider(d) => {
            assert_eq!(d.char, Some('='));
        }
        _ => panic!("expected Divider"),
    }
}

#[test]
fn test_rsx_progress() {
    let node = rsx! {
        Progress(value = 75.0, width = 20, show_percent = true)
    };
    match &node {
        UiNode::ProgressBar(p) => {
            assert_eq!(p.value, 75.0);
            assert_eq!(p.width, 20);
            assert!(p.show_percent);
        }
        _ => panic!("expected ProgressBar"),
    }
}

#[test]
fn test_rsx_input() {
    let node = rsx! {
        Input(value = "hello", placeholder = "type...", focused)
    };
    match &node {
        UiNode::Input(i) => {
            assert_eq!(i.value, "hello");
            assert_eq!(i.placeholder, "type...");
            assert!(i.focused);
        }
        _ => panic!("expected Input"),
    }
}

#[test]
fn test_rsx_nested_rows() {
    let node = rsx! {
        Column(gap = 2) {
            Row(gap = 1) {
                Text(color = Color::RED) { "R" }
                Text(color = Color::GREEN) { "G" }
            }
            Divider()
            Text(bold, dim) { "end" }
        }
    };
    match &node {
        UiNode::Column(c) => {
            assert_eq!(c.children.len(), 3);
            assert!(matches!(&c.children[0], UiNode::Row(_)));
            assert!(matches!(&c.children[1], UiNode::Divider(_)));
            assert!(matches!(&c.children[2], UiNode::Text(_)));
        }
        _ => panic!("expected Column"),
    }
}

#[test]
fn test_rsx_empty_tree() {
    let node = rsx! {};
    assert!(matches!(node, UiNode::None));
}

#[test]
fn test_rsx_expression_interpolation() {
    let extra = UiNode::text("extra");
    let node = rsx! {
        Column() {
            Text() { "hello" }
            { extra }
        }
    };
    match &node {
        UiNode::Column(c) => {
            assert_eq!(c.children.len(), 2);
        }
        _ => panic!("expected Column"),
    }
}
