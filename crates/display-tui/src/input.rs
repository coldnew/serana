use crossterm::event::{self, Event, KeyCode, KeyEvent as CrosstermKeyEvent, KeyModifiers};
use display_protocol::{
    InputEvent, KeyCode as ProtoKeyCode, KeyEvent, KeyModifiers as ProtoModifiers, MouseEventKind,
};
use std::time::Duration;

/// Poll for terminal input with timeout. Returns display-protocol InputEvent.
pub fn poll_input(timeout_ms: u64) -> Option<InputEvent> {
    if event::poll(Duration::from_millis(timeout_ms)).ok()? {
        match event::read().ok()? {
            Event::Key(key) => {
                let proto_key = crossterm_to_key_event(key);
                Some(InputEvent::Key(proto_key))
            }
            Event::Resize(w, h) => Some(InputEvent::Resize {
                width: w,
                height: h,
            }),
            Event::Mouse(mouse) => {
                let kind = match mouse.kind {
                    event::MouseEventKind::Down(_) => MouseEventKind::Press,
                    event::MouseEventKind::Up(_) => MouseEventKind::Release,
                    event::MouseEventKind::Drag(_) => MouseEventKind::Drag,
                    event::MouseEventKind::ScrollUp => MouseEventKind::ScrollUp,
                    event::MouseEventKind::ScrollDown => MouseEventKind::ScrollDown,
                    _ => return None,
                };
                Some(InputEvent::Mouse {
                    x: mouse.column,
                    y: mouse.row,
                    kind,
                    modifiers: crossterm_mods_to_proto(mouse.modifiers),
                })
            }
            Event::FocusGained => Some(InputEvent::FocusGained),
            Event::FocusLost => Some(InputEvent::FocusLost),
            Event::Paste(text) => Some(InputEvent::Paste(text)),
        }
    } else {
        None
    }
}

/// Convert a crossterm KeyEvent to display-protocol KeyEvent.
pub fn crossterm_to_key_event(key: CrosstermKeyEvent) -> KeyEvent {
    let code = match key.code {
        KeyCode::Char(c) => ProtoKeyCode::Char(c),
        KeyCode::Enter => ProtoKeyCode::Enter,
        KeyCode::Tab => ProtoKeyCode::Tab,
        KeyCode::Backspace => ProtoKeyCode::Backspace,
        KeyCode::Delete => ProtoKeyCode::Delete,
        KeyCode::Esc => ProtoKeyCode::Esc,
        KeyCode::Left => ProtoKeyCode::Left,
        KeyCode::Right => ProtoKeyCode::Right,
        KeyCode::Up => ProtoKeyCode::Up,
        KeyCode::Down => ProtoKeyCode::Down,
        KeyCode::Home => ProtoKeyCode::Home,
        KeyCode::End => ProtoKeyCode::End,
        KeyCode::PageUp => ProtoKeyCode::PageUp,
        KeyCode::PageDown => ProtoKeyCode::PageDown,
        KeyCode::Insert => ProtoKeyCode::Insert,
        KeyCode::BackTab => ProtoKeyCode::BackTab,
        KeyCode::F(n) => ProtoKeyCode::F(n),
        _ => return KeyEvent::new(ProtoKeyCode::Char('\0'), ProtoModifiers::EMPTY),
    };
    let modifiers = crossterm_mods_to_proto(key.modifiers);
    KeyEvent::new(code, modifiers)
}

/// Convert crossterm KeyModifiers to display-protocol KeyModifiers.
pub fn crossterm_mods_to_proto(mods: KeyModifiers) -> ProtoModifiers {
    ProtoModifiers {
        ctrl: mods.contains(KeyModifiers::CONTROL),
        alt: mods.contains(KeyModifiers::ALT),
        shift: mods.contains(KeyModifiers::SHIFT),
        super_key: mods.contains(KeyModifiers::SUPER),
    }
}

/// Convert a crossterm KeyEvent directly to display-protocol InputEvent::Key.
pub fn crossterm_to_input_event(key: CrosstermKeyEvent) -> InputEvent {
    InputEvent::Key(crossterm_to_key_event(key))
}
