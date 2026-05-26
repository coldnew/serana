use std::collections::VecDeque;

const KILL_RING_MAX: usize = 60;

#[derive(Debug, Clone)]
pub struct KillEntry {
    pub text: String,
    pub rect: bool,
}

#[derive(Debug)]
pub struct KillRing {
    entries: VecDeque<KillEntry>,
    index: usize,
}

impl KillRing {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            index: 0,
        }
    }

    pub fn kill(&mut self, text: &str, rect: bool) {
        if text.is_empty() {
            return;
        }
        self.entries.push_front(KillEntry {
            text: text.to_string(),
            rect,
        });
        if self.entries.len() > KILL_RING_MAX {
            self.entries.pop_back();
        }
        self.index = 0;
    }

    pub fn append_kill(&mut self, text: &str, rect: bool) {
        if let Some(front) = self.entries.front_mut() {
            front.text.push_str(text);
        } else {
            self.entries.push_front(KillEntry {
                text: text.to_string(),
                rect,
            });
        }
    }

    pub fn yank(&self) -> Option<&KillEntry> {
        self.entries.get(self.index)
    }

    pub fn yank_pop_forward(&mut self) -> Option<&KillEntry> {
        if self.entries.is_empty() {
            return None;
        }
        self.index = (self.index + 1) % self.entries.len();
        self.entries.get(self.index)
    }

    pub fn yank_pop_backward(&mut self) -> Option<&KillEntry> {
        if self.entries.is_empty() {
            return None;
        }
        self.index = if self.index == 0 {
            self.entries.len() - 1
        } else {
            self.index - 1
        };
        self.entries.get(self.index)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kill_and_yank() {
        let mut kr = KillRing::new();
        kr.kill("first", false);
        kr.kill("second", false);
        assert_eq!(kr.yank().unwrap().text, "second");
        assert_eq!(kr.len(), 2);
    }

    #[test]
    fn test_yank_pop_forward() {
        let mut kr = KillRing::new();
        kr.kill("a", false);
        kr.kill("b", false);
        kr.kill("c", false);
        assert_eq!(kr.yank().unwrap().text, "c");
        assert_eq!(kr.yank_pop_forward().unwrap().text, "b");
    }

    #[test]
    fn test_empty_kill() {
        let mut kr = KillRing::new();
        kr.kill("", false);
        assert!(kr.yank().is_none());
    }

    #[test]
    fn test_kill_ring_max() {
        let mut kr = KillRing::new();
        for i in 0..KILL_RING_MAX + 10 {
            kr.kill(&i.to_string(), false);
        }
        assert_eq!(kr.len(), KILL_RING_MAX);
    }

    #[test]
    fn test_append_kill() {
        let mut kr = KillRing::new();
        kr.kill("hello ", false);
        kr.append_kill("world", false);
        assert_eq!(kr.yank().unwrap().text, "hello world");
        assert_eq!(kr.len(), 1);
    }

    #[test]
    fn test_yank_pop_cycles() {
        let mut kr = KillRing::new();
        kr.kill("a", false);
        kr.kill("b", false);
        kr.kill("c", false);
        assert_eq!(kr.yank().unwrap().text, "c");
        assert_eq!(kr.yank_pop_forward().unwrap().text, "b");
        assert_eq!(kr.yank_pop_forward().unwrap().text, "a");
        assert_eq!(kr.yank_pop_forward().unwrap().text, "c");
    }
}
