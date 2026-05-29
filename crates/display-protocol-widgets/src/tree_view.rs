use display_protocol::{Style, TreeViewNode, TreeItem, UiNode};
use crate::palette;

/// A collapsible tree view (file explorer, nested menus, AST viewer).
///
/// Works on both TUI (indented lines with ▶/▼) and WGPU
/// (file icons, hover highlights).
///
/// ```ignore
/// TreeView::new(vec![
///     TreeBranch::new("src")
///         .icon("📁")
///         .expanded(true)
///         .children(vec![
///             TreeBranch::new("main.rs").icon("🦀"),
///             TreeBranch::new("lib.rs").icon("🦀"),
///         ]),
///     TreeBranch::new("Cargo.toml").icon("📄"),
/// ])
/// .selected(Some(1))
/// .build()
/// ```
#[derive(Debug, Clone)]
pub struct TreeView {
    items: Vec<TreeBranch>,
    selected: Option<usize>,
    indent: u16,
}

/// Convenience wrapper around `TreeItem` with builder API.
#[derive(Debug, Clone)]
pub struct TreeBranch {
    label: String,
    icon: Option<String>,
    children: Vec<TreeBranch>,
    expanded: bool,
}

impl TreeBranch {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            children: Vec::new(),
            expanded: false,
        }
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self { self.icon = Some(icon.into()); self }
    pub fn expanded(mut self, e: bool) -> Self { self.expanded = e; self }
    pub fn children(mut self, items: Vec<TreeBranch>) -> Self { self.children = items; self }

    fn into_tree_item(self) -> TreeItem {
        TreeItem {
            label: self.label,
            icon: self.icon,
            children: self.children.into_iter().map(|c| c.into_tree_item()).collect(),
            expanded: self.expanded,
        }
    }
}

impl TreeView {
    pub fn new(branches: Vec<TreeBranch>) -> Self {
        Self {
            items: branches,
            selected: None,
            indent: 2,
        }
    }

    pub fn selected(mut self, idx: Option<usize>) -> Self { self.selected = idx; self }
    pub fn indent(mut self, i: u16) -> Self { self.indent = i; self }

    pub fn build(self) -> UiNode {
        let items: Vec<TreeItem> = self.items
            .into_iter()
            .map(|b| b.into_tree_item())
            .collect();

        UiNode::TreeView(TreeViewNode {
            items,
            selected: self.selected,
            style: Style::default().fg(palette::LIGHT),
            selected_style: Style::default().fg(palette::WHITE).bg(palette::Color::new(40, 50, 70)),
            indent: self.indent,
        })
    }
}

impl From<TreeView> for UiNode {
    fn from(tv: TreeView) -> Self { tv.build() }
}
