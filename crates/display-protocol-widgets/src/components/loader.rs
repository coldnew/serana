use crate::palette;
use display_protocol::{UiNode, Wrap};

#[derive(Debug, Clone)]
pub struct Loader {
    label: Option<String>,
    frame: usize,
}

impl Loader {
    pub fn new(frame: usize) -> Self {
        Self { label: None, frame }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn build(self) -> UiNode {
        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let spinner = frames[self.frame % frames.len()];
        match self.label {
            Some(label) => UiNode::text(format!("{spinner} {label}"))
                .color(palette::MUTED)
                .wrap(Wrap::NoWrap),
            None => UiNode::text(spinner)
                .color(palette::PRIMARY)
                .wrap(Wrap::NoWrap),
        }
    }
}

impl From<Loader> for UiNode {
    fn from(loader: Loader) -> Self {
        loader.build()
    }
}

#[derive(Debug, Clone)]
pub struct CancellableLoader {
    label: String,
    cancel_hint: String,
    frame: usize,
}

impl CancellableLoader {
    pub fn new(label: impl Into<String>, frame: usize) -> Self {
        Self {
            label: label.into(),
            cancel_hint: "Esc to cancel".to_string(),
            frame,
        }
    }

    pub fn cancel_hint(mut self, hint: impl Into<String>) -> Self {
        self.cancel_hint = hint.into();
        self
    }

    pub fn build(self) -> UiNode {
        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        UiNode::text(format!(
            "{} {}  {}",
            frames[self.frame % frames.len()],
            self.label,
            self.cancel_hint
        ))
        .color(palette::MUTED)
        .wrap(Wrap::NoWrap)
    }
}

impl From<CancellableLoader> for UiNode {
    fn from(loader: CancellableLoader) -> Self {
        loader.build()
    }
}
