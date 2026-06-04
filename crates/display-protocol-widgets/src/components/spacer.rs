use super::BoxComponent;
use display_protocol::UiNode;

#[derive(Debug, Clone)]
pub struct Spacer {
    width: Option<u16>,
    height: Option<u16>,
}

impl Spacer {
    pub fn new() -> Self {
        Self {
            width: None,
            height: None,
        }
    }

    pub fn width(width: u16) -> Self {
        Self {
            width: Some(width),
            height: None,
        }
    }

    pub fn height(height: u16) -> Self {
        Self {
            width: None,
            height: Some(height),
        }
    }

    pub fn build(self) -> UiNode {
        BoxComponent::new(Vec::new())
            .width_opt(self.width)
            .height_opt(self.height)
            .build()
    }
}

impl Default for Spacer {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Spacer> for UiNode {
    fn from(spacer: Spacer) -> Self {
        spacer.build()
    }
}
