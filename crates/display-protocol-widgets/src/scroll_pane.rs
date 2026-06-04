use display_protocol::{ScrollNode, ScrollPolicy, UiNode};

/// A scrollable container for a child node.
///
/// Works on both TUI and WGPU backends. Supports virtual scrolling
/// for large content.
///
/// ```ignore
/// ScrollPane::new(content)
///     .viewport(80, 24)
///     .build()
///
/// ScrollPane::new(content)
///     .viewport(80, 24)
///     .scroll_offset(0, 100)
///     .content_size(Some(1000), None)
///     .virtual_scroll(true)
///     .build()
/// ```
#[derive(Debug, Clone)]
pub struct ScrollPane {
    child: UiNode,
    scroll_x: u32,
    scroll_y: u32,
    viewport_width: u16,
    viewport_height: u16,
    content_width: Option<u32>,
    content_height: Option<u32>,
    virtual_scroll: bool,
    scroll_policy: ScrollPolicy,
}

impl ScrollPane {
    pub fn new(child: impl Into<UiNode>) -> Self {
        Self {
            child: child.into(),
            scroll_x: 0,
            scroll_y: 0,
            viewport_width: 80,
            viewport_height: 24,
            content_width: None,
            content_height: None,
            virtual_scroll: false,
            scroll_policy: ScrollPolicy::Auto,
        }
    }

    pub fn viewport(mut self, width: u16, height: u16) -> Self {
        self.viewport_width = width;
        self.viewport_height = height;
        self
    }

    pub fn scroll_offset(mut self, x: u32, y: u32) -> Self {
        self.scroll_x = x;
        self.scroll_y = y;
        self
    }

    pub fn scroll_x(mut self, x: u32) -> Self {
        self.scroll_x = x;
        self
    }
    pub fn scroll_y(mut self, y: u32) -> Self {
        self.scroll_y = y;
        self
    }

    pub fn content_size(mut self, width: Option<u32>, height: Option<u32>) -> Self {
        self.content_width = width;
        self.content_height = height;
        self
    }

    pub fn virtual_scroll(mut self, v: bool) -> Self {
        self.virtual_scroll = v;
        self
    }

    pub fn scroll_policy(mut self, policy: ScrollPolicy) -> Self {
        self.scroll_policy = policy;
        self
    }

    pub fn build(self) -> UiNode {
        UiNode::ScrollView(ScrollNode {
            child: Box::new(self.child),
            scroll_x: self.scroll_x,
            scroll_y: self.scroll_y,
            viewport_width: self.viewport_width,
            viewport_height: self.viewport_height,
            content_width: self.content_width,
            content_height: self.content_height,
            virtual_scroll: self.virtual_scroll,
            scroll_policy: self.scroll_policy,
        })
    }
}

impl From<ScrollPane> for UiNode {
    fn from(s: ScrollPane) -> Self {
        s.build()
    }
}
