use crate::layout::{ElementId, LayoutResult, UiNodeKind};
use crate::render::{RenderCommand, RenderCommandArray};
use crate::types::Color;

/// Public layout data for inspector and debug tools.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElementData {
    pub id: ElementId,
    pub kind: UiNodeKind,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub depth: usize,
}

impl ElementData {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height
    }
}

/// Return a preorder view of every laid out element.
pub fn flatten_layout(layout: &LayoutResult) -> Vec<ElementData> {
    let mut out = Vec::new();
    push_flattened(layout, 0, &mut out);
    out
}

fn push_flattened(layout: &LayoutResult, depth: usize, out: &mut Vec<ElementData>) {
    if let Some(id) = layout.id {
        out.push(ElementData {
            id,
            kind: layout.kind,
            x: layout.x,
            y: layout.y,
            width: layout.width,
            height: layout.height,
            depth,
        });
    }

    for child in &layout.children {
        push_flattened(child, depth + 1, out);
    }
}

/// Return layout data for one element id.
pub fn element_data(layout: &LayoutResult, id: ElementId) -> Option<ElementData> {
    flatten_layout(layout)
        .into_iter()
        .find(|element| element.id == id)
}

/// Return the deepest element containing a point.
pub fn hit_test(layout: &LayoutResult, x: f32, y: f32) -> Option<ElementData> {
    let mut hit = None;
    hit_test_inner(layout, x, y, 0, &mut hit);
    hit
}

/// Return all elements containing a point, ordered root to deepest child.
pub fn pointer_over_elements(layout: &LayoutResult, x: f32, y: f32) -> Vec<ElementData> {
    let mut elements = Vec::new();
    pointer_over_inner(layout, x, y, 0, &mut elements);
    elements
}

/// Return all element ids containing a point, ordered root to deepest child.
pub fn pointer_over_ids(layout: &LayoutResult, x: f32, y: f32) -> Vec<ElementId> {
    pointer_over_elements(layout, x, y)
        .into_iter()
        .map(|element| element.id)
        .collect()
}

fn hit_test_inner(
    layout: &LayoutResult,
    x: f32,
    y: f32,
    depth: usize,
    hit: &mut Option<ElementData>,
) {
    if !layout.contains(x, y) {
        return;
    }

    if let Some(id) = layout.id {
        *hit = Some(ElementData {
            id,
            kind: layout.kind,
            x: layout.x,
            y: layout.y,
            width: layout.width,
            height: layout.height,
            depth,
        });
    }

    for child in &layout.children {
        hit_test_inner(child, x, y, depth + 1, hit);
    }
}

fn pointer_over_inner(
    layout: &LayoutResult,
    x: f32,
    y: f32,
    depth: usize,
    elements: &mut Vec<ElementData>,
) {
    if !layout.contains(x, y) {
        return;
    }

    if let Some(id) = layout.id {
        elements.push(ElementData {
            id,
            kind: layout.kind,
            x: layout.x,
            y: layout.y,
            width: layout.width,
            height: layout.height,
            depth,
        });
    }

    for child in &layout.children {
        pointer_over_inner(child, x, y, depth + 1, elements);
    }
}

/// Build overlay commands for a Clay-style layout inspector.
pub fn debug_overlay_commands(layout: &LayoutResult) -> Vec<RenderCommand> {
    let mut commands = Vec::new();
    push_debug_overlay(layout, &mut commands);
    commands
}

/// Append debug overlay commands to an existing render command array.
pub fn append_debug_overlay(commands: &mut RenderCommandArray, layout: &LayoutResult) {
    for command in debug_overlay_commands(layout) {
        commands.push_command(command);
    }
}

fn push_debug_overlay(layout: &LayoutResult, commands: &mut Vec<RenderCommand>) {
    let w = layout.width.ceil().max(0.0) as u16;
    let h = layout.height.ceil().max(0.0) as u16;
    if w > 1 && h > 1 {
        let title = layout.id.map(|id| format!("{:?}#{}", layout.kind, id.0));
        commands.push(RenderCommand::DrawBorder {
            x: layout.x.max(0.0) as u16,
            y: layout.y.max(0.0) as u16,
            w,
            h,
            fg: Color::YELLOW,
            bg: Color::BLACK,
            title,
        });
    }

    for child in &layout.children {
        push_debug_overlay(child, commands);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compute_layout, hash_element_key, UiNode};

    #[test]
    fn flatten_layout_reports_node_kinds_and_ids() {
        let node = UiNode::column(vec![UiNode::text("a"), UiNode::text("b")]);
        let layout = compute_layout(&node, 20, 5);
        let elements = flatten_layout(&layout);
        assert_eq!(elements.len(), 3);
        assert_eq!(elements[0].kind, UiNodeKind::Column);
        assert_eq!(elements[1].kind, UiNodeKind::Text);
        assert_ne!(elements[0].id, elements[1].id);
    }

    #[test]
    fn hit_test_returns_deepest_element() {
        let node = UiNode::column(vec![UiNode::text("a"), UiNode::text("b")]);
        let layout = compute_layout(&node, 20, 5);
        let hit = hit_test(&layout, 0.0, 1.0).expect("hit");
        assert_eq!(hit.kind, UiNodeKind::Text);
        assert_eq!(hit.depth, 1);
    }

    #[test]
    fn pointer_over_elements_returns_ancestry() {
        let node = UiNode::column(vec![UiNode::text("a"), UiNode::text("b")]);
        let layout = compute_layout(&node, 20, 5);
        let elements = pointer_over_elements(&layout, 0.0, 1.0);
        assert_eq!(elements.len(), 2);
        assert_eq!(elements[0].kind, UiNodeKind::Column);
        assert_eq!(elements[1].kind, UiNodeKind::Text);
    }

    #[test]
    fn keyed_node_uses_stable_hash_id() {
        let node = UiNode::keyed("modeline", UiNode::text("mode"));
        let layout = compute_layout(&node, 20, 5);
        assert_eq!(layout.kind, UiNodeKind::Keyed);
        assert_eq!(layout.id, Some(ElementId(hash_element_key("modeline"))));
    }

    #[test]
    fn debug_overlay_uses_render_commands() {
        let node = UiNode::boxed(vec![UiNode::text("content")]).height(3);
        let layout = compute_layout(&node, 20, 5);
        let commands = debug_overlay_commands(&layout);
        assert!(matches!(
            commands.first(),
            Some(RenderCommand::DrawBorder { .. })
        ));
    }
}
