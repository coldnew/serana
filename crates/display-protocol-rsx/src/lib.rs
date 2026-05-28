//! display-protocol-rsx: declarative JSX-like macro for building UiNode trees.
//!
//! This crate provides the `rsx!` macro, extracted from `display-protocol`
//! as a separate crate for cleaner dependency management.
//!
//! # Usage
//!
//! ```ignore
//! use display_protocol_rsx::rsx;
//! use display_protocol::*;
//!
//! let tree = rsx! {
//!     Column(gap = 1) {
//!         Text(bold) { "Hello, World!" }
//!         Divider()
//!         Row(gap = 2) {
//!             Text(color = Color::new(255, 0, 0)) { "Red" }
//!             Text(color = Color::new(0, 255, 0)) { "Green" }
//!         }
//!     }
//! };
//! ```

// Re-export everything from display-protocol so $crate:: resolves correctly.
pub use display_protocol::*;

/// Declarative JSX-like macro for building UiNode trees.
///
/// # Syntax
///
/// - Self-closing: `Divider()`
/// - With props: `Text(bold, color = "red")`
/// - With children: `Box(title = "Panel") { Text(bold) { "Hello" } }`
/// - Text content: `{ "literal" }` or `{ expr }`
#[macro_export]
macro_rules! rsx {
    // Empty tree
    () => { $crate::UiNode::None };

    // ── Text ──

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

    // ── Box ──

    (Box () { $($child:tt)* }) => {{
        let mut __node = $crate::UiNode::boxed(vec![]);
        rsx!(@push_children __node $($child)*);
        __node
    }};

    (Box ( $($prop:ident $(= $val:expr)?),* $(,)? ) { $($child:tt)* }) => {{
        let mut __node = $crate::UiNode::boxed(vec![]);
        $(
            rsx!(@apply_box_prop __node $prop $(= $val)?);
        )*
        rsx!(@push_children __node $($child)*);
        __node
    }};

    (Box ( $($prop:ident $(= $val:expr)?),* $(,)? )) => {{
        let mut __node = $crate::UiNode::boxed(vec![]);
        $(
            rsx!(@apply_box_prop __node $prop $(= $val)?);
        )*
        __node
    }};

    // ── Row / Column ──

    (Row () { $($child:tt)* }) => {{
        let mut __node = $crate::UiNode::row(vec![]);
        rsx!(@push_children __node $($child)*);
        __node
    }};

    (Row ( $($prop:ident $(= $val:expr)?),* $(,)? ) { $($child:tt)* }) => {{
        let mut __node = $crate::UiNode::row(vec![]);
        $(
            rsx!(@apply_flex_prop __node $prop $(= $val)?);
        )*
        rsx!(@push_children __node $($child)*);
        __node
    }};

    (Column () { $($child:tt)* }) => {{
        let mut __node = $crate::UiNode::column(vec![]);
        rsx!(@push_children __node $($child)*);
        __node
    }};

    (Column ( $($prop:ident $(= $val:expr)?),* $(,)? ) { $($child:tt)* }) => {{
        let mut __node = $crate::UiNode::column(vec![]);
        $(
            rsx!(@apply_flex_prop __node $prop $(= $val)?);
        )*
        rsx!(@push_children __node $($child)*);
        __node
    }};

    // ── Divider ──

    (Divider ()) => {{ $crate::UiNode::divider() }};

    (Divider ( $($prop:ident $(= $val:expr)?),* $(,)? )) => {{
        let mut __node = $crate::UiNode::divider();
        $(
            rsx!(@apply_divider_prop __node $prop $(= $val)?);
        )*
        __node
    }};

    // ── Progress ──

    (Progress ( $($prop:ident $(= $val:expr)?),* $(,)? )) => {{
        let mut __node = $crate::UiNode::progress(0.0, 100.0);
        $(
            rsx!(@apply_progress_prop __node $prop $(= $val)?);
        )*
        __node
    }};

    // ── List ──

    (List ( $($prop:ident $(= $val:expr)?),* $(,)? ) { $($child:tt)* }) => {{
        let mut __node = $crate::UiNode::list(vec![]);
        $(
            rsx!(@apply_list_prop __node $prop $(= $val)?);
        )*
        rsx!(@push_children __node $($child)*);
        __node
    }};

    // ── Input ──

    (Input ( $($prop:ident $(= $val:expr)?),* $(,)? )) => {{
        let mut __node = $crate::UiNode::input("");
        $(
            rsx!(@apply_input_prop __node $prop $(= $val)?);
        )*
        __node
    }};

    // ── TabBar ──

    (TabBar ( $($prop:ident $(= $val:expr)?),* $(,)? )) => {{
        let mut __node = $crate::UiNode::tab_bar(vec![]);
        $(
            rsx!(@apply_tabbar_prop __node $prop $(= $val)?);
        )*
        __node
    }};

    // ── SplitPane ──

    (Split ( horizontal ) { $($first:tt)* } { $($second:tt)* }) => {{
        let __first = rsx!(@single_child $($first)*);
        let __second = rsx!(@single_child $($second)*);
        $crate::UiNode::split($crate::Orientation::Horizontal, __first, __second)
    }};
    (Split ( vertical ) { $($first:tt)* } { $($second:tt)* }) => {{
        let __first = rsx!(@single_child $($first)*);
        let __second = rsx!(@single_child $($second)*);
        $crate::UiNode::split($crate::Orientation::Vertical, __first, __second)
    }};

    // ── Canvas ──

    (Canvas ( width = $w:expr, height = $h:expr, id = $id:expr )) => {{
        $crate::UiNode::canvas($w, $h, $id)
    }};

    // ── StatusBar ──

    (StatusBar ( $($prop:ident $(= $val:expr)?),* $(,)? ) { left { $($left:tt)* } right { $($right:tt)* } }) => {{
        let mut __left = $crate::UiNode::row(vec![]);
        rsx!(@push_children __left $($left)*);
        let mut __right = $crate::UiNode::row(vec![]);
        rsx!(@push_children __right $($right)*);
        let __left_items = match __left { $crate::UiNode::Row(f) => f.children, _ => vec![] };
        let __right_items = match __right { $crate::UiNode::Row(f) => f.children, _ => vec![] };
        $crate::UiNode::status_bar(__left_items, __right_items)
    }};

    // ── Show ──

    (Show ( when = $cond:expr ) { $($child:tt)* }) => {{
        let __child = rsx!(@single_child $($child)*);
        $crate::UiNode::show($cond, __child)
    }};

    // ── Single child helper ──

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

    // Input props
    (@apply_input_prop $node:ident value = $val:expr) => {
        if let $crate::UiNode::Input(ref mut i) = $node { i.value = $val.to_string(); }
    };
    (@apply_input_prop $node:ident placeholder = $val:expr) => {
        if let $crate::UiNode::Input(ref mut i) = $node { i.placeholder = $val.to_string(); }
    };
    (@apply_input_prop $node:ident width = $val:expr) => {
        if let $crate::UiNode::Input(ref mut i) = $node { i.width = Some($val); }
    };
    (@apply_input_prop $node:ident focused) => {
        if let $crate::UiNode::Input(ref mut i) = $node { i.focused = true; }
    };
    (@apply_input_prop $node:ident cursor = $val:expr) => {
        if let $crate::UiNode::Input(ref mut i) = $node { i.cursor = $val; }
    };

    // TabBar props
    (@apply_tabbar_prop $node:ident active = $val:expr) => {
        if let $crate::UiNode::TabBar(ref mut t) = $node { t.active = $val; }
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

    // String literal child
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
