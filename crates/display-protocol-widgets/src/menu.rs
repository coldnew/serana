use display_protocol::{MenuAnchor, MenuItem, MenuNode};


/// A dropdown or context menu.
///
/// The menu itself is not a `UiNode` — it's used with `DisplayCmd::ShowPopup`
/// to instruct the frontend to render it as an overlay. The builder produces
/// a `MenuNode` directly.
///
/// ```ignore
/// let menu = Menu::new()
///     .item(MenuItem::new("Copy").shortcut("Ctrl+C"))
///     .item(MenuItem::new("Paste").shortcut("Ctrl+V"))
///     .separator()
///     .item(MenuItem::new("Delete").icon("🗑"))
///     .build();
///
/// // With the display protocol:
/// DisplayCmd::ShowPopup { items: menu.items, x: 10, y: 5 }
/// ```
#[derive(Debug, Clone)]
pub struct Menu {
    items: Vec<MenuItem>,
    anchor: MenuAnchor,
}

impl Menu {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            anchor: MenuAnchor::AtCursor,
        }
    }

    /// Add a menu item.
    pub fn item(mut self, item: MenuItem) -> Self { self.items.push(item); self }

    /// Add a separator line (disabled empty item).
    pub fn separator(mut self) -> Self {
        self.items.push(MenuNode::separator());
        self
    }

    pub fn anchor(mut self, anchor: MenuAnchor) -> Self { self.anchor = anchor; self }

    /// Position the menu below a specific point.
    pub fn at(mut self, x: u16, y: u16) -> Self {
        self.anchor = MenuAnchor::BelowPoint { x, y };
        self
    }

    /// Center the menu in the viewport.
    pub fn centered(mut self) -> Self {
        self.anchor = MenuAnchor::Centered;
        self
    }

    pub fn build(self) -> MenuNode {
        MenuNode {
            items: self.items,
            selected: 0,
            anchor: self.anchor,
        }
    }
}

impl Default for Menu {
    fn default() -> Self { Self::new() }
}

/// Convenience: build a simple context menu from label strings.
pub fn context_menu(labels: Vec<&str>) -> MenuNode {
    let items = labels.into_iter().map(MenuItem::new).collect();
    MenuNode::new(items)
}
