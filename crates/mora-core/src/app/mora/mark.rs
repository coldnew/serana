use crate::app::mora::buffer::Cursor;

const MARK_RING_MAX: usize = 16;

#[derive(Debug)]
pub struct MarkRing {
    ring: Vec<Cursor>,
    active: bool,
}

impl MarkRing {
    pub fn new() -> Self {
        Self {
            ring: Vec::new(),
            active: false,
        }
    }

    pub fn push(&mut self, pos: Cursor) {
        self.ring.push(pos);
        if self.ring.len() > MARK_RING_MAX {
            self.ring.remove(0);
        }
    }

    pub fn pop(&mut self) -> Option<Cursor> {
        self.ring.pop()
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn peek(&self) -> Option<&Cursor> {
        self.ring.last()
    }

    pub fn clear(&mut self) {
        self.ring.clear();
        self.active = false;
    }

    pub fn len(&self) -> usize {
        self.ring.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mark_push_pop() {
        let mut mr = MarkRing::new();
        assert!(mr.pop().is_none());
        mr.push(Cursor { row: 5, col: 3 });
        mr.push(Cursor { row: 10, col: 7 });
        assert_eq!(mr.pop().unwrap().row, 10);
        assert_eq!(mr.pop().unwrap().row, 5);
        assert!(mr.pop().is_none());
    }

    #[test]
    fn test_mark_active() {
        let mut mr = MarkRing::new();
        assert!(!mr.is_active());
        mr.set_active(true);
        assert!(mr.is_active());
    }

    #[test]
    fn test_mark_ring_max() {
        let mut mr = MarkRing::new();
        for i in 0..MARK_RING_MAX + 5 {
            mr.push(Cursor { row: i, col: 0 });
        }
        assert_eq!(mr.len(), MARK_RING_MAX);
        assert_eq!(mr.peek().unwrap().row, MARK_RING_MAX + 4);
    }
}
