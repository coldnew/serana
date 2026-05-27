use crate::mora::buffer::Cursor;
use super::display::event::MoraKeyEvent as KeyEvent;

#[derive(Debug, Clone)]
pub enum RegisterValue {
    Text(String),
    Lines(Vec<String>),
    Position(Cursor),
    Macro(Vec<KeyEvent>),
    Rectangle(Vec<String>),
    Number(usize),
}

#[derive(Debug)]
pub struct Registers {
    entries: Vec<Option<RegisterValue>>,
}

impl Registers {
    pub fn new() -> Self {
        Self {
            entries: vec![None; 128],
        }
    }

    pub fn set(&mut self, name: char, value: RegisterValue) {
        let idx = (name as u8) as usize;
        if idx < self.entries.len() {
            self.entries[idx] = Some(value);
        }
    }

    pub fn get(&self, name: char) -> Option<&RegisterValue> {
        let idx = (name as u8) as usize;
        self.entries.get(idx).and_then(|r| r.as_ref())
    }

    pub fn get_mut(&mut self, name: char) -> Option<&mut RegisterValue> {
        let idx = (name as u8) as usize;
        self.entries.get_mut(idx).and_then(|r| r.as_mut())
    }

    pub fn text(&self, name: char) -> Option<&str> {
        match self.get(name)? {
            RegisterValue::Text(t) => Some(t),
            RegisterValue::Lines(l) => l.first().map(|s| s.as_str()),
            _ => None,
        }
    }

    pub fn clear(&mut self, name: char) {
        let idx = (name as u8) as usize;
        if idx < self.entries.len() {
            self.entries[idx] = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_text() {
        let mut reg = Registers::new();
        reg.set('a', RegisterValue::Text("hello".to_string()));
        assert_eq!(reg.text('a'), Some("hello"));
    }

    #[test]
    fn test_register_default_empty() {
        let reg = Registers::new();
        assert!(reg.get('z').is_none());
    }

    #[test]
    fn test_register_position() {
        let mut reg = Registers::new();
        reg.set('m', RegisterValue::Position(Cursor { row: 10, col: 5 }));
        match reg.get('m').unwrap() {
            RegisterValue::Position(c) => {
                assert_eq!(c.row, 10);
                assert_eq!(c.col, 5);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_register_clear() {
        let mut reg = Registers::new();
        reg.set('x', RegisterValue::Text("data".to_string()));
        assert!(reg.get('x').is_some());
        reg.clear('x');
        assert!(reg.get('x').is_none());
    }
}
