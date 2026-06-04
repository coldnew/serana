use display_protocol::{InputEvent, KeyCode, KeyEvent, KeyModifiers, MouseEventKind};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{Key, ModifiersState, NamedKey};

/// Convert a winit `WindowEvent` into a display-protocol `InputEvent`.
///
/// `modifiers` is the most recently observed `ModifiersState` (from
/// `ModifiersChanged`); winit does not include modifier info on
/// `KeyboardInput` events, so the caller must track it.
///
/// Returns `None` for events that don't map to the display protocol
/// (e.g. redraw requested, close requested).
pub fn winit_to_input_event(event: &WindowEvent, modifiers: ModifiersState) -> Option<InputEvent> {
    let proto_mods = winit_mods_to_proto(modifiers);
    match event {
        WindowEvent::KeyboardInput { event, .. } => {
            if event.state == ElementState::Pressed {
                let code = winit_key_to_code(&event.logical_key);
                Some(InputEvent::Key(KeyEvent::new(code, proto_mods)))
            } else {
                None
            }
        }
        WindowEvent::MouseInput {
            state, button: _, ..
        } => {
            let kind = match state {
                ElementState::Pressed => MouseEventKind::Press,
                ElementState::Released => MouseEventKind::Release,
            };
            // Position is tracked separately via CursorMoved
            Some(InputEvent::Mouse {
                x: 0,
                y: 0,
                kind,
                modifiers: proto_mods,
            })
        }
        WindowEvent::CursorMoved { position, .. } => Some(InputEvent::Mouse {
            x: position.x as u16,
            y: position.y as u16,
            kind: MouseEventKind::Drag,
            modifiers: proto_mods,
        }),
        WindowEvent::MouseWheel { delta, .. } => {
            let (_scroll_x, scroll_y) = match delta {
                MouseScrollDelta::LineDelta(dx, dy) => (*dx as f64, *dy as f64),
                MouseScrollDelta::PixelDelta(pos) => (pos.x, pos.y),
            };
            let kind = if scroll_y > 0.0 {
                MouseEventKind::ScrollUp
            } else if scroll_y < 0.0 {
                MouseEventKind::ScrollDown
            } else {
                return None;
            };
            Some(InputEvent::Mouse {
                x: 0,
                y: 0,
                kind,
                modifiers: proto_mods,
            })
        }
        WindowEvent::Resized(size) => Some(InputEvent::Resize {
            width: size.width as u16,
            height: size.height as u16,
        }),
        WindowEvent::Focused(focused) => {
            if *focused {
                Some(InputEvent::FocusGained)
            } else {
                Some(InputEvent::FocusLost)
            }
        }
        _ => None,
    }
}

/// Convert winit `Key` to display-protocol `KeyCode`.
pub fn winit_key_to_code(key: &Key) -> KeyCode {
    match key {
        Key::Character(s) => {
            let ch = s.chars().next().unwrap_or('\0');
            KeyCode::Char(ch)
        }
        Key::Named(named) => match named {
            NamedKey::Enter => KeyCode::Enter,
            NamedKey::Tab => KeyCode::Tab,
            NamedKey::Backspace => KeyCode::Backspace,
            NamedKey::Delete => KeyCode::Delete,
            NamedKey::Escape => KeyCode::Esc,
            NamedKey::ArrowLeft => KeyCode::Left,
            NamedKey::ArrowRight => KeyCode::Right,
            NamedKey::ArrowUp => KeyCode::Up,
            NamedKey::ArrowDown => KeyCode::Down,
            NamedKey::Home => KeyCode::Home,
            NamedKey::End => KeyCode::End,
            NamedKey::PageUp => KeyCode::PageUp,
            NamedKey::PageDown => KeyCode::PageDown,
            NamedKey::Insert => KeyCode::Insert,
            NamedKey::F1 => KeyCode::F(1),
            NamedKey::F2 => KeyCode::F(2),
            NamedKey::F3 => KeyCode::F(3),
            NamedKey::F4 => KeyCode::F(4),
            NamedKey::F5 => KeyCode::F(5),
            NamedKey::F6 => KeyCode::F(6),
            NamedKey::F7 => KeyCode::F(7),
            NamedKey::F8 => KeyCode::F(8),
            NamedKey::F9 => KeyCode::F(9),
            NamedKey::F10 => KeyCode::F(10),
            NamedKey::F11 => KeyCode::F(11),
            NamedKey::F12 => KeyCode::F(12),
            NamedKey::F13 => KeyCode::F(13),
            NamedKey::F14 => KeyCode::F(14),
            NamedKey::F15 => KeyCode::F(15),
            NamedKey::F16 => KeyCode::F(16),
            NamedKey::F17 => KeyCode::F(17),
            NamedKey::F18 => KeyCode::F(18),
            NamedKey::F19 => KeyCode::F(19),
            NamedKey::F20 => KeyCode::F(20),
            NamedKey::F21 => KeyCode::F(21),
            NamedKey::F22 => KeyCode::F(22),
            NamedKey::F23 => KeyCode::F(23),
            NamedKey::F24 => KeyCode::F(24),
            NamedKey::F25 => KeyCode::F(25),
            _ => KeyCode::Char('\0'),
        },
        _ => KeyCode::Char('\0'),
    }
}

/// Convert winit `ModifiersState` to display-protocol `KeyModifiers`.
pub fn winit_mods_to_proto(mods: ModifiersState) -> KeyModifiers {
    KeyModifiers {
        ctrl: mods.control_key(),
        alt: mods.alt_key(),
        shift: mods.shift_key(),
        super_key: mods.super_key(),
    }
}

/// Convert winit `MouseButton` to a character label.
pub fn mouse_button_label(button: &MouseButton) -> char {
    match button {
        MouseButton::Left => 'L',
        MouseButton::Right => 'R',
        MouseButton::Middle => 'M',
        MouseButton::Back => 'B',
        MouseButton::Forward => 'F',
        MouseButton::Other(n) => char::from(*n as u8),
    }
}
