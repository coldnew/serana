//! Backend-neutral widget specifications.
//!
//! These structs describe the shared widget state that backend crates render.
//! `display-tui` owns the terminal rendering, `display-wgpu` can render the same
//! specs graphically, and core crates can build specs without depending on a
//! concrete backend.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WidgetSpec {
    Text(TextSpec),
    Box(BoxSpec),
    Spacer(SpacerSpec),
    TruncatedText(TruncatedTextSpec),
    Input(InputSpec),
    Loader(LoaderSpec),
    CancellableLoader(CancellableLoaderSpec),
    SelectList(SelectListSpec),
    SettingsList(SettingsListSpec),
    Markdown(MarkdownSpec),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextSpec {
    pub text: String,
    pub padding_x: u16,
    pub padding_y: u16,
}

impl TextSpec {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            padding_x: 1,
            padding_y: 1,
        }
    }

    pub fn padding(mut self, x: u16, y: u16) -> Self {
        self.padding_x = x;
        self.padding_y = y;
        self
    }
}

impl From<TextSpec> for WidgetSpec {
    fn from(spec: TextSpec) -> Self {
        Self::Text(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoxSpec {
    pub children: Vec<WidgetSpec>,
    pub padding_x: u16,
    pub padding_y: u16,
}

impl BoxSpec {
    pub fn new(children: Vec<WidgetSpec>) -> Self {
        Self {
            children,
            padding_x: 1,
            padding_y: 1,
        }
    }

    pub fn padding(mut self, x: u16, y: u16) -> Self {
        self.padding_x = x;
        self.padding_y = y;
        self
    }
}

impl From<BoxSpec> for WidgetSpec {
    fn from(spec: BoxSpec) -> Self {
        Self::Box(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpacerSpec {
    pub lines: u16,
}

impl SpacerSpec {
    pub fn new(lines: u16) -> Self {
        Self { lines }
    }
}

impl From<SpacerSpec> for WidgetSpec {
    fn from(spec: SpacerSpec) -> Self {
        Self::Spacer(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TruncatedTextSpec {
    pub text: String,
    pub padding_x: u16,
    pub padding_y: u16,
}

impl TruncatedTextSpec {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            padding_x: 0,
            padding_y: 0,
        }
    }

    pub fn padding(mut self, x: u16, y: u16) -> Self {
        self.padding_x = x;
        self.padding_y = y;
        self
    }
}

impl From<TruncatedTextSpec> for WidgetSpec {
    fn from(spec: TruncatedTextSpec) -> Self {
        Self::TruncatedText(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputSpec {
    pub value: String,
    pub cursor: usize,
    pub focused: bool,
    pub prompt: String,
}

impl InputSpec {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        Self {
            cursor: value.len(),
            value,
            focused: false,
            prompt: "> ".to_string(),
        }
    }

    pub fn cursor(mut self, cursor: usize) -> Self {
        self.cursor = cursor.min(self.value.len());
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }
}

impl From<InputSpec> for WidgetSpec {
    fn from(spec: InputSpec) -> Self {
        Self::Input(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoaderSpec {
    pub message: String,
    pub frame: usize,
    pub frames: Vec<String>,
}

impl LoaderSpec {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            frame: 0,
            frames: default_spinner_frames(),
        }
    }

    pub fn frame(mut self, frame: usize) -> Self {
        self.frame = frame;
        self
    }

    pub fn frames(mut self, frames: Vec<String>) -> Self {
        self.frames = frames;
        self
    }
}

impl From<LoaderSpec> for WidgetSpec {
    fn from(spec: LoaderSpec) -> Self {
        Self::Loader(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancellableLoaderSpec {
    pub loader: LoaderSpec,
    pub cancel_hint: String,
}

impl CancellableLoaderSpec {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            loader: LoaderSpec::new(message),
            cancel_hint: "Esc to cancel".to_string(),
        }
    }

    pub fn frame(mut self, frame: usize) -> Self {
        self.loader.frame = frame;
        self
    }

    pub fn cancel_hint(mut self, hint: impl Into<String>) -> Self {
        self.cancel_hint = hint.into();
        self
    }
}

impl From<CancellableLoaderSpec> for WidgetSpec {
    fn from(spec: CancellableLoaderSpec) -> Self {
        Self::CancellableLoader(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectItemSpec {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

impl SelectItemSpec {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            description: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectListSpec {
    pub items: Vec<SelectItemSpec>,
    pub selected: usize,
    pub max_visible: usize,
    pub min_primary_column_width: usize,
    pub max_primary_column_width: usize,
}

impl SelectListSpec {
    pub fn new(items: Vec<SelectItemSpec>) -> Self {
        Self {
            items,
            selected: 0,
            max_visible: 5,
            min_primary_column_width: 32,
            max_primary_column_width: 32,
        }
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }

    pub fn max_visible(mut self, max_visible: usize) -> Self {
        self.max_visible = max_visible.max(1);
        self
    }
}

impl From<SelectListSpec> for WidgetSpec {
    fn from(spec: SelectListSpec) -> Self {
        Self::SelectList(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingItemSpec {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub current_value: String,
    pub values: Vec<String>,
}

impl SettingItemSpec {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        current_value: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: None,
            current_value: current_value.into(),
            values: Vec::new(),
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn values(mut self, values: Vec<String>) -> Self {
        self.values = values;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsListSpec {
    pub items: Vec<SettingItemSpec>,
    pub selected: usize,
    pub max_visible: usize,
    pub show_hint: bool,
    pub hint: String,
}

impl SettingsListSpec {
    pub fn new(items: Vec<SettingItemSpec>) -> Self {
        Self {
            items,
            selected: 0,
            max_visible: 8,
            show_hint: true,
            hint: "Enter/Space to change · Esc to cancel".to_string(),
        }
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }

    pub fn max_visible(mut self, max_visible: usize) -> Self {
        self.max_visible = max_visible.max(1);
        self
    }
}

impl From<SettingsListSpec> for WidgetSpec {
    fn from(spec: SettingsListSpec) -> Self {
        Self::SettingsList(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownSpec {
    pub text: String,
    pub padding_x: u16,
    pub padding_y: u16,
}

impl MarkdownSpec {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            padding_x: 1,
            padding_y: 0,
        }
    }

    pub fn padding(mut self, x: u16, y: u16) -> Self {
        self.padding_x = x;
        self.padding_y = y;
        self
    }
}

impl From<MarkdownSpec> for WidgetSpec {
    fn from(spec: MarkdownSpec) -> Self {
        Self::Markdown(spec)
    }
}

fn default_spinner_frames() -> Vec<String> {
    ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
        .into_iter()
        .map(String::from)
        .collect()
}
