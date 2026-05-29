use display_protocol::{Style, TabBarNode, TabItem, UiNode};
use crate::palette;

/// A tab bar with selectable tabs.
///
/// Works on both TUI (renders `[ title ]` segments) and WGPU
/// (can render close buttons, icons, drag handles).
///
/// ```ignore
/// Tabs::new(vec!["Files", "Search", "Git"])
///     .active(1)
///     .build()
///
/// Tabs::new(vec!["main.rs", "lib.rs"])
///     .active(0)
///     .modified(1, true)
///     .build()
/// ```
#[derive(Debug, Clone)]
pub struct Tabs {
    items: Vec<(String, bool)>, // (title, modified)
    active: usize,
}

impl Tabs {
    pub fn new(titles: Vec<impl Into<String>>) -> Self {
        Self {
            items: titles.into_iter().map(|t| (t.into(), false)).collect(),
            active: 0,
        }
    }

    pub fn active(mut self, idx: usize) -> Self { self.active = idx; self }

    /// Mark a tab as modified (shows a dot indicator).
    pub fn modified(mut self, idx: usize, m: bool) -> Self {
        if idx < self.items.len() {
            self.items[idx].1 = m;
        }
        self
    }

    pub fn build(self) -> UiNode {
        let tab_items: Vec<TabItem> = self.items
            .into_iter()
            .map(|(title, modified)| TabItem {
                title,
                icon: None,
                modified,
            })
            .collect();

        UiNode::TabBar(TabBarNode {
            items: tab_items,
            active: self.active,
            style: Style::default(),
            active_style: Style::default()
                .fg(palette::PRIMARY)
                .underline(),
        })
    }
}

impl From<Tabs> for UiNode {
    fn from(t: Tabs) -> Self { t.build() }
}
