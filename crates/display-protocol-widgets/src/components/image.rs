use display_protocol::{CanvasNode, Color, UiNode};

#[derive(Debug, Clone)]
pub struct Image {
    frame_id: String,
    width: u16,
    height: u16,
    bg: Color,
}

impl Image {
    pub fn new(frame_id: impl Into<String>, width: u16, height: u16) -> Self {
        Self {
            frame_id: frame_id.into(),
            width,
            height,
            bg: Color::BLACK,
        }
    }

    pub fn background(mut self, color: Color) -> Self {
        self.bg = color;
        self
    }

    pub fn build(self) -> UiNode {
        UiNode::Canvas(CanvasNode {
            width: self.width,
            height: self.height,
            frame_id: self.frame_id,
            bg: self.bg,
        })
    }
}

impl From<Image> for UiNode {
    fn from(image: Image) -> Self {
        image.build()
    }
}
