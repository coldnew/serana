use super::display::event::MoraKeyEvent;
use super::keymap::KeyAction;

mod aggressive_indent;
mod auto_fill;
mod auto_indent;
mod auto_save;
mod electric_pair;
mod highlight_trailing;
mod line_numbers;
mod read_only;
mod relative_line_numbers;
mod whitespace;

pub trait MinorMode: std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn modeline_indicator(&self) -> &'static str;
    fn on_key(&self, _key: MoraKeyEvent) -> Option<KeyAction> {
        None
    }
    fn on_enable(&self) {}
    fn on_disable(&self) {}
    fn priority(&self) -> i32 {
        0
    }
    fn on_insert_char(&self, _ch: char) -> Option<KeyAction> {
        None
    }
    fn on_insert_newline(&self) -> Option<KeyAction> {
        None
    }
}

#[derive(Debug)]
pub struct MinorModeRegistry {
    modes: Vec<Box<dyn MinorMode>>,
}

impl MinorModeRegistry {
    pub fn new() -> Self {
        Self { modes: Vec::new() }
    }

    pub fn register(&mut self, mode: Box<dyn MinorMode>) {
        mode.on_enable();
        self.modes.push(mode);
        self.modes.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    pub fn enable_by_name(&mut self, name: &str) -> bool {
        if self.is_enabled(name) {
            return false;
        }
        if let Some(mode) = create_minor_mode(name) {
            self.register(mode);
            true
        } else {
            false
        }
    }

    pub fn disable_by_name(&mut self, name: &str) -> bool {
        if let Some(pos) = self.modes.iter().position(|m| m.name() == name) {
            self.modes[pos].on_disable();
            self.modes.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn toggle_by_name(&mut self, name: &str) -> bool {
        if self.is_enabled(name) {
            self.disable_by_name(name);
            false
        } else {
            self.enable_by_name(name)
        }
    }

    pub fn is_enabled(&self, name: &str) -> bool {
        self.modes.iter().any(|m| m.name() == name)
    }

    pub fn intercept_key(&self, key: MoraKeyEvent) -> Option<KeyAction> {
        for mode in &self.modes {
            if let Some(action) = mode.on_key(key) {
                return Some(action);
            }
        }
        None
    }

    pub fn on_insert_char(&self, ch: char) -> Option<KeyAction> {
        for mode in &self.modes {
            if let Some(action) = mode.on_insert_char(ch) {
                return Some(action);
            }
        }
        None
    }

    pub fn on_insert_newline(&self) -> Option<KeyAction> {
        for mode in &self.modes {
            if let Some(action) = mode.on_insert_newline() {
                return Some(action);
            }
        }
        None
    }

    pub fn modeline_string(&self) -> String {
        let indicators: Vec<&str> = self
            .modes
            .iter()
            .map(|m| m.modeline_indicator())
            .filter(|s| !s.is_empty())
            .collect();
        if indicators.is_empty() {
            String::new()
        } else {
            format!(" {} ", indicators.join(" "))
        }
    }

    pub fn enabled_names(&self) -> Vec<&'static str> {
        self.modes.iter().map(|m| m.name()).collect()
    }

    pub fn count(&self) -> usize {
        self.modes.len()
    }
}

pub fn all_minor_mode_names() -> Vec<&'static str> {
    vec![
        "line-numbers",
        "relative-line-numbers",
        "auto-fill",
        "whitespace",
        "highlight-trailing",
        "auto-save",
        "read-only",
        "electric-pair",
        "aggressive-indent",
        "auto-indent",
    ]
}

pub fn create_minor_mode(name: &str) -> Option<Box<dyn MinorMode>> {
    match name.to_lowercase().as_str() {
        "line-numbers" | "line-numbers-mode" => Some(Box::new(line_numbers::LineNumbersMode)),
        "relative-line-numbers" | "relative-line-numbers-mode" => {
            Some(Box::new(relative_line_numbers::RelativeLineNumbersMode))
        }
        "auto-fill" | "auto-fill-mode" => Some(Box::new(auto_fill::AutoFillMode)),
        "whitespace" | "whitespace-mode" => Some(Box::new(whitespace::WhitespaceMode)),
        "highlight-trailing" | "highlight-trailing-whitespace" => {
            Some(Box::new(highlight_trailing::HighlightTrailingMode))
        }
        "auto-save" | "auto-save-mode" => Some(Box::new(auto_save::AutoSaveMode)),
        "read-only" | "read-only-mode" => Some(Box::new(read_only::ReadOnlyMode)),
        "electric-pair" | "electric-pair-mode" => Some(Box::new(electric_pair::ElectricPairMode)),
        "aggressive-indent" | "aggressive-indent-mode" => {
            Some(Box::new(aggressive_indent::AggressiveIndentMode))
        }
        "auto-indent" | "auto-indent-mode" => Some(Box::new(auto_indent::AutoIndentMode)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::display::event::{MoraKeyCode, MoraKeyModifiers};
    use super::*;

    #[test]
    fn test_registry_create() {
        let registry = MinorModeRegistry::new();
        assert_eq!(registry.count(), 0);
        assert!(registry.modeline_string().is_empty());
    }

    #[test]
    fn test_enable_disable() {
        let mut registry = MinorModeRegistry::new();
        assert!(registry.enable_by_name("line-numbers"));
        assert_eq!(registry.count(), 1);
        assert!(registry.is_enabled("line-numbers"));
        assert!(!registry.is_enabled("auto-fill"));

        assert!(registry.disable_by_name("line-numbers"));
        assert_eq!(registry.count(), 0);
        assert!(!registry.is_enabled("line-numbers"));
    }

    #[test]
    fn test_toggle() {
        let mut registry = MinorModeRegistry::new();
        assert!(registry.toggle_by_name("line-numbers"));
        assert!(registry.is_enabled("line-numbers"));
        assert!(!registry.toggle_by_name("line-numbers"));
        assert!(!registry.is_enabled("line-numbers"));
    }

    #[test]
    fn test_enable_same_twice() {
        let mut registry = MinorModeRegistry::new();
        assert!(registry.enable_by_name("line-numbers"));
        assert!(!registry.enable_by_name("line-numbers"));
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_enable_unknown() {
        let mut registry = MinorModeRegistry::new();
        assert!(!registry.enable_by_name("nonexistent"));
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_modeline_string() {
        let mut registry = MinorModeRegistry::new();
        registry.enable_by_name("line-numbers");
        registry.enable_by_name("auto-fill");
        let s = registry.modeline_string();
        assert!(s.contains("LN"));
        assert!(s.contains("Fill"));
    }

    #[test]
    fn test_enabled_names() {
        let mut registry = MinorModeRegistry::new();
        registry.enable_by_name("whitespace");
        registry.enable_by_name("auto-save");
        let names = registry.enabled_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"whitespace"));
        assert!(names.contains(&"auto-save"));
    }

    #[test]
    fn test_priority_order() {
        let mut registry = MinorModeRegistry::new();
        registry.enable_by_name("line-numbers");
        registry.enable_by_name("read-only");
        registry.enable_by_name("auto-fill");
        let names = registry.enabled_names();
        assert_eq!(names[0], "read-only");
        assert_eq!(names[1], "line-numbers");
        assert_eq!(names[2], "auto-fill");
    }

    #[test]
    fn test_read_only_intercept() {
        let mode = read_only::ReadOnlyMode;
        let insert_key = MoraKeyEvent::new(MoraKeyCode::Char('a'), MoraKeyModifiers::NONE);
        assert!(mode.on_key(insert_key).is_some());
        let esc_key = MoraKeyEvent::new(MoraKeyCode::Esc, MoraKeyModifiers::NONE);
        assert!(mode.on_key(esc_key).is_none());
        let ctrl_c = MoraKeyEvent::new(MoraKeyCode::Char('c'), MoraKeyModifiers::CTRL);
        assert!(mode.on_key(ctrl_c).is_none());
    }

    #[test]
    fn test_all_names() {
        let names = all_minor_mode_names();
        assert!(names.len() >= 10);
        assert!(names.contains(&"line-numbers"));
        assert!(names.contains(&"read-only"));
        assert!(names.contains(&"electric-pair"));
        assert!(names.contains(&"aggressive-indent"));
        assert!(names.contains(&"auto-indent"));
    }

    #[test]
    fn test_create_minor_mode() {
        assert!(create_minor_mode("line-numbers").is_some());
        assert!(create_minor_mode("line-numbers-mode").is_some());
        assert!(create_minor_mode("read-only").is_some());
        assert!(create_minor_mode("electric-pair").is_some());
        assert!(create_minor_mode("electric-pair-mode").is_some());
        assert!(create_minor_mode("aggressive-indent").is_some());
        assert!(create_minor_mode("auto-indent").is_some());
        assert!(create_minor_mode("nonexistent").is_none());
    }

    #[test]
    fn test_electric_pair_brackets() {
        let mode = electric_pair::ElectricPairMode;
        assert_eq!(mode.on_insert_char('('), Some(KeyAction::InsertChar(')')));
        assert_eq!(mode.on_insert_char('['), Some(KeyAction::InsertChar(']')));
        assert_eq!(mode.on_insert_char('{'), Some(KeyAction::InsertChar('}')));
        assert_eq!(mode.on_insert_char('"'), Some(KeyAction::InsertChar('"')));
        assert_eq!(mode.on_insert_char('\''), Some(KeyAction::InsertChar('\'')));
        assert_eq!(mode.on_insert_char('a'), None);
        assert_eq!(mode.modeline_indicator(), "EP");
    }

    #[test]
    fn test_aggressive_indent_hooks() {
        let mode = aggressive_indent::AggressiveIndentMode;
        assert_eq!(mode.on_insert_newline(), Some(KeyAction::IndentLine));
        assert_eq!(mode.modeline_indicator(), "AI");
    }

    #[test]
    fn test_auto_indent_hooks() {
        let mode = auto_indent::AutoIndentMode;
        assert_eq!(mode.on_insert_newline(), Some(KeyAction::IndentLine));
        assert_eq!(mode.modeline_indicator(), "Ind");
    }

    #[test]
    fn test_registry_insert_char_hooks() {
        let mut registry = MinorModeRegistry::new();
        registry.enable_by_name("electric-pair");
        assert_eq!(
            registry.on_insert_char('('),
            Some(KeyAction::InsertChar(')'))
        );
        assert_eq!(registry.on_insert_char('a'), None);
    }

    #[test]
    fn test_registry_insert_newline_hooks() {
        let mut registry = MinorModeRegistry::new();
        registry.enable_by_name("auto-indent");
        assert_eq!(registry.on_insert_newline(), Some(KeyAction::IndentLine));

        let registry2 = MinorModeRegistry::new();
        assert_eq!(registry2.on_insert_newline(), None);
    }
}
