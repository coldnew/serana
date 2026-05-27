use std::collections::HashMap;

use super::display::style::{MoraColor, MoraStyle};

#[derive(Debug, Clone)]
pub struct OverlayFace {
    pub fg: Option<MoraColor>,
    pub bg: Option<MoraColor>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub strikethrough: Option<bool>,
}

impl OverlayFace {
    pub fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            bold: None,
            italic: None,
            underline: None,
            strikethrough: None,
        }
    }

    pub fn apply_to(&self, base: MoraStyle) -> MoraStyle {
        let mut style = base;
        if let Some(fg) = self.fg {
            style.fg = Some(fg);
        }
        if let Some(bg) = self.bg {
            style.bg = Some(bg);
        }
        if let Some(bold) = self.bold {
            style.bold = bold;
        }
        if let Some(italic) = self.italic {
            style.italic = italic;
        }
        if let Some(underline) = self.underline {
            style.underline = underline;
        }
        if let Some(strikethrough) = self.strikethrough {
            style.strikethrough = strikethrough;
        }
        style
    }
}

#[derive(Debug, Clone)]
pub struct Overlay {
    pub id: usize,
    pub start: usize,
    pub end: usize,
    pub priority: i32,
    pub face: Option<OverlayFace>,
    pub read_only: bool,
    pub invisible: bool,
    pub category: Option<String>,
    pub properties: HashMap<String, String>,
}

impl Overlay {
    pub fn new(id: usize, start: usize, end: usize) -> Self {
        Self {
            id,
            start,
            end,
            priority: 0,
            face: None,
            read_only: false,
            invisible: false,
            category: None,
            properties: HashMap::new(),
        }
    }

    pub fn contains(&self, pos: usize) -> bool {
        pos >= self.start && pos < self.end
    }

    pub fn overlaps(&self, start: usize, end: usize) -> bool {
        self.start < end && self.end > start
    }
}

#[derive(Debug)]
pub struct OverlayStore {
    overlays: Vec<Overlay>,
    next_id: usize,
}

impl OverlayStore {
    pub fn new() -> Self {
        Self {
            overlays: Vec::new(),
            next_id: 1,
        }
    }

    pub fn add(&mut self, start: usize, end: usize) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.overlays.push(Overlay::new(id, start, end));
        id
    }

    pub fn remove(&mut self, id: usize) -> bool {
        let len = self.overlays.len();
        self.overlays.retain(|o| o.id != id);
        self.overlays.len() < len
    }

    pub fn get(&self, id: usize) -> Option<&Overlay> {
        self.overlays.iter().find(|o| o.id == id)
    }

    pub fn get_mut(&mut self, id: usize) -> Option<&mut Overlay> {
        self.overlays.iter_mut().find(|o| o.id == id)
    }

    pub fn overlays_at(&self, pos: usize) -> Vec<&Overlay> {
        let mut result: Vec<&Overlay> = self.overlays.iter().filter(|o| o.contains(pos)).collect();
        result.sort_by_key(|o| -o.priority);
        result
    }

    pub fn overlays_in(&self, start: usize, end: usize) -> Vec<&Overlay> {
        let mut result: Vec<&Overlay> = self.overlays.iter().filter(|o| o.overlaps(start, end)).collect();
        result.sort_by_key(|o| -o.priority);
        result
    }

    pub fn clear(&mut self) {
        self.overlays.clear();
    }

    pub fn len(&self) -> usize {
        self.overlays.len()
    }

    pub fn is_empty(&self) -> bool {
        self.overlays.is_empty()
    }

    pub fn adjust_for_insert(&mut self, pos: usize, len: usize) {
        for ov in &mut self.overlays {
            if ov.start >= pos {
                ov.start += len;
                ov.end += len;
            } else if ov.end > pos {
                ov.end += len;
            }
        }
    }

    pub fn adjust_for_delete(&mut self, pos: usize, len: usize) {
        for ov in &mut self.overlays {
            if ov.start >= pos + len {
                ov.start -= len;
                ov.end -= len;
            } else if ov.start >= pos {
                ov.start = pos;
                ov.end = ov.end.saturating_sub(len).max(pos);
            } else if ov.end > pos {
                ov.end = ov.end.saturating_sub(len).max(pos);
            }
        }
        self.overlays.retain(|o| o.start < o.end);
    }

    pub fn style_at(&self, pos: usize, base: MoraStyle) -> MoraStyle {
        let overlays = self.overlays_at(pos);
        let mut style = base;
        for ov in &overlays {
            if let Some(ref face) = ov.face {
                style = face.apply_to(style);
            }
        }
        style
    }

    pub fn is_invisible(&self, pos: usize) -> bool {
        self.overlays.iter().any(|o| o.invisible && o.contains(pos))
    }

    pub fn is_read_only(&self, pos: usize) -> bool {
        self.overlays.iter().any(|o| o.read_only && o.contains(pos))
    }

    pub fn is_range_read_only(&self, start: usize, end: usize) -> bool {
        self.overlays.iter().any(|o| o.read_only && o.overlaps(start, end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_add_remove() {
        let mut store = OverlayStore::new();
        let id1 = store.add(10, 20);
        let id2 = store.add(15, 25);
        assert_eq!(store.len(), 2);

        assert!(store.remove(id1));
        assert_eq!(store.len(), 1);
        assert!(store.get(id2).is_some());
    }

    #[test]
    fn test_overlay_at() {
        let mut store = OverlayStore::new();
        store.add(10, 20);
        store.add(15, 25);

        let at_12 = store.overlays_at(12);
        assert_eq!(at_12.len(), 1);

        let at_17 = store.overlays_at(17);
        assert_eq!(at_17.len(), 2);

        let at_30 = store.overlays_at(30);
        assert_eq!(at_30.len(), 0);
    }

    #[test]
    fn test_overlay_priority() {
        let mut store = OverlayStore::new();
        let id1 = store.add(10, 20);
        let id2 = store.add(10, 20);
        store.get_mut(id1).unwrap().priority = 1;
        store.get_mut(id2).unwrap().priority = 5;

        let overlays = store.overlays_at(15);
        assert_eq!(overlays[0].id, id2);
        assert_eq!(overlays[1].id, id1);
    }

    #[test]
    fn test_overlay_face_apply() {
        let mut store = OverlayStore::new();
        let id = store.add(10, 20);
        {
            let ov = store.get_mut(id).unwrap();
            ov.face = Some(OverlayFace {
                fg: Some(MoraColor::new(255, 0, 0)),
                bg: None,
                bold: Some(true),
                italic: None,
                underline: None,
                strikethrough: None,
            });
        }

        let base = MoraStyle::new().fg(MoraColor::new(200, 200, 200));
        let styled = store.style_at(15, base);
        assert_eq!(styled.fg, Some(MoraColor::new(255, 0, 0)));
        assert!(styled.bold);
    }

    #[test]
    fn test_overlay_invisible() {
        let mut store = OverlayStore::new();
        let id = store.add(10, 20);
        store.get_mut(id).unwrap().invisible = true;

        assert!(store.is_invisible(15));
        assert!(!store.is_invisible(5));
        assert!(!store.is_invisible(25));
    }

    #[test]
    fn test_overlay_read_only() {
        let mut store = OverlayStore::new();
        let id = store.add(10, 20);
        store.get_mut(id).unwrap().read_only = true;

        assert!(store.is_read_only(15));
        assert!(!store.is_read_only(5));
        assert!(store.is_range_read_only(5, 15));
        assert!(!store.is_range_read_only(25, 30));
    }

    #[test]
    fn test_overlay_adjust_insert() {
        let mut store = OverlayStore::new();
        let id = store.add(10, 20);
        store.adjust_for_insert(5, 10);

        let ov = store.get(id).unwrap();
        assert_eq!(ov.start, 20);
        assert_eq!(ov.end, 30);
    }

    #[test]
    fn test_overlay_adjust_insert_inside() {
        let mut store = OverlayStore::new();
        let id = store.add(10, 20);
        store.adjust_for_insert(15, 5);

        let ov = store.get(id).unwrap();
        assert_eq!(ov.start, 10);
        assert_eq!(ov.end, 25);
    }

    #[test]
    fn test_overlay_adjust_delete() {
        let mut store = OverlayStore::new();
        let id = store.add(10, 20);
        store.adjust_for_delete(5, 3);

        let ov = store.get(id).unwrap();
        assert_eq!(ov.start, 7);
        assert_eq!(ov.end, 17);
    }

    #[test]
    fn test_overlay_adjust_delete_overlapping() {
        let mut store = OverlayStore::new();
        let id = store.add(10, 20);
        store.adjust_for_delete(12, 5);

        let ov = store.get(id).unwrap();
        assert_eq!(ov.start, 10);
        assert_eq!(ov.end, 15);
    }
}
