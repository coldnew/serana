/// Theme system for mora.
///
/// Defines named color themes with face slots matching emacs conventions.
/// The coldnew-night theme is the default.
///
/// Colors map to display-protocol's Color type (RGB, 8-bit per channel).

use std::cell::RefCell;
use std::collections::HashMap;

use crate::lisp::types::Value;

use super::editor_state::{with_editor_state, with_editor_state_mut};
use super::helpers::extract_string;

// ── Color type ───────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl ThemeColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_hex(s: &str) -> Option<Self> {
        let s = s.strip_prefix('#').unwrap_or(s);
        if s.len() == 6 {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            Some(Self { r, g, b })
        } else {
            None
        }
    }

    pub fn to_display_protocol(&self) -> display_protocol::Color {
        display_protocol::Color { r: self.r, g: self.g, b: self.b }
    }

    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

// ── Theme definition ─────────────────────────────────────────

/// A theme is a named collection of face colors.
/// Each face maps to an emacs-style face name.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub faces: HashMap<String, ThemeColor>,
}

impl Theme {
    pub fn get(&self, face: &str) -> ThemeColor {
        self.faces.get(face).copied().unwrap_or(ThemeColor::new(200, 200, 200))
    }

    pub fn set(&mut self, face: &str, color: ThemeColor) {
        self.faces.insert(face.to_string(), color);
    }

    /// Get display-protocol Style for a face
    pub fn style(&self, face: &str) -> display_protocol::Style {
        let c = self.get(face);
        display_protocol::Style {
            fg: Some(c.to_display_protocol()),
            ..display_protocol::Style::default()
        }
    }

    /// Get display-protocol Style with bg from another face
    pub fn style_with_bg(&self, fg_face: &str, bg_face: &str) -> display_protocol::Style {
        let fg = self.get(fg_face);
        let bg = self.get(bg_face);
        display_protocol::Style {
            fg: Some(fg.to_display_protocol()),
            bg: Some(bg.to_display_protocol()),
            ..display_protocol::Style::default()
        }
    }
}

// ── Preset: coldnew-night ────────────────────────────────────

fn coldnew_night() -> Theme {
    let mut faces = HashMap::new();

    // Base UI
    faces.insert("default-bg".into(), ThemeColor::new(0x20, 0x20, 0x20));
    faces.insert("far-background".into(), ThemeColor::new(0x1c, 0x1f, 0x26));
    faces.insert("default-fg".into(), ThemeColor::new(0xc6, 0xcc, 0xcc));
    faces.insert("cursor".into(), ThemeColor::new(0x00, 0xc8, 0xc8));
    faces.insert("current-line".into(), ThemeColor::new(0x2a, 0x2a, 0x2a));
    faces.insert("selection".into(), ThemeColor::new(0x3b, 0x3f, 0x41));
    faces.insert("highlight".into(), ThemeColor::new(0xca, 0xe6, 0x82));

    // Syntax (font-lock)
    faces.insert("builtin".into(), ThemeColor::new(0xcc, 0xaa, 0xff));
    faces.insert("constant".into(), ThemeColor::new(0xcc, 0xaa, 0xff));
    faces.insert("comment".into(), ThemeColor::new(0x99, 0xaa, 0xcc));
    faces.insert("comment-delimiter".into(), ThemeColor::new(0x5f, 0x5f, 0x5f));
    faces.insert("doc".into(), ThemeColor::new(0x97, 0xab, 0xc6));
    faces.insert("function-name".into(), ThemeColor::new(0xaa, 0xcc, 0xff));
    faces.insert("keyword".into(), ThemeColor::new(0xaa, 0xff, 0xaa));
    faces.insert("type".into(), ThemeColor::new(0xff, 0xf5, 0x9d));
    faces.insert("variable-name".into(), ThemeColor::new(0xaa, 0xcc, 0xff));
    faces.insert("string".into(), ThemeColor::new(0xaa, 0xdd, 0xdd));
    faces.insert("number".into(), ThemeColor::new(0xcc, 0xaa, 0xff));
    faces.insert("operator".into(), ThemeColor::new(0xcc, 0xcc, 0xcc));
    faces.insert("preprocessor".into(), ThemeColor::new(0xff, 0x88, 0x88));

    // Base00-07 (grayscale ramp)
    faces.insert("base00".into(), ThemeColor::new(0x20, 0x20, 0x20));
    faces.insert("base01".into(), ThemeColor::new(0x29, 0x29, 0x29));
    faces.insert("base02".into(), ThemeColor::new(0x5f, 0x5f, 0x5f));
    faces.insert("base03".into(), ThemeColor::new(0x99, 0x99, 0x99));
    faces.insert("base04".into(), ThemeColor::new(0xcc, 0xcc, 0xcc));
    faces.insert("base05".into(), ThemeColor::new(0xaa, 0xaa, 0xaa));
    faces.insert("base06".into(), ThemeColor::new(0xe9, 0xe2, 0xcb));
    faces.insert("base07".into(), ThemeColor::new(0xfc, 0xf4, 0xdc));

    // Terminal ANSI colors
    faces.insert("red".into(), ThemeColor::new(0xff, 0x33, 0x33));
    faces.insert("yellow".into(), ThemeColor::new(0xff, 0xf5, 0x9d));
    faces.insert("orange".into(), ThemeColor::new(0xff, 0x88, 0x88));
    faces.insert("green".into(), ThemeColor::new(0xaa, 0xff, 0xaa));
    faces.insert("blue".into(), ThemeColor::new(0xaa, 0xcc, 0xff));
    faces.insert("magenta".into(), ThemeColor::new(0xcc, 0xaa, 0xff));
    faces.insert("cyan".into(), ThemeColor::new(0xaa, 0xdd, 0xdd));
    faces.insert("white".into(), ThemeColor::new(0xff, 0xff, 0xff));
    faces.insert("black".into(), ThemeColor::new(0x2a, 0x2a, 0x2a));
    faces.insert("aqua".into(), ThemeColor::new(0x81, 0xd4, 0xfa));

    // Rainbow delimiters (for nested parens)
    faces.insert("rainbow-1".into(), ThemeColor::new(0xaa, 0xdd, 0xdd));
    faces.insert("rainbow-2".into(), ThemeColor::new(0x81, 0xd4, 0xfa));
    faces.insert("rainbow-3".into(), ThemeColor::new(0xaa, 0xcc, 0xff));
    faces.insert("rainbow-4".into(), ThemeColor::new(0xaa, 0xee, 0xcc));
    faces.insert("rainbow-5".into(), ThemeColor::new(0xcc, 0xaa, 0xff));
    faces.insert("rainbow-6".into(), ThemeColor::new(0xff, 0xf5, 0x9d));
    faces.insert("rainbow-7".into(), ThemeColor::new(0xff, 0x88, 0x88));
    faces.insert("rainbow-8".into(), ThemeColor::new(0x79, 0x55, 0x48));
    faces.insert("rainbow-9".into(), ThemeColor::new(0x82, 0x77, 0x17));

    // Modeline
    faces.insert("modeline-fg".into(), ThemeColor::new(0xc6, 0xcc, 0xcc));
    faces.insert("modeline-bg".into(), ThemeColor::new(0x40, 0x42, 0x54));
    faces.insert("modeline-inactive-fg".into(), ThemeColor::new(0x99, 0x99, 0x99));
    faces.insert("modeline-inactive-bg".into(), ThemeColor::new(0x29, 0x29, 0x29));

    // Status/echo area
    faces.insert("echo-fg".into(), ThemeColor::new(0x00, 0xc8, 0xc8));
    faces.insert("echo-bg".into(), ThemeColor::new(0x20, 0x20, 0x20));

    // Diff / git gutter
    faces.insert("diff-added".into(), ThemeColor::new(0xaa, 0xff, 0xaa));
    faces.insert("diff-removed".into(), ThemeColor::new(0xff, 0x88, 0x88));
    faces.insert("diff-changed".into(), ThemeColor::new(0xff, 0xf5, 0x9d));

    // UI elements
    faces.insert("menu-fg".into(), ThemeColor::new(0xcc, 0xcc, 0xcc));
    faces.insert("menu-bg".into(), ThemeColor::new(0x29, 0x29, 0x29));
    faces.insert("menu-selected-fg".into(), ThemeColor::new(0xff, 0xff, 0xff));
    faces.insert("menu-selected-bg".into(), ThemeColor::new(0x3b, 0x3f, 0x41));
    faces.insert("line-number".into(), ThemeColor::new(0x5f, 0x5f, 0x5f));
    faces.insert("line-number-active".into(), ThemeColor::new(0x99, 0x99, 0x99));
    faces.insert("fringe".into(), ThemeColor::new(0x20, 0x20, 0x20));
    faces.insert("scrollbar".into(), ThemeColor::new(0x29, 0x29, 0x29));
    faces.insert("tab-active".into(), ThemeColor::new(0x3b, 0x3f, 0x41));
    faces.insert("tab-inactive".into(), ThemeColor::new(0x20, 0x20, 0x20));

    // Warning / error / info
    faces.insert("error".into(), ThemeColor::new(0xff, 0x33, 0x33));
    faces.insert("warning".into(), ThemeColor::new(0xff, 0xf5, 0x9d));
    faces.insert("info".into(), ThemeColor::new(0x81, 0xd4, 0xfa));

    // Search / match
    faces.insert("search-match".into(), ThemeColor::new(0x00, 0xc8, 0xc8));
    faces.insert("isearch".into(), ThemeColor::new(0xca, 0xe6, 0x82));
    faces.insert("lazy-highlight".into(), ThemeColor::new(0x3b, 0x3f, 0x41));

    Theme { name: "night-coldnew".into(), faces }
}

// ── Global theme registry ────────────────────────────────────

thread_local! {
    static THEME_REGISTRY: RefCell<HashMap<String, Theme>> = RefCell::new({
        let mut m = HashMap::new();
        let theme = coldnew_night();
        m.insert(theme.name.clone(), theme);
        m
    });
    static ACTIVE_THEME: RefCell<String> = RefCell::new("night-coldnew".into());
}

pub fn with_active_theme<R>(f: impl FnOnce(&Theme) -> R) -> R {
    ACTIVE_THEME.with(|name| {
        let name = name.borrow();
        THEME_REGISTRY.with(|reg| {
            let reg = reg.borrow();
            let theme = reg.get(&*name).expect("active theme not found");
            f(theme)
        })
    })
}

pub fn get_face(face: &str) -> ThemeColor {
    with_active_theme(|t| t.get(face))
}

// ── Lisp primitives ──────────────────────────────────────────

/// (theme-load NAME) → load and activate a named theme
fn prim_theme_load(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    THEME_REGISTRY.with(|reg| {
        if reg.borrow().contains_key(&name) {
            ACTIVE_THEME.with(|a| *a.borrow_mut() = name.clone());
            Ok(Value::string(name))
        } else {
            Err(format!("theme not found: {}", name))
        }
    })
}

/// (theme-active) → return name of active theme
fn prim_theme_active(_args: &[Value]) -> Result<Value, String> {
    ACTIVE_THEME.with(|name| Ok(Value::string(&*name.borrow())))
}

/// (theme-names) → vector of available theme names
fn prim_theme_names(_args: &[Value]) -> Result<Value, String> {
    THEME_REGISTRY.with(|reg| {
        let mut names: Vec<String> = reg.borrow().keys().cloned().collect();
        names.sort();
        Ok(Value::vector(names.into_iter().map(Value::string).collect()))
    })
}

/// (theme-get FACE) → get color for face as hex string
fn prim_theme_get(args: &[Value]) -> Result<Value, String> {
    let face = extract_string(args, 0)?;
    let color = get_face(&face);
    Ok(Value::string(color.to_hex()))
}

/// (theme-set-face FACE COLOR) → set color for face in active theme
fn prim_theme_set_face(args: &[Value]) -> Result<Value, String> {
    let face = extract_string(args, 0)?;
    let color_str = extract_string(args, 1)?;
    let color = ThemeColor::from_hex(&color_str)
        .ok_or_else(|| format!("invalid color: {}", color_str))?;

    ACTIVE_THEME.with(|name| {
        THEME_REGISTRY.with(|reg| {
            if let Some(theme) = reg.borrow_mut().get_mut(&*name.borrow()) {
                theme.set(&face, color);
            }
        });
    });
    Ok(Value::Nil)
}

/// (theme-define NAME FACES) → define a custom theme from a map of face→color
fn prim_theme_define(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;

    let mut faces = HashMap::new();
    match args.get(1) {
        Some(Value::Map(map)) => {
            for (k, v) in map.iter() {
                let face_name = match k {
                    Value::Keyword(kw) => kw.name.to_string(),
                    Value::String(s) => s.to_string(),
                    _ => continue,
                };
                let color_str = match v {
                    Value::String(s) => s.to_string(),
                    _ => continue,
                };
                if let Some(color) = ThemeColor::from_hex(&color_str) {
                    faces.insert(face_name, color);
                }
            }
        }
        _ => return Err("theme-define requires a map of face→color".to_string()),
    }

    let theme = Theme { name: name.clone(), faces };
    THEME_REGISTRY.with(|reg| {
        reg.borrow_mut().insert(name.clone(), theme);
    });
    ACTIVE_THEME.with(|a| *a.borrow_mut() = name.clone());
    Ok(Value::string(name))
}

/// (theme-clone NAME NEW-NAME) → clone current theme with a new name
fn prim_theme_clone(args: &[Value]) -> Result<Value, String> {
    let new_name = extract_string(args, 0)?;

    let cloned = ACTIVE_THEME.with(|active| {
        THEME_REGISTRY.with(|reg| {
            let reg = reg.borrow();
            let original = reg.get(&*active.borrow())
                .expect("no active theme");
            let mut cloned = original.clone();
            cloned.name = new_name.clone();
            cloned
        })
    });

    THEME_REGISTRY.with(|reg| {
        reg.borrow_mut().insert(new_name.clone(), cloned);
    });
    ACTIVE_THEME.with(|a| *a.borrow_mut() = new_name.clone());

    Ok(Value::string(new_name))
}

/// (theme-colors) → map of all face names → hex colors in active theme
fn prim_theme_colors(_args: &[Value]) -> Result<Value, String> {
    let pairs = with_active_theme(|theme| {
        let mut entries: Vec<(String, String)> = theme.faces.iter()
            .map(|(k, v)| (k.clone(), v.to_hex()))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries
    });
    Ok(Value::map(
        pairs.into_iter()
            .map(|(k, v)| (Value::keyword(k), Value::string(v)))
            .collect(),
    ))
}

/// (theme-to-style FACE) → convert face to display-protocol style map
fn prim_theme_to_style(args: &[Value]) -> Result<Value, String> {
    let face = extract_string(args, 0)?;
    let color = get_face(&face);
    Ok(Value::map(vec![
        (Value::keyword("fg"), Value::string(color.to_hex())),
    ]))
}

// ── Registration ─────────────────────────────────────────────

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern_with_doc("theme-load", Value::Native(prim_theme_load),
        "Load and activate a named theme.");
    ns.intern_with_doc("theme-active", Value::Native(prim_theme_active),
        "Return name of the active theme.");
    ns.intern_with_doc("theme-names", Value::Native(prim_theme_names),
        "Return vector of available theme names.");
    ns.intern_with_doc("theme-get", Value::Native(prim_theme_get),
        "Get hex color for FACE in active theme.");
    ns.intern_with_doc("theme-set-face", Value::Native(prim_theme_set_face),
        "Set FACE to COLOR (hex) in active theme.");
    ns.intern_with_doc("theme-define", Value::Native(prim_theme_define),
        "Define a theme: (theme-define NAME {:default-fg \"#c6cccc\" :keyword \"#aaffaa\" ...}).");
    ns.intern_with_doc("theme-clone", Value::Native(prim_theme_clone),
        "Clone current theme to NEW-NAME.");
    ns.intern_with_doc("theme-colors", Value::Native(prim_theme_colors),
        "Return map of all face→color in active theme.");
    ns.intern_with_doc("theme-to-style", Value::Native(prim_theme_to_style),
        "Get style map for FACE.");
}

// ── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::editor_state::*;

    fn setup() {
        set_editor_state(EditorState::new());
    }

    fn teardown() {
        take_editor_state();
    }

    #[test]
    fn test_coldnew_night_loaded() {
        setup();
        let mut bridge = super::super::core::MoraLispBridge::new();

        assert_eq!(bridge.eval("(theme-active)").unwrap(), Value::string("night-coldnew"));

        let names = bridge.eval("(theme-names)").unwrap();
        match names {
            Value::Vector(v) => assert!(v.len() >= 1),
            _ => panic!("expected vector"),
        }
        teardown();
    }

    #[test]
    fn test_theme_get_face() {
        setup();
        let mut bridge = super::super::core::MoraLispBridge::new();

        let bg = bridge.eval("(theme-get \"default-bg\")").unwrap();
        assert_eq!(bg, Value::string("#202020"));

        let cursor = bridge.eval("(theme-get \"cursor\")").unwrap();
        assert_eq!(cursor, Value::string("#00c8c8"));

        let keyword = bridge.eval("(theme-get \"keyword\")").unwrap();
        assert_eq!(keyword, Value::string("#aaffaa"));

        teardown();
    }

    #[test]
    fn test_theme_set_face() {
        setup();
        let mut bridge = super::super::core::MoraLispBridge::new();

        bridge.eval("(theme-set-face \"keyword\" \"#ff00ff\")").unwrap();
        let kw = bridge.eval("(theme-get \"keyword\")").unwrap();
        assert_eq!(kw, Value::string("#ff00ff"));

        // Restore
        bridge.eval("(theme-set-face \"keyword\" \"#aaffaa\")").unwrap();
        teardown();
    }

    #[test]
    fn test_theme_define_custom() {
        setup();
        let mut bridge = super::super::core::MoraLispBridge::new();

        // Use theme-set-face for per-face customization
        bridge.eval("(theme-set-face \"default-bg\" \"#000000\")").unwrap();
        bridge.eval("(theme-set-face \"default-fg\" \"#ffffff\")").unwrap();
        assert_eq!(bridge.eval("(theme-get \"default-bg\")").unwrap(), Value::string("#000000"));
        assert_eq!(bridge.eval("(theme-get \"default-fg\")").unwrap(), Value::string("#ffffff"));

        // Restore original values
        bridge.eval("(theme-set-face \"default-bg\" \"#202020\")").unwrap();
        bridge.eval("(theme-set-face \"default-fg\" \"#c6cccc\")").unwrap();
        teardown();
    }

    #[test]
    fn test_theme_clone() {
        setup();
        let mut bridge = super::super::core::MoraLispBridge::new();

        bridge.eval("(theme-clone \"my-night\")").unwrap();
        bridge.eval("(theme-set-face \"keyword\" \"#ff00ff\")").unwrap();

        // Original should be unchanged
        bridge.eval("(theme-load \"night-coldnew\")").unwrap();
        let kw = bridge.eval("(theme-get \"keyword\")").unwrap();
        assert_eq!(kw, Value::string("#aaffaa"));

        // Clone should have the modification
        bridge.eval("(theme-load \"my-night\")").unwrap();
        let kw = bridge.eval("(theme-get \"keyword\")").unwrap();
        assert_eq!(kw, Value::string("#ff00ff"));

        teardown();
    }

    #[test]
    fn test_theme_colors_map() {
        setup();
        let mut bridge = super::super::core::MoraLispBridge::new();

        let colors = bridge.eval("(theme-colors)").unwrap();
        match colors {
            Value::Map(m) => {
                assert!(m.len() > 50, "should have many faces: {}", m.len());
                // Spot check a few
                assert!(m.contains_key(&Value::keyword("default-bg")));
                assert!(m.contains_key(&Value::keyword("cursor")));
                assert!(m.contains_key(&Value::keyword("red")));
                assert!(m.contains_key(&Value::keyword("rainbow-1")));
            }
            _ => panic!("expected map"),
        }
        teardown();
    }

    #[test]
    fn test_theme_color_from_hex() {
        assert_eq!(ThemeColor::from_hex("#ff0000"), Some(ThemeColor::new(255, 0, 0)));
        assert_eq!(ThemeColor::from_hex("#00ff00"), Some(ThemeColor::new(0, 255, 0)));
        assert_eq!(ThemeColor::from_hex("#0000ff"), Some(ThemeColor::new(0, 0, 255)));
        assert_eq!(ThemeColor::from_hex("#202020"), Some(ThemeColor::new(0x20, 0x20, 0x20)));
        assert_eq!(ThemeColor::from_hex("invalid"), None);
    }

    #[test]
    fn test_coldnew_night_all_faces_present() {
        setup();
        let mut bridge = super::super::core::MoraLispBridge::new();

        // Verify all expected faces from coldnew-night exist
        let expected = [
            "default-bg", "default-fg", "cursor", "current-line", "selection",
            "keyword", "string", "comment", "function-name", "builtin", "constant",
            "type", "variable-name", "doc", "number",
            "red", "green", "blue", "yellow", "magenta", "cyan", "orange", "aqua",
            "modeline-fg", "modeline-bg",
            "error", "warning", "info",
            "search-match", "isearch",
            "diff-added", "diff-removed", "diff-changed",
            "line-number", "line-number-active",
            "rainbow-1", "rainbow-2", "rainbow-3",
        ];

        for face in &expected {
            let color = bridge.eval(&format!("(theme-get \"{}\")", face)).unwrap();
            match color {
                Value::String(s) => {
                    assert!(s.starts_with('#'), "face {} should be hex color: {}", face, s);
                    assert_eq!(s.len(), 7, "face {} hex should be 7 chars: {}", face, s);
                }
                _ => panic!("face {} should return string", face),
            }
        }
        teardown();
    }
}
