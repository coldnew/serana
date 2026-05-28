use crate::ui::*;

/// Declarative JSX-like macro for building UiNode trees.
///
/// # Syntax
///
/// - Self-closing: `Divider()`
/// - With props: `Text(bold, color = "red")`
/// - With children: `Box(title = "Panel") { Text(bold) { "Hello" } }`
/// - Text content: `{ "literal" }` or `{ expr }`
///
/// # Example
///
/// ```ignore
/// let tree = rsx! {
///     Column(gap = 1) {
///         Text(bold) { "Hello, World!" }
///         Divider()
///         Row(gap = 2) {
///             Text(color = Color::new(255, 0, 0)) { "Red" }
///             Text(color = Color::new(0, 255, 0)) { "Green" }
///         }
///     }
/// };
/// ```
#[macro_export]
macro_rules! rsx {
    // Empty tree
    () => { $crate::UiNode::None };

    // Text with no props
    (Text () { $($child:tt)* }) => {{
        let mut __node = $crate::UiNode::Text($crate::TextNode {
            content: String::new(),
            style: $crate::Style::default(),
            wrap: $crate::Wrap::default(),
        });
        rsx!(@text_content __node $($child)*);
        __node
    }};

    // Text with props
    (Text ( $($prop:ident $(= $val:expr)?),* $(,)? ) { $($child:tt)* }) => {{
        let mut __text = $crate::TextNode {
            content: String::new(),
            style: $crate::Style::default(),
            wrap: $crate::Wrap::default(),
        };
        $(
            rsx!(@apply_text_prop __text $prop $(= $val)?);
        )*
        let mut __node = $crate::UiNode::Text(__text);
        rsx!(@text_content __node $($child)*);
        __node
    }};

    // Self-closing text
    (Text ( $($prop:ident $(= $val:expr)?),* $(,)? )) => {{
        let mut __text = $crate::TextNode {
            content: String::new(),
            style: $crate::Style::default(),
            wrap: $crate::Wrap::default(),
        };
        $(
            rsx!(@apply_text_prop __text $prop $(= $val)?);
        )*
        $crate::UiNode::Text(__text)
    }};

    // Box with children
    (Box () { $($child:tt)* }) => {{
        let mut __node = $crate::UiNode::boxed(vec![]);
        rsx!(@push_children __node $($child)*);
        __node
    }};

    // Box with props and children
    (Box ( $($prop:ident $(= $val:expr)?),* $(,)? ) { $($child:tt)* }) => {{
        let mut __node = $crate::UiNode::boxed(vec![]);
        $(
            rsx!(@apply_box_prop __node $prop $(= $val)?);
        )*
        rsx!(@push_children __node $($child)*);
        __node
    }};

    // Self-closing Box
    (Box ( $($prop:ident $(= $val:expr)?),* $(,)? )) => {{
        let mut __node = $crate::UiNode::boxed(vec![]);
        $(
            rsx!(@apply_box_prop __node $prop $(= $val)?);
        )*
        __node
    }};

    // Row with children
    (Row () { $($child:tt)* }) => {{
        let mut __node = $crate::UiNode::row(vec![]);
        rsx!(@push_children __node $($child)*);
        __node
    }};

    // Row with props and children
    (Row ( $($prop:ident $(= $val:expr)?),* $(,)? ) { $($child:tt)* }) => {{
        let mut __node = $crate::UiNode::row(vec![]);
        $(
            rsx!(@apply_flex_prop __node $prop $(= $val)?);
        )*
        rsx!(@push_children __node $($child)*);
        __node
    }};

    // Column with children
    (Column () { $($child:tt)* }) => {{
        let mut __node = $crate::UiNode::column(vec![]);
        rsx!(@push_children __node $($child)*);
        __node
    }};

    // Column with props and children
    (Column ( $($prop:ident $(= $val:expr)?),* $(,)? ) { $($child:tt)* }) => {{
        let mut __node = $crate::UiNode::column(vec![]);
        $(
            rsx!(@apply_flex_prop __node $prop $(= $val)?);
        )*
        rsx!(@push_children __node $($child)*);
        __node
    }};

    // Divider self-closing
    (Divider ()) => {{ $crate::UiNode::divider() }};

    // Divider with props
    (Divider ( $($prop:ident $(= $val:expr)?),* $(,)? )) => {{
        let mut __node = $crate::UiNode::divider();
        $(
            rsx!(@apply_divider_prop __node $prop $(= $val)?);
        )*
        __node
    }};

    // Progress self-closing
    (Progress ( $($prop:ident $(= $val:expr)?),* $(,)? )) => {{
        let mut __node = $crate::UiNode::progress(0.0, 100.0);
        $(
            rsx!(@apply_progress_prop __node $prop $(= $val)?);
        )*
        __node
    }};

    // List with children
    (List ( $($prop:ident $(= $val:expr)?),* $(,)? ) { $($child:tt)* }) => {{
        let mut __node = $crate::UiNode::list(vec![]);
        $(
            rsx!(@apply_list_prop __node $prop $(= $val)?);
        )*
        rsx!(@push_children __node $($child)*);
        __node
    }};

    // Show with when condition and child
    (Show ( when = $cond:expr ) { $($child:tt)* }) => {{
        let __child = rsx!(@single_child $($child)*);
        $crate::UiNode::show($cond, __child)
    }};

    // ── Single child helper (for Show) ──
    (@single_child $Tag:ident ( $($prop:ident $(= $val:expr)?),* $(,)? ) { $($inner:tt)* }) => {
        rsx!($Tag ( $($prop $(= $val)?),* ) { $($inner)* })
    };
    (@single_child $Tag:ident ( $($prop:ident $(= $val:expr)?),* $(,)? )) => {
        rsx!($Tag ( $($prop $(= $val)?),* ))
    };
    (@single_child $Tag:ident () { $($inner:tt)* }) => {
        rsx!($Tag () { $($inner)* })
    };
    (@single_child $Tag:ident ()) => {
        rsx!($Tag ())
    };
    (@single_child { $expr:expr }) => {
        $expr
    };

    // ── Prop application ──

    // Text props
    (@apply_text_prop $text:ident bold) => { $text.style = $text.style.bold(); };
    (@apply_text_prop $text:ident italic) => { $text.style = $text.style.italic(); };
    (@apply_text_prop $text:ident underline) => { $text.style = $text.style.underline(); };
    (@apply_text_prop $text:ident dim) => { $text.style = $text.style.dim(); };
    (@apply_text_prop $text:ident strikethrough) => { $text.style = $text.style.strikethrough(); };
    (@apply_text_prop $text:ident reverse) => { $text.style = $text.style.reverse(); };
    (@apply_text_prop $text:ident color = $val:expr) => { $text.style.fg = Some($val); };
    (@apply_text_prop $text:ident bg = $val:expr) => { $text.style.bg = Some($val); };
    (@apply_text_prop $text:ident wrap = $val:expr) => { $text.wrap = $val; };

    // Box props
    (@apply_box_prop $node:ident title = $val:expr) => {
        if let $crate::UiNode::Box(ref mut b) = $node { b.title = Some($val.to_string()); }
    };
    (@apply_box_prop $node:ident width = $val:expr) => {
        if let $crate::UiNode::Box(ref mut b) = $node { b.width = Some($val); }
    };
    (@apply_box_prop $node:ident height = $val:expr) => {
        if let $crate::UiNode::Box(ref mut b) = $node { b.height = Some($val); }
    };
    (@apply_box_prop $node:ident padding = $val:expr) => {
        if let $crate::UiNode::Box(ref mut b) = $node { b.padding = $val; }
    };
    (@apply_box_prop $node:ident border) => {
        if let $crate::UiNode::Box(ref mut b) = $node { b.border = $crate::Border::all(None); }
    };
    (@apply_box_prop $node:ident style = $val:expr) => {
        if let $crate::UiNode::Box(ref mut b) = $node { b.style = $val; }
    };

    // Flex props (Row/Column)
    (@apply_flex_prop $node:ident gap = $val:expr) => {
        match $node {
            $crate::UiNode::Row(ref mut f) | $crate::UiNode::Column(ref mut f) => { f.gap = $val; }
            _ => {}
        }
    };
    (@apply_flex_prop $node:ident align = $val:expr) => {
        match $node {
            $crate::UiNode::Row(ref mut f) | $crate::UiNode::Column(ref mut f) => { f.align = $val; }
            _ => {}
        }
    };
    (@apply_flex_prop $node:ident justify = $val:expr) => {
        match $node {
            $crate::UiNode::Row(ref mut f) | $crate::UiNode::Column(ref mut f) => { f.justify = $val; }
            _ => {}
        }
    };
    (@apply_flex_prop $node:ident padding = $val:expr) => {
        match $node {
            $crate::UiNode::Row(ref mut f) | $crate::UiNode::Column(ref mut f) => { f.padding = $val; }
            _ => {}
        }
    };
    (@apply_flex_prop $node:ident width = $val:expr) => {
        match $node {
            $crate::UiNode::Row(ref mut f) | $crate::UiNode::Column(ref mut f) => { f.width = Some($val); }
            _ => {}
        }
    };
    (@apply_flex_prop $node:ident height = $val:expr) => {
        match $node {
            $crate::UiNode::Row(ref mut f) | $crate::UiNode::Column(ref mut f) => { f.height = Some($val); }
            _ => {}
        }
    };
    (@apply_flex_prop $node:ident flex_grow = $val:expr) => {
        match $node {
            $crate::UiNode::Row(ref mut f) | $crate::UiNode::Column(ref mut f) => { f.flex_grow = $val; }
            _ => {}
        }
    };

    // Divider props
    (@apply_divider_prop $node:ident char = $val:expr) => {
        if let $crate::UiNode::Divider(ref mut d) = $node { d.char = Some($val); }
    };
    (@apply_divider_prop $node:ident style = $val:expr) => {
        if let $crate::UiNode::Divider(ref mut d) = $node { d.style = $val; }
    };

    // Progress props
    (@apply_progress_prop $node:ident value = $val:expr) => {
        if let $crate::UiNode::ProgressBar(ref mut p) = $node { p.value = $val; }
    };
    (@apply_progress_prop $node:ident width = $val:expr) => {
        if let $crate::UiNode::ProgressBar(ref mut p) = $node { p.width = $val; }
    };
    (@apply_progress_prop $node:ident show_percent = $val:expr) => {
        if let $crate::UiNode::ProgressBar(ref mut p) = $node { p.show_percent = $val; }
    };

    // List props
    (@apply_list_prop $node:ident marker = $val:expr) => {
        if let $crate::UiNode::List(ref mut l) = $node { l.marker = $val; }
    };

    // ── Text content ──
    (@text_content $node:ident $text:literal $($rest:tt)*) => {
        if let $crate::UiNode::Text(ref mut t) = $node {
            t.content.push_str($text);
        }
    };
    (@text_content $node:ident { $expr:expr } $($rest:tt)*) => {
        if let $crate::UiNode::Text(ref mut t) = $node {
            t.content.push_str(&$expr.to_string());
        }
    };
    (@text_content $node:ident) => {};

    // ── Push children ──

    // String literal child → text node
    (@push_children $node:ident $text:literal $($rest:tt)*) => {
        $node = $crate::UiNode::add_child_to($node, $crate::UiNode::text($text));
        rsx!(@push_children $node $($rest)*);
    };

    // Expression interpolation
    (@push_children $node:ident { $expr:expr } $($rest:tt)*) => {
        $node = $crate::UiNode::add_child_to($node, $crate::UiNode::text(&$expr.to_string()));
        rsx!(@push_children $node $($rest)*);
    };

    // Nested component with props and children
    (@push_children $node:ident $Tag:ident ( $($prop:ident $(= $val:expr)?),* $(,)? ) { $($inner:tt)* } $($rest:tt)*) => {
        $node = $crate::UiNode::add_child_to($node, rsx!($Tag ( $($prop $(= $val)?),* ) { $($inner)* }));
        rsx!(@push_children $node $($rest)*);
    };

    // Nested component with children, no props
    (@push_children $node:ident $Tag:ident () { $($inner:tt)* } $($rest:tt)*) => {
        $node = $crate::UiNode::add_child_to($node, rsx!($Tag () { $($inner)* }));
        rsx!(@push_children $node $($rest)*);
    };

    // Self-closing with props
    (@push_children $node:ident $Tag:ident ( $($prop:ident $(= $val:expr)?),* $(,)? ) $($rest:tt)*) => {
        $node = $crate::UiNode::add_child_to($node, rsx!($Tag ( $($prop $(= $val)?),* )));
        rsx!(@push_children $node $($rest)*);
    };

    // Self-closing without props
    (@push_children $node:ident $Tag:ident () $($rest:tt)*) => {
        $node = $crate::UiNode::add_child_to($node, rsx!($Tag ()));
        rsx!(@push_children $node $($rest)*);
    };

    // Expression child (UiNode) — must be wrapped in braces
    (@push_children $node:ident { $expr:expr } $($rest:tt)*) => {
        $node = $crate::UiNode::add_child_to($node, $expr);
        rsx!(@push_children $node $($rest)*);
    };

    // Terminal: no more children
    (@push_children $node:ident) => {};
}

impl UiNode {
    /// Helper for the rsx! macro — add a child to a container node.
    pub fn add_child_to(mut node: UiNode, child: UiNode) -> UiNode {
        match &mut node {
            UiNode::Box(b) => b.children.push(child),
            UiNode::Row(f) | UiNode::Column(f) => f.children.push(child),
            UiNode::List(l) => l.items.push(child),
            _ => {}
        }
        node
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_rsx_text() {
        let node = rsx!(Text(bold) { "Hello" });
        match &node {
            UiNode::Text(t) => {
                assert_eq!(t.content, "Hello");
                assert!(t.style.bold);
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn test_rsx_box_with_children() {
        let node = rsx! {
            Box(title = "Panel", border) {
                Text(bold) { "Header" }
                Text(dim) { "Content" }
            }
        };
        match &node {
            UiNode::Box(b) => {
                assert_eq!(b.title.as_deref(), Some("Panel"));
                assert!(b.border.has_any());
                assert_eq!(b.children.len(), 2);
            }
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn test_rsx_row_column() {
        let node = rsx! {
            Column(gap = 1) {
                Text() { "Line 1" }
                Row(gap = 2) {
                    Text() { "A" }
                    Text() { "B" }
                }
            }
        };
        match &node {
            UiNode::Column(c) => {
                assert_eq!(c.gap, 1);
                assert_eq!(c.children.len(), 2);
            }
            _ => panic!("expected Column"),
        }
    }

    #[test]
    fn test_rsx_divider() {
        let node = rsx!(Divider());
        assert!(matches!(node, UiNode::Divider(_)));
    }

    #[test]
    fn test_rsx_nested() {
        let node = rsx! {
            Column(gap = 1) {
                Text(bold, color = Color::new(255, 255, 0)) { "Title" }
                Divider()
                Box(padding = Padding::all(1)) {
                    Text(dim) { "Content area" }
                }
            }
        };
        match &node {
            UiNode::Column(c) => {
                assert_eq!(c.children.len(), 3);
                assert!(matches!(c.children[1], UiNode::Divider(_)));
            }
            _ => panic!("expected Column"),
        }
    }

    #[test]
    fn test_rsx_text_no_children() {
        let node = rsx!(Text(bold, color = Color::new(255, 0, 0)));
        match &node {
            UiNode::Text(t) => {
                assert!(t.style.bold);
                assert_eq!(t.style.fg, Some(Color::new(255, 0, 0)));
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn test_rsx_show() {
        let items = vec!["a", "b", "c"];
        let node = rsx! {
            Column() {
                Text(bold) { "Header" }
                { UiNode::show(true, UiNode::text("conditional")) }
                { UiNode::for_each(items, |s| UiNode::text(s)) }
            }
        };
        match &node {
            UiNode::Column(c) => {
                assert_eq!(c.children.len(), 3);
            }
            _ => panic!("expected Column"),
        }
    }

    #[test]
    fn test_rsx_show_macro() {
        let node = rsx! {
            Column() {
                Text(bold) { "Header" }
                Show(when = true) {
                    Text() { "visible" }
                }
            }
        };
        match &node {
            UiNode::Column(c) => {
                assert_eq!(c.children.len(), 2);
                assert!(matches!(&c.children[1], UiNode::Show { when: true, .. }));
            }
            _ => panic!("expected Column"),
        }
    }

    #[test]
    fn test_rsx_for_each() {
        let items = vec!["a", "b", "c"];
        let node = UiNode::for_each(items, |s| UiNode::text(s));
        match &node {
            UiNode::For { children } => assert_eq!(children.len(), 3),
            _ => panic!("expected For"),
        }
    }
}
