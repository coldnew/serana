use crate::palette;
use display_protocol::UiNode;

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
        let frames = ["|", "/", "-", "\\"];
        let spinner = UiNode::text(frames[self.frame % frames.len()]).color(palette::PRIMARY);
        match self.label {
            Some(label) => {
                UiNode::row(vec![spinner, UiNode::text(label).color(palette::MUTED)]).gap(1)
            }
            None => spinner,
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
        UiNode::row(vec![
            Loader::new(self.frame).label(self.label).build(),
            UiNode::text(self.cancel_hint).color(palette::MUTED).dim(),
        ])
        .gap(2)
    }
}

impl From<CancellableLoader> for UiNode {
    fn from(loader: CancellableLoader) -> Self {
        loader.build()
    }
}
