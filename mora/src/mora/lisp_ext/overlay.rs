use std::sync::Arc;

use crate::lisp::ns::Namespace;
use crate::lisp::types::Value;

use super::editor_state::{with_editor_state, with_editor_state_mut};
use super::helpers::{extract_int, extract_string};
use super::super::display::style::MoraColor;
use super::super::overlay::OverlayFace;

fn parse_color(s: &str) -> Option<MoraColor> {
    let s = s.trim();
    if s.starts_with('#') && s.len() == 7 {
        let r = u8::from_str_radix(&s[1..3], 16).ok()?;
        let g = u8::from_str_radix(&s[3..5], 16).ok()?;
        let b = u8::from_str_radix(&s[5..7], 16).ok()?;
        Some(MoraColor::new(r, g, b))
    } else {
        match s.to_lowercase().as_str() {
            "red" => Some(MoraColor::new(255, 0, 0)),
            "green" => Some(MoraColor::new(0, 255, 0)),
            "blue" => Some(MoraColor::new(0, 0, 255)),
            "yellow" => Some(MoraColor::new(255, 255, 0)),
            "cyan" => Some(MoraColor::new(0, 255, 255)),
            "magenta" => Some(MoraColor::new(255, 0, 255)),
            "white" => Some(MoraColor::new(255, 255, 255)),
            "black" => Some(MoraColor::new(0, 0, 0)),
            "orange" => Some(MoraColor::new(255, 165, 0)),
            "gray" | "grey" => Some(MoraColor::new(128, 128, 128)),
            _ => None,
        }
    }
}

fn prim_make_overlay(args: &[Value]) -> Result<Value, String> {
    let start = extract_int(args, 0)? as usize;
    let end = extract_int(args, 1)? as usize;
    with_editor_state_mut(|state| {
        let id = state.overlays.add(start, end);
        Ok(Value::Int(id as i64))
    })
}

fn prim_overlay_put_face(args: &[Value]) -> Result<Value, String> {
    let id = extract_int(args, 0)? as usize;
    let fg = args.get(1).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        _ => None,
    });
    let bg = args.get(2).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        _ => None,
    });
    let bold = args.get(3).and_then(|v| match v {
        Value::Bool(b) => Some(*b),
        _ => None,
    });
    with_editor_state_mut(|state| {
        if let Some(ov) = state.overlays.get_mut(id) {
            let mut face = OverlayFace::new();
            if let Some(fg_str) = fg {
                face.fg = parse_color(&fg_str);
            }
            if let Some(bg_str) = bg {
                face.bg = parse_color(&bg_str);
            }
            face.bold = bold;
            ov.face = Some(face);
        }
        Ok(Value::Nil)
    })
}

fn prim_overlay_put_property(args: &[Value]) -> Result<Value, String> {
    let id = extract_int(args, 0)? as usize;
    let key = extract_string(args, 1)?;
    let val = extract_string(args, 2)?;
    with_editor_state_mut(|state| {
        if let Some(ov) = state.overlays.get_mut(id) {
            ov.properties.insert(key, val);
        }
        Ok(Value::Nil)
    })
}

fn prim_overlay_delete(args: &[Value]) -> Result<Value, String> {
    let id = extract_int(args, 0)? as usize;
    with_editor_state_mut(|state| {
        state.overlays.remove(id);
        Ok(Value::Nil)
    })
}

fn prim_overlay_get(args: &[Value]) -> Result<Value, String> {
    let id = extract_int(args, 0)? as usize;
    let key = extract_string(args, 1)?;
    with_editor_state(|state| {
        if let Some(ov) = state.overlays.get(id) {
            if let Some(val) = ov.properties.get(&key) {
                Ok(Value::string(val.clone()))
            } else {
                Ok(Value::Nil)
            }
        } else {
            Ok(Value::Nil)
        }
    })
}

fn prim_overlays_at(args: &[Value]) -> Result<Value, String> {
    let pos = extract_int(args, 0)? as usize;
    with_editor_state(|state| {
        let overlays = state.overlays.overlays_at(pos);
        let ids: Vec<Value> = overlays.iter().map(|o| Value::Int(o.id as i64)).collect();
        Ok(Value::Vector(Arc::new(ids)))
    })
}

fn prim_overlay_put_invisible(args: &[Value]) -> Result<Value, String> {
    let id = extract_int(args, 0)? as usize;
    let invisible = args
        .get(1)
        .and_then(|v| match v {
            Value::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(true);
    with_editor_state_mut(|state| {
        if let Some(ov) = state.overlays.get_mut(id) {
            ov.invisible = invisible;
        }
        Ok(Value::Nil)
    })
}

fn prim_overlay_put_read_only(args: &[Value]) -> Result<Value, String> {
    let id = extract_int(args, 0)? as usize;
    let read_only = args
        .get(1)
        .and_then(|v| match v {
            Value::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(true);
    with_editor_state_mut(|state| {
        if let Some(ov) = state.overlays.get_mut(id) {
            ov.read_only = read_only;
        }
        Ok(Value::Nil)
    })
}

pub fn register(ns: &mut Namespace) {
    ns.intern("make-overlay", Value::Native(prim_make_overlay));
    ns.intern_private("make", Value::Native(prim_make_overlay));
    ns.intern("overlay-put-face", Value::Native(prim_overlay_put_face));
    ns.intern_private("put-face", Value::Native(prim_overlay_put_face));
    ns.intern(
        "overlay-put-property",
        Value::Native(prim_overlay_put_property),
    );
    ns.intern_private("put-property", Value::Native(prim_overlay_put_property));
    ns.intern("overlay-delete", Value::Native(prim_overlay_delete));
    ns.intern_private("delete", Value::Native(prim_overlay_delete));
    ns.intern("overlay-get", Value::Native(prim_overlay_get));
    ns.intern_private("get", Value::Native(prim_overlay_get));
    ns.intern("overlays-at", Value::Native(prim_overlays_at));
    ns.intern_private("at", Value::Native(prim_overlays_at));
    ns.intern(
        "overlay-put-invisible",
        Value::Native(prim_overlay_put_invisible),
    );
    ns.intern_private("put-invisible", Value::Native(prim_overlay_put_invisible));
    ns.intern(
        "overlay-put-read-only",
        Value::Native(prim_overlay_put_read_only),
    );
    ns.intern_private("put-read-only", Value::Native(prim_overlay_put_read_only));
}
