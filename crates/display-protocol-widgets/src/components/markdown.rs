use crate::palette;
use display_protocol::UiNode;

#[derive(Debug, Clone)]
pub struct Markdown {
    source: String,
}

impl Markdown {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
        }
    }

    pub fn build(self) -> UiNode {
        UiNode::column(
            self.source
                .lines()
                .map(markdown_line)
                .collect::<Vec<UiNode>>(),
        )
    }
}

impl From<Markdown> for UiNode {
    fn from(markdown: Markdown) -> Self {
        markdown.build()
    }
}

fn markdown_line(line: &str) -> UiNode {
    if let Some(text) = line.strip_prefix("# ") {
        return UiNode::text(text).bold().color(palette::PRIMARY);
    }
    if let Some(text) = line.strip_prefix("## ") {
        return UiNode::text(text).bold();
    }
    if let Some(text) = line.strip_prefix("> ") {
        return UiNode::text(text).color(palette::MUTED).italic();
    }
    if let Some(text) = line.strip_prefix("- ") {
        return UiNode::row(vec![
            UiNode::text("-").color(palette::MUTED),
            UiNode::text(text),
        ])
        .gap(1);
    }
    UiNode::text(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_heading_is_styled_text() {
        let node = Markdown::new("# Title").build();

        match node {
            UiNode::Column(column) => match &column.children[0] {
                UiNode::Text(text) => assert_eq!(text.content, "Title"),
                _ => panic!("expected text node"),
            },
            _ => panic!("expected column node"),
        }
    }
}
