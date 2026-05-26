use crossterm::event::KeyEvent;

#[derive(Debug, Clone)]
pub struct KeyboardMacro {
    pub name: char,
    pub events: Vec<KeyEvent>,
    pub counter: usize,
}

#[derive(Debug)]
pub struct MacroState {
    recording: bool,
    record_name: char,
    current_events: Vec<KeyEvent>,
    playing: bool,
    play_index: usize,
    macros: Vec<KeyboardMacro>,
}

impl MacroState {
    pub fn new() -> Self {
        Self {
            recording: false,
            record_name: '\0',
            current_events: Vec::new(),
            playing: false,
            play_index: 0,
            macros: Vec::new(),
        }
    }

    pub fn is_recording(&self) -> bool {
        self.recording
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn start_recording(&mut self, name: char) {
        self.recording = true;
        self.record_name = name;
        self.current_events.clear();
    }

    pub fn stop_recording(&mut self) {
        self.recording = false;
        if !self.current_events.is_empty() {
            if let Some(existing) = self
                .macros
                .iter_mut()
                .find(|m| m.name == self.record_name)
            {
                existing.events = self.current_events.clone();
            } else {
                self.macros.push(KeyboardMacro {
                    name: self.record_name,
                    events: self.current_events.clone(),
                    counter: 0,
                });
            }
        }
        self.current_events.clear();
    }

    pub fn record_key(&mut self, key: &KeyEvent) {
        if self.recording {
            self.current_events.push(*key);
        }
    }

    pub fn start_playback(&mut self, name: char) {
        if let Some(m) = self.macros.iter_mut().find(|m| m.name == name) {
            m.counter = m.counter.wrapping_add(1);
            self.playing = true;
            self.play_index = 0;
            self.current_events = m.events.clone();
        }
    }

    pub fn resume_playback(&mut self) {
        self.playing = true;
        self.play_index = 0;
    }

    pub fn next_event(&mut self) -> Option<KeyEvent> {
        if !self.playing {
            return None;
        }
        let idx = self.play_index;
        if idx < self.current_events.len() {
            self.play_index += 1;
            Some(self.current_events[idx])
        } else {
            self.playing = false;
            None
        }
    }

    pub fn cancel_playback(&mut self) {
        self.playing = false;
        self.current_events.clear();
    }

    pub fn store_in_register(&self, name: char) -> Option<Vec<KeyEvent>> {
        self.macros
            .iter()
            .find(|m| m.name == name)
            .map(|m| m.events.clone())
    }

    pub fn load_from_register(&mut self, name: char, events: &[KeyEvent]) {
        if let Some(existing) = self
            .macros
            .iter_mut()
            .find(|m| m.name == name)
        {
            existing.events = events.to_vec();
        } else {
            self.macros.push(KeyboardMacro {
                name,
                events: events.to_vec(),
                counter: 0,
            });
        }
    }

    pub fn list_macros(&self) -> &[KeyboardMacro] {
        &self.macros
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn fake_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn fake_ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn test_record_and_playback() {
        let mut ms = MacroState::new();
        assert!(!ms.is_recording());

        ms.start_recording('e');
        assert!(ms.is_recording());
        ms.record_key(&fake_key(KeyCode::Char('h')));
        ms.record_key(&fake_key(KeyCode::Char('i')));
        ms.stop_recording();
        assert!(!ms.is_recording());

        ms.start_playback('e');
        assert_eq!(ms.next_event().unwrap().code, KeyCode::Char('h'));
        assert_eq!(ms.next_event().unwrap().code, KeyCode::Char('i'));
        assert!(ms.next_event().is_none());
    }

    #[test]
    fn test_macro_counter() {
        let mut ms = MacroState::new();
        ms.start_recording('x');
        ms.record_key(&fake_key(KeyCode::Char('a')));
        ms.stop_recording();

        ms.start_playback('x');
        while ms.next_event().is_some() {}
        assert!(!ms.is_playing());

        let macro_ref = ms.list_macros().iter().find(|m| m.name == 'x').unwrap();
        assert_eq!(macro_ref.counter, 1);
    }

    #[test]
    fn test_cancel_playback() {
        let mut ms = MacroState::new();
        ms.start_recording('q');
        ms.record_key(&fake_ctrl('f'));
        ms.stop_recording();
        ms.start_playback('q');
        assert!(ms.is_playing());
        ms.cancel_playback();
        assert!(!ms.is_playing());
    }
}
