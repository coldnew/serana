use crate::types::{Color, Style, StyledLine, Selection};

/// A declarative UI node — the building block of the component tree.
///
/// Like React's virtual DOM or storm-rs's RsxNode, UiNode represents
/// a declarative description of UI that can be rendered to any backend.
///
/// # Backend rendering rules
/// - **TUI**: All nodes render to character cells. `Canvas` nodes leave their
///   rect blank (reserved space, no content). `Overlay` nodes paint on top.
/// - **WGPU**: All nodes render fully, including Canvas pixel content.
#[derive(Debug, Clone)]
pub enum UiNode {
    /// A text leaf node.
    Text(TextNode),
    /// A container with layout properties.
    Box(BoxNode),
    /// A horizontal layout container.
    Row(FlexNode),
    /// A vertical layout container.
    Column(FlexNode),
    /// A styled span of inline text.
    Span(SpanNode),
    /// A list of items.
    List(ListNode),
    /// A single list item.
    ListItem(Box<UiNode>),
    /// A horizontal rule / divider.
    Divider(DividerNode),
    /// A progress bar.
    ProgressBar(ProgressNode),
    /// A table with rows and columns.
    Table(TableNode),
    /// A scrollable view wrapping a child.
    ScrollView(ScrollNode),
    /// Conditional rendering — renders child if condition is true.
    Show { when: bool, child: Box<UiNode> },
    /// Iteration — renders children from an iterator.
    For { children: Vec<UiNode> },

    // ── Editor / IDE / Agent widgets ──

    /// Single-line text input with cursor.
    Input(InputNode),
    /// Multi-line text editor with cursor, selection, scrolling.
    TextArea(TextAreaNode),
    /// Tab bar (for editor tabs, browser tabs, etc).
    TabBar(TabBarNode),
    /// Collapsible tree view (file explorer, AST, etc).
    TreeView(TreeViewNode),
    /// Split pane with draggable divider (horizontal or vertical).
    SplitPane(SplitPaneNode),
    /// Bottom/top status bar with left/right sections.
    StatusBar(StatusBarNode),

    // ── WGPU-only widgets (TUI leaves rect blank) ──

    /// Canvas for custom pixel-level drawing.
    /// TUI: reserves the rect but leaves it blank.
    /// WGPU: renders the frame data (images, graphs, rich content).
    Canvas(CanvasNode),
    /// Absolutely-positioned overlay (popup, tooltip, dropdown menu).
    /// Stacked by `z_index`. TUI paints on top of existing content.
    Overlay(OverlayNode),

    /// Empty / null node.
    None,
}

/// Text leaf node.
#[derive(Debug, Clone)]
pub struct TextNode {
    pub content: String,
    pub style: Style,
    pub wrap: Wrap,
}

/// Box container node with optional border and padding.
#[derive(Debug, Clone)]
pub struct BoxNode {
    pub children: Vec<UiNode>,
    pub style: Style,
    pub padding: Padding,
    pub border: Border,
    pub title: Option<String>,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub min_width: Option<u16>,
    pub min_height: Option<u16>,
    pub max_width: Option<u16>,
    pub max_height: Option<u16>,
}

/// Flex container (Row or Column).
#[derive(Debug, Clone)]
pub struct FlexNode {
    pub children: Vec<UiNode>,
    pub style: Style,
    pub gap: u16,
    pub align: Align,
    pub justify: Justify,
    pub padding: Padding,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub flex_grow: f32,
    pub flex_shrink: f32,
}

/// Inline span node.
#[derive(Debug, Clone)]
pub struct SpanNode {
    pub content: String,
    pub style: Style,
}

/// List container.
#[derive(Debug, Clone)]
pub struct ListNode {
    pub items: Vec<UiNode>,
    pub marker: ListMarker,
    pub style: Style,
}

/// Divider / horizontal rule.
#[derive(Debug, Clone)]
pub struct DividerNode {
    pub style: Style,
    pub char: Option<char>,
}

/// Progress bar.
#[derive(Debug, Clone)]
pub struct ProgressNode {
    pub value: f32,
    pub max: f32,
    pub width: u16,
    pub filled_style: Style,
    pub empty_style: Style,
    pub show_percent: bool,
}

/// Table.
#[derive(Debug, Clone)]
pub struct TableNode {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub style: Style,
    pub header_style: Style,
    pub border: bool,
}

/// Scrollable view.
#[derive(Debug, Clone)]
pub struct ScrollNode {
    pub child: Box<UiNode>,
    pub scroll_top: u16,
    pub height: u16,
}

/// Single-line text input field.
///
/// TUI: renders the value with a cursor indicator (█ or underline).
/// WGPU: renders a native-feel input with selection highlight.
#[derive(Debug, Clone)]
pub struct InputNode {
    pub value: String,
    pub placeholder: String,
    pub cursor: u16,
    pub style: Style,
    pub cursor_style: Style,
    pub width: Option<u16>,
    pub focused: bool,
}

/// Multi-line text area / editor buffer.
///
/// TUI: renders lines with a cursor block and scroll offset.
/// WGPU: renders with selection highlight, gutter, minimap, etc.
#[derive(Debug, Clone)]
pub struct TextAreaNode {
    pub lines: Vec<StyledLine>,
    pub cursor_line: u16,
    pub cursor_col: u16,
    pub selection: Option<Selection>,
    pub scroll_top: u16,
    pub scroll_left: u16,
    pub height: u16,
    pub style: Style,
    pub cursor_style: Style,
    pub selection_style: Style,
    pub gutter: bool,
    pub focused: bool,
}

/// Tab bar (editor tabs, browser tabs, terminal tabs).
///
/// TUI: renders as a row of `[ title ]` segments with active underline.
/// WGPU: can render close buttons, icons, drag handles.
#[derive(Debug, Clone)]
pub struct TabBarNode {
    pub items: Vec<TabItem>,
    pub active: usize,
    pub style: Style,
    pub active_style: Style,
}

/// A single tab item.
#[derive(Debug, Clone)]
pub struct TabItem {
    pub title: String,
    pub icon: Option<String>,
    pub modified: bool,
}

/// Collapsible tree view (file explorer, AST viewer, nested menus).
///
/// TUI: renders as indented lines with expand/collapse indicators (▶/▼).
/// WGPU: can render file icons, hover highlights, drag-and-drop.
#[derive(Debug, Clone)]
pub struct TreeViewNode {
    pub items: Vec<TreeItem>,
    pub selected: Option<usize>,
    pub style: Style,
    pub selected_style: Style,
    pub indent: u16,
}

/// A single tree item with optional children.
#[derive(Debug, Clone)]
pub struct TreeItem {
    pub label: String,
    pub icon: Option<String>,
    pub children: Vec<TreeItem>,
    pub expanded: bool,
}

/// Split pane with horizontal or vertical divider.
///
/// TUI: renders both children with a `│` or `─` divider line.
/// WGPU: adds a draggable resize handle.
#[derive(Debug, Clone)]
pub struct SplitPaneNode {
    pub orientation: Orientation,
    pub ratio: f32,
    pub first: Box<UiNode>,
    pub second: Box<UiNode>,
    pub divider_style: Style,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

/// Canvas for custom pixel-level drawing (WGPU-only in spirit).
///
/// TUI backend: reserves the given width×height rect but leaves cells blank.
/// WGPU backend: reads `frame_data` and renders pixels, images, graphs, etc.
#[derive(Debug, Clone)]
pub struct CanvasNode {
    pub width: u16,
    pub height: u16,
    pub frame_id: String,
    pub bg: Color,
}

/// Absolutely-positioned overlay for popups, tooltips, context menus.
///
/// Positioned relative to the viewport. Stacked by `z_index` (higher = on top).
/// TUI: paints over existing cell content.
/// WGPU: renders with full alpha compositing.
#[derive(Debug, Clone)]
pub struct OverlayNode {
    pub child: Box<UiNode>,
    pub x: u16,
    pub y: u16,
    pub z_index: i32,
    pub style: Style,
}

/// Status bar with left and right sections.
///
/// TUI: renders left-aligned and right-aligned sections on one line.
/// WGPU: same, with richer styling.
#[derive(Debug, Clone)]
pub struct StatusBarNode {
    pub left: Vec<UiNode>,
    pub right: Vec<UiNode>,
    pub style: Style,
}

// ── Layout types ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Wrap {
    #[default]
    Wrap,
    NoWrap,
    Truncate,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Padding {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

impl Padding {
    pub const ZERO: Self = Self { top: 0, right: 0, bottom: 0, left: 0 };
    pub const fn all(v: u16) -> Self {
        Self { top: v, right: v, bottom: v, left: v }
    }
    pub const fn horizontal(v: u16) -> Self {
        Self { top: 0, right: v, bottom: 0, left: v }
    }
    pub const fn vertical(v: u16) -> Self {
        Self { top: v, right: 0, bottom: v, left: 0 }
    }
    pub const fn new(top: u16, right: u16, bottom: u16, left: u16) -> Self {
        Self { top, right, bottom, left }
    }
    pub fn horizontal_total(&self) -> u16 { self.left + self.right }
    pub fn vertical_total(&self) -> u16 { self.top + self.bottom }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Border {
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
    pub style: Option<Style>,
}

impl Border {
    pub const NONE: Self = Self { top: false, right: false, bottom: false, left: false, style: None };
    pub const fn all(style: Option<Style>) -> Self {
        Self { top: true, right: true, bottom: true, left: true, style }
    }
    pub fn has_any(&self) -> bool {
        self.top || self.right || self.bottom || self.left
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Justify {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListMarker {
    #[default]
    None,
    Bullet,
    Number,
    Dash,
}

// ── Constructor helpers (builder API) ──

impl UiNode {
    pub fn text(content: impl Into<String>) -> Self {
        UiNode::Text(TextNode {
            content: content.into(),
            style: Style::default(),
            wrap: Wrap::default(),
        })
    }

    pub fn span(content: impl Into<String>, style: Style) -> Self {
        UiNode::Span(SpanNode { content: content.into(), style })
    }

    pub fn boxed(children: Vec<UiNode>) -> Self {
        UiNode::Box(BoxNode {
            children,
            style: Style::default(),
            padding: Padding::ZERO,
            border: Border::NONE,
            title: None,
            width: None,
            height: None,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
        })
    }

    pub fn row(children: Vec<UiNode>) -> Self {
        UiNode::Row(FlexNode {
            children,
            style: Style::default(),
            gap: 0,
            align: Align::default(),
            justify: Justify::default(),
            padding: Padding::ZERO,
            width: None,
            height: None,
            flex_grow: 0.0,
            flex_shrink: 1.0,
        })
    }

    pub fn column(children: Vec<UiNode>) -> Self {
        UiNode::Column(FlexNode {
            children,
            style: Style::default(),
            gap: 0,
            align: Align::default(),
            justify: Justify::default(),
            padding: Padding::ZERO,
            width: None,
            height: None,
            flex_grow: 0.0,
            flex_shrink: 1.0,
        })
    }

    pub fn divider() -> Self {
        UiNode::Divider(DividerNode { style: Style::default(), char: None })
    }

    pub fn progress(value: f32, max: f32) -> Self {
        UiNode::ProgressBar(ProgressNode {
            value,
            max,
            width: 20,
            filled_style: Style::default(),
            empty_style: Style::default(),
            show_percent: true,
        })
    }

    pub fn list(items: Vec<UiNode>) -> Self {
        UiNode::List(ListNode {
            items,
            marker: ListMarker::Bullet,
            style: Style::default(),
        })
    }

    pub fn show(when: bool, child: impl Into<UiNode>) -> Self {
        UiNode::Show { when, child: Box::new(child.into()) }
    }

    pub fn for_each<T>(items: impl IntoIterator<Item = T>, mut f: impl FnMut(T) -> UiNode) -> Self {
        UiNode::For { children: items.into_iter().map(|item| f(item)).collect() }
    }

    pub fn input(value: impl Into<String>) -> Self {
        UiNode::Input(InputNode {
            value: value.into(),
            placeholder: String::new(),
            cursor: 0,
            style: Style::default(),
            cursor_style: Style::default(),
            width: None,
            focused: false,
        })
    }

    pub fn textarea(lines: Vec<StyledLine>) -> Self {
        UiNode::TextArea(TextAreaNode {
            lines,
            cursor_line: 0,
            cursor_col: 0,
            selection: None,
            scroll_top: 0,
            scroll_left: 0,
            height: 10,
            style: Style::default(),
            cursor_style: Style::default(),
            selection_style: Style::default().bg(Color::new(50, 100, 180)),
            gutter: false,
            focused: false,
        })
    }

    /// Convenience: build a TextAreaNode from plain strings (no styling).
    pub fn textarea_plain(lines: Vec<String>) -> Self {
        Self::textarea(lines.into_iter().map(StyledLine::plain).collect())
    }


    pub fn tab_bar(items: Vec<TabItem>) -> Self {
        UiNode::TabBar(TabBarNode {
            items,
            active: 0,
            style: Style::default(),
            active_style: Style::default().underline(),
        })
    }

    pub fn tree_view(items: Vec<TreeItem>) -> Self {
        UiNode::TreeView(TreeViewNode {
            items,
            selected: None,
            style: Style::default(),
            selected_style: Style::default().reverse(),
            indent: 2,
        })
    }

    pub fn split(orientation: Orientation, first: UiNode, second: UiNode) -> Self {
        UiNode::SplitPane(SplitPaneNode {
            orientation,
            ratio: 0.5,
            first: Box::new(first),
            second: Box::new(second),
            divider_style: Style::default(),
        })
    }

    pub fn canvas(width: u16, height: u16, frame_id: impl Into<String>) -> Self {
        UiNode::Canvas(CanvasNode {
            width,
            height,
            frame_id: frame_id.into(),
            bg: Color::BLACK,
        })
    }

    pub fn overlay(child: UiNode, x: u16, y: u16) -> Self {
        UiNode::Overlay(OverlayNode {
            child: Box::new(child),
            x,
            y,
            z_index: 0,
            style: Style::default(),
        })
    }

    pub fn status_bar(left: Vec<UiNode>, right: Vec<UiNode>) -> Self {
        UiNode::StatusBar(StatusBarNode {
            left,
            right,
            style: Style::default(),
        })
    }

    /// Check if this node is None/empty.
    pub fn is_none(&self) -> bool {
        matches!(self, UiNode::None)
    }

    /// Returns true if this node type is WGPU-only (TUI should leave rect blank).
    pub fn is_wgpu_only(&self) -> bool {
        matches!(self, UiNode::Canvas(_))
    }
}

// ── Delegating builder methods on UiNode ──
// These allow chaining like: UiNode::text("x").bold().color(c)

impl UiNode {
    pub fn bold(self) -> Self {
        match self {
            UiNode::Text(t) => UiNode::Text(t.bold()),
            other => other,
        }
    }
    pub fn italic(self) -> Self {
        match self {
            UiNode::Text(t) => UiNode::Text(t.italic()),
            other => other,
        }
    }
    pub fn underline(self) -> Self {
        match self {
            UiNode::Text(t) => UiNode::Text(t.underline()),
            other => other,
        }
    }
    pub fn dim(self) -> Self {
        match self {
            UiNode::Text(t) => UiNode::Text(t.dim()),
            other => other,
        }
    }
    pub fn reverse(self) -> Self {
        match self {
            UiNode::Text(mut t) => { t.style = t.style.reverse(); UiNode::Text(t) }
            other => other,
        }
    }
    pub fn strikethrough(self) -> Self {
        match self {
            UiNode::Text(mut t) => { t.style = t.style.strikethrough(); UiNode::Text(t) }
            other => other,
        }
    }
    pub fn color(self, c: Color) -> Self {
        match self {
            UiNode::Text(t) => UiNode::Text(t.color(c)),
            other => other,
        }
    }
    pub fn bg(self, c: Color) -> Self {
        match self {
            UiNode::Text(t) => UiNode::Text(t.bg(c)),
            other => other,
        }
    }
    pub fn wrap(self, w: Wrap) -> Self {
        match self {
            UiNode::Text(t) => UiNode::Text(t.wrap(w)),
            other => other,
        }
    }
    pub fn gap(self, g: u16) -> Self {
        match self {
            UiNode::Row(f) => UiNode::Row(f.gap(g)),
            UiNode::Column(f) => UiNode::Column(f.gap(g)),
            other => other,
        }
    }
    pub fn align(self, a: Align) -> Self {
        match self {
            UiNode::Row(f) => UiNode::Row(f.align(a)),
            UiNode::Column(f) => UiNode::Column(f.align(a)),
            other => other,
        }
    }
    pub fn justify(self, j: Justify) -> Self {
        match self {
            UiNode::Row(f) => UiNode::Row(f.justify(j)),
            UiNode::Column(f) => UiNode::Column(f.justify(j)),
            other => other,
        }
    }
    pub fn padding(self, p: Padding) -> Self {
        match self {
            UiNode::Box(b) => UiNode::Box(b.padding(p)),
            UiNode::Row(f) => UiNode::Row(f.padding(p)),
            UiNode::Column(f) => UiNode::Column(f.padding(p)),
            other => other,
        }
    }
    pub fn border(self, b: Border) -> Self {
        match self {
            UiNode::Box(bx) => UiNode::Box(bx.border(b)),
            other => other,
        }
    }
    pub fn title(self, t: impl Into<String>) -> Self {
        match self {
            UiNode::Box(b) => UiNode::Box(b.title(t)),
            other => other,
        }
    }
    pub fn width(self, w: u16) -> Self {
        match self {
            UiNode::Box(b) => UiNode::Box(b.width(w)),
            UiNode::Row(f) => UiNode::Row(f.width(w)),
            UiNode::Column(f) => UiNode::Column(f.width(w)),
            UiNode::ProgressBar(p) => UiNode::ProgressBar(p.width(w)),
            other => other,
        }
    }
    pub fn height(self, h: u16) -> Self {
        match self {
            UiNode::Box(b) => UiNode::Box(b.height(h)),
            UiNode::Row(f) => UiNode::Row(f.height(h)),
            UiNode::Column(f) => UiNode::Column(f.height(h)),
            other => other,
        }
    }
    pub fn flex_grow(self, g: f32) -> Self {
        match self {
            UiNode::Row(f) => UiNode::Row(f.flex_grow(g)),
            UiNode::Column(f) => UiNode::Column(f.flex_grow(g)),
            other => other,
        }
    }
    pub fn flex_shrink(self, s: f32) -> Self {
        match self {
            UiNode::Row(f) => UiNode::Row(f.flex_shrink(s)),
            UiNode::Column(f) => UiNode::Column(f.flex_shrink(s)),
            other => other,
        }
    }
    pub fn marker(self, m: ListMarker) -> Self {
        match self {
            UiNode::List(l) => UiNode::List(l.marker(m)),
            other => other,
        }
    }
    pub fn filled_style(self, s: Style) -> Self {
        match self {
            UiNode::ProgressBar(p) => UiNode::ProgressBar(p.filled_style(s)),
            other => other,
        }
    }
    pub fn empty_style(self, s: Style) -> Self {
        match self {
            UiNode::ProgressBar(p) => UiNode::ProgressBar(p.empty_style(s)),
            other => other,
        }
    }
    pub fn show_percent(self, show: bool) -> Self {
        match self {
            UiNode::ProgressBar(p) => UiNode::ProgressBar(p.show_percent(show)),
            other => other,
        }
    }
    pub fn placeholder(self, p: impl Into<String>) -> Self {
        match self {
            UiNode::Input(i) => UiNode::Input(i.placeholder(p)),
            other => other,
        }
    }
    pub fn cursor_pos(self, pos: u16) -> Self {
        match self {
            UiNode::Input(i) => UiNode::Input(i.cursor_pos(pos)),
            other => other,
        }
    }
    pub fn focused(self, f: bool) -> Self {
        match self {
            UiNode::Input(i) => UiNode::Input(i.focused(f)),
            UiNode::TextArea(t) => UiNode::TextArea(t.focused(f)),
            other => other,
        }
    }
    pub fn gutter(self, show: bool) -> Self {
        match self {
            UiNode::TextArea(t) => UiNode::TextArea(t.gutter(show)),
            other => other,
        }
    }
    pub fn selection(self, sel: Selection) -> Self {
        match self {
            UiNode::TextArea(t) => UiNode::TextArea(t.selection(sel)),
            other => other,
        }
    }
    pub fn clear_selection(self) -> Self {
        match self {
            UiNode::TextArea(t) => UiNode::TextArea(t.clear_selection()),
            other => other,
        }
    }
    pub fn active_tab(self, idx: usize) -> Self {
        match self {
            UiNode::TabBar(t) => UiNode::TabBar(t.active(idx)),
            other => other,
        }
    }
    pub fn selected(self, idx: Option<usize>) -> Self {
        match self {
            UiNode::TreeView(t) => UiNode::TreeView(t.selected(idx)),
            other => other,
        }
    }
    pub fn ratio(self, r: f32) -> Self {
        match self {
            UiNode::SplitPane(s) => UiNode::SplitPane(s.ratio(r)),
            other => other,
        }
    }
    pub fn z_index(self, z: i32) -> Self {
        match self {
            UiNode::Overlay(o) => UiNode::Overlay(o.z_index(z)),
            other => other,
        }
    }
}

impl std::fmt::Display for UiNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UiNode::Text(t) => write!(f, "{}", t.content),
            UiNode::Span(s) => write!(f, "{}", s.content),
            UiNode::Input(i) => write!(f, "{}", i.value),
            UiNode::None => Ok(()),
            other => write!(f, "<{:?}>", std::mem::discriminant(other)),
        }
    }
}

// ── Builder pattern methods on node types ──

impl TextNode {
    pub fn style(mut self, style: Style) -> Self { self.style = style; self }
    pub fn bold(mut self) -> Self { self.style = self.style.bold(); self }
    pub fn italic(mut self) -> Self { self.style = self.style.italic(); self }
    pub fn underline(mut self) -> Self { self.style = self.style.underline(); self }
    pub fn dim(mut self) -> Self { self.style = self.style.dim(); self }
    pub fn color(mut self, color: Color) -> Self { self.style.fg = Some(color); self }
    pub fn bg(mut self, color: Color) -> Self { self.style.bg = Some(color); self }
    pub fn wrap(mut self, wrap: Wrap) -> Self { self.wrap = wrap; self }
}

impl BoxNode {
    pub fn style(mut self, style: Style) -> Self { self.style = style; self }
    pub fn padding(mut self, padding: Padding) -> Self { self.padding = padding; self }
    pub fn border(mut self, border: Border) -> Self { self.border = border; self }
    pub fn title(mut self, title: impl Into<String>) -> Self { self.title = Some(title.into()); self }
    pub fn width(mut self, w: u16) -> Self { self.width = Some(w); self }
    pub fn height(mut self, h: u16) -> Self { self.height = Some(h); self }
    pub fn min_width(mut self, w: u16) -> Self { self.min_width = Some(w); self }
    pub fn min_height(mut self, h: u16) -> Self { self.min_height = Some(h); self }
    pub fn max_width(mut self, w: u16) -> Self { self.max_width = Some(w); self }
    pub fn max_height(mut self, h: u16) -> Self { self.max_height = Some(h); self }
}

impl FlexNode {
    pub fn style(mut self, style: Style) -> Self { self.style = style; self }
    pub fn gap(mut self, gap: u16) -> Self { self.gap = gap; self }
    pub fn align(mut self, align: Align) -> Self { self.align = align; self }
    pub fn justify(mut self, justify: Justify) -> Self { self.justify = justify; self }
    pub fn padding(mut self, padding: Padding) -> Self { self.padding = padding; self }
    pub fn width(mut self, w: u16) -> Self { self.width = Some(w); self }
    pub fn height(mut self, h: u16) -> Self { self.height = Some(h); self }
    pub fn flex_grow(mut self, grow: f32) -> Self { self.flex_grow = grow; self }
    pub fn flex_shrink(mut self, shrink: f32) -> Self { self.flex_shrink = shrink; self }
}

impl ProgressNode {
    pub fn width(mut self, w: u16) -> Self { self.width = w; self }
    pub fn filled_style(mut self, style: Style) -> Self { self.filled_style = style; self }
    pub fn empty_style(mut self, style: Style) -> Self { self.empty_style = style; self }
    pub fn show_percent(mut self, show: bool) -> Self { self.show_percent = show; self }
}

impl ListNode {
    pub fn marker(mut self, marker: ListMarker) -> Self { self.marker = marker; self }
    pub fn style(mut self, style: Style) -> Self { self.style = style; self }
}

impl DividerNode {
    pub fn char(mut self, c: char) -> Self { self.char = Some(c); self }
    pub fn style(mut self, style: Style) -> Self { self.style = style; self }
}

impl TableNode {
    pub fn border(mut self, border: bool) -> Self { self.border = border; self }
    pub fn style(mut self, style: Style) -> Self { self.style = style; self }
    pub fn header_style(mut self, style: Style) -> Self { self.header_style = style; self }
}

impl InputNode {
    pub fn placeholder(mut self, p: impl Into<String>) -> Self { self.placeholder = p.into(); self }
    pub fn cursor_pos(mut self, pos: u16) -> Self { self.cursor = pos; self }
    pub fn style(mut self, style: Style) -> Self { self.style = style; self }
    pub fn width(mut self, w: u16) -> Self { self.width = Some(w); self }
    pub fn focused(mut self, f: bool) -> Self { self.focused = f; self }
}

impl TextAreaNode {
    pub fn cursor(mut self, line: u16, col: u16) -> Self { self.cursor_line = line; self.cursor_col = col; self }
    pub fn selection(mut self, sel: Selection) -> Self { self.selection = Some(sel); self }
    pub fn clear_selection(mut self) -> Self { self.selection = None; self }
    pub fn scroll(mut self, top: u16, left: u16) -> Self { self.scroll_top = top; self.scroll_left = left; self }
    pub fn height(mut self, h: u16) -> Self { self.height = h; self }
    pub fn style(mut self, style: Style) -> Self { self.style = style; self }
    pub fn selection_style(mut self, style: Style) -> Self { self.selection_style = style; self }
    pub fn gutter(mut self, show: bool) -> Self { self.gutter = show; self }
    pub fn focused(mut self, f: bool) -> Self { self.focused = f; self }
}

impl TabBarNode {
    pub fn active(mut self, idx: usize) -> Self { self.active = idx; self }
    pub fn style(mut self, style: Style) -> Self { self.style = style; self }
    pub fn active_style(mut self, style: Style) -> Self { self.active_style = style; self }
}

impl TabItem {
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), icon: None, modified: false }
    }
    pub fn icon(mut self, icon: impl Into<String>) -> Self { self.icon = Some(icon.into()); self }
    pub fn modified(mut self, m: bool) -> Self { self.modified = m; self }
}

impl TreeViewNode {
    pub fn selected(mut self, idx: Option<usize>) -> Self { self.selected = idx; self }
    pub fn style(mut self, style: Style) -> Self { self.style = style; self }
    pub fn selected_style(mut self, style: Style) -> Self { self.selected_style = style; self }
    pub fn indent(mut self, indent: u16) -> Self { self.indent = indent; self }
}

impl TreeItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), icon: None, children: Vec::new(), expanded: false }
    }
    pub fn children(mut self, children: Vec<TreeItem>) -> Self { self.children = children; self }
    pub fn expanded(mut self, e: bool) -> Self { self.expanded = e; self }
    pub fn icon(mut self, icon: impl Into<String>) -> Self { self.icon = Some(icon.into()); self }
    /// Recursively count visible items (expanded children only).
    pub fn visible_count(&self) -> usize {
        1 + if self.expanded {
            self.children.iter().map(|c| c.visible_count()).sum::<usize>()
        } else { 0 }
    }
}

impl SplitPaneNode {
    pub fn ratio(mut self, r: f32) -> Self { self.ratio = r.clamp(0.0, 1.0); self }
    pub fn divider_style(mut self, style: Style) -> Self { self.divider_style = style; self }
}

impl CanvasNode {
    pub fn bg(mut self, bg: Color) -> Self { self.bg = bg; self }
}

impl OverlayNode {
    pub fn z_index(mut self, z: i32) -> Self { self.z_index = z; self }
    pub fn style(mut self, style: Style) -> Self { self.style = style; self }
}

// ── Into<UiNode> conversions ──

impl From<&str> for UiNode {
    fn from(s: &str) -> Self { UiNode::text(s) }
}

impl From<String> for UiNode {
    fn from(s: String) -> Self { UiNode::text(s) }
}

impl From<TextNode> for UiNode {
    fn from(t: TextNode) -> Self { UiNode::Text(t) }
}

impl From<BoxNode> for UiNode {
    fn from(b: BoxNode) -> Self { UiNode::Box(b) }
}

impl From<FlexNode> for UiNode {
    fn from(f: FlexNode) -> Self { UiNode::Row(f) }
}

impl From<InputNode> for UiNode {
    fn from(i: InputNode) -> Self { UiNode::Input(i) }
}

impl From<TextAreaNode> for UiNode {
    fn from(t: TextAreaNode) -> Self { UiNode::TextArea(t) }
}

impl From<TabBarNode> for UiNode {
    fn from(t: TabBarNode) -> Self { UiNode::TabBar(t) }
}

impl From<TreeViewNode> for UiNode {
    fn from(t: TreeViewNode) -> Self { UiNode::TreeView(t) }
}

impl From<SplitPaneNode> for UiNode {
    fn from(s: SplitPaneNode) -> Self { UiNode::SplitPane(s) }
}

impl From<CanvasNode> for UiNode {
    fn from(c: CanvasNode) -> Self { UiNode::Canvas(c) }
}

impl From<OverlayNode> for UiNode {
    fn from(o: OverlayNode) -> Self { UiNode::Overlay(o) }
}

impl From<StatusBarNode> for UiNode {
    fn from(s: StatusBarNode) -> Self { UiNode::StatusBar(s) }
}

impl Default for UiNode {
    fn default() -> Self { UiNode::None }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_node() {
        let node = UiNode::text("hello");
        match &node {
            UiNode::Text(t) => assert_eq!(t.content, "hello"),
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn test_box_with_children() {
        let node = UiNode::boxed(vec![UiNode::text("a"), UiNode::text("b")]);
        match &node {
            UiNode::Box(b) => assert_eq!(b.children.len(), 2),
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn test_builder_chain() {
        let node = UiNode::text("styled")
            .bold()
            .color(Color::new(255, 0, 0))
            .bg(Color::new(0, 0, 0));
        match &node {
            UiNode::Text(t) => {
                assert!(t.style.bold);
                assert_eq!(t.style.fg, Some(Color::new(255, 0, 0)));
                assert_eq!(t.style.bg, Some(Color::new(0, 0, 0)));
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn test_show_conditional() {
        let visible = UiNode::show(true, UiNode::text("shown"));
        let hidden = UiNode::show(false, UiNode::text("hidden"));

        match &visible {
            UiNode::Show { when, .. } => assert!(*when),
            _ => panic!("expected Show"),
        }
        match &hidden {
            UiNode::Show { when, .. } => assert!(!(*when)),
            _ => panic!("expected Show"),
        }
    }

    #[test]
    fn test_for_each() {
        let items = vec!["a", "b", "c"];
        let node = UiNode::for_each(items, |s| UiNode::text(s));
        match &node {
            UiNode::For { children } => assert_eq!(children.len(), 3),
            _ => panic!("expected For"),
        }
    }

    #[test]
    fn test_row_column() {
        let row = UiNode::row(vec![UiNode::text("1"), UiNode::text("2")])
            .gap(1)
            .align(Align::Center);
        match &row {
            UiNode::Row(f) => {
                assert_eq!(f.gap, 1);
                assert_eq!(f.align, Align::Center);
            }
            _ => panic!("expected Row"),
        }
    }

    #[test]
    fn test_progress() {
        let p = UiNode::progress(75.0, 100.0).width(30);
        match &p {
            UiNode::ProgressBar(pb) => {
                assert_eq!(pb.value, 75.0);
                assert_eq!(pb.width, 30);
            }
            _ => panic!("expected ProgressBar"),
        }
    }

    #[test]
    fn test_padding() {
        let p = Padding::new(1, 2, 3, 4);
        assert_eq!(p.horizontal_total(), 6);
        assert_eq!(p.vertical_total(), 4);
    }

    #[test]
    fn test_from_str() {
        let node: UiNode = "hello".into();
        match &node {
            UiNode::Text(t) => assert_eq!(t.content, "hello"),
            _ => panic!("expected Text"),
        }
    }
}
