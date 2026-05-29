use display_protocol::{Style, TableNode, UiNode};
use crate::palette;

/// A table widget with headers and rows.
///
/// Works on both TUI and WGPU backends.
///
/// ```ignore
/// Table::new(vec!["Name", "Age", "Role"])
///     .row(vec!["Alice", "30", "Engineer"])
///     .row(vec!["Bob", "25", "Designer"])
///     .bordered(true)
///     .build()
/// ```
#[derive(Debug, Clone)]
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    header_style: Style,
    bordered: bool,
}

impl Table {
    pub fn new(headers: Vec<impl Into<String>>) -> Self {
        Self {
            headers: headers.into_iter().map(|h| h.into()).collect(),
            rows: Vec::new(),
            header_style: Style::default()
                .fg(palette::WHITE)
                .bold(),
            bordered: true,
        }
    }

    /// Add a data row.
    pub fn row(mut self, cells: Vec<impl Into<String>>) -> Self {
        self.rows.push(cells.into_iter().map(|c| c.into()).collect());
        self
    }

    /// Add multiple data rows at once.
    pub fn rows(mut self, rows: Vec<Vec<impl Into<String>>>) -> Self {
        for row in rows {
            self.rows.push(row.into_iter().map(|c| c.into()).collect());
        }
        self
    }

    pub fn header_style(mut self, style: Style) -> Self { self.header_style = style; self }
    pub fn bordered(mut self, v: bool) -> Self { self.bordered = v; self }

    pub fn build(self) -> UiNode {
        UiNode::Table(TableNode {
            headers: self.headers,
            rows: self.rows,
            style: Style::default(),
            header_style: self.header_style,
            border: self.bordered,
        })
    }
}

impl From<Table> for UiNode {
    fn from(t: Table) -> Self { t.build() }
}
