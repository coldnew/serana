use display_protocol::{UiNode, Wrap};

#[derive(Debug, Clone)]
pub struct TruncatedText {
    content: String,
    max_chars: usize,
    suffix: String,
}

impl TruncatedText {
    pub fn new(content: impl Into<String>, max_chars: usize) -> Self {
        Self {
            content: content.into(),
            max_chars,
            suffix: String::new(),
        }
    }

    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }

    pub fn build(self) -> UiNode {
        UiNode::text(truncate_chars(&self.content, self.max_chars, &self.suffix)).wrap(Wrap::NoWrap)
    }
}

impl From<TruncatedText> for UiNode {
    fn from(text: TruncatedText) -> Self {
        text.build()
    }
}

fn truncate_chars(text: &str, max_chars: usize, suffix: &str) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_string();
    }
    if max_chars == 0 {
        return suffix.to_string();
    }
    let keep = max_chars.saturating_sub(suffix.chars().count());
    let mut result: String = text.chars().take(keep).collect();
    result.push_str(suffix);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncated_text_applies_suffix() {
        let node = TruncatedText::new("abcdef", 4).suffix("..").build();

        match node {
            UiNode::Text(text) => assert_eq!(text.content, "ab.."),
            _ => panic!("expected text node"),
        }
    }
}
