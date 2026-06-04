//! Pi-style shared components built as backend-agnostic `UiNode` trees.

use crate::palette;
use crate::{Editor, TextInput};
use display_protocol::{
    Align, Border, BoxNode, CanvasNode, Color, Justify, Padding, Style, TextNode, UiNode, Wrap,
};

pub use crate::{Editor as EditorComponent, TextInput as Input};

#[derive(Debug, Clone)]
pub struct BoxComponent {
    children: Vec<UiNode>,
    style: Style,
    padding: Padding,
    border: Border,
    title: Option<String>,
    width: Option<u16>,
    height: Option<u16>,
}

impl BoxComponent {
    pub fn new(children: Vec<UiNode>) -> Self {
        Self {
            children,
            style: Style::default(),
            padding: Padding::ZERO,
            border: Border::NONE,
            title: None,
            width: None,
            height: None,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    pub fn border(mut self, border: Border) -> Self {
        self.border = border;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: u16) -> Self {
        self.height = Some(height);
        self
    }

    pub fn build(self) -> UiNode {
        UiNode::Box(BoxNode {
            children: self.children,
            style: self.style,
            padding: self.padding,
            border: self.border,
            title: self.title,
            width: self.width,
            height: self.height,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
        })
    }
}

impl From<BoxComponent> for UiNode {
    fn from(component: BoxComponent) -> Self {
        component.build()
    }
}

#[derive(Debug, Clone)]
pub struct Text {
    content: String,
    style: Style,
    wrap: Wrap,
}

impl Text {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: Style::default(),
            wrap: Wrap::Wrap,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.style = self.style.fg(color);
        self
    }

    pub fn bold(mut self) -> Self {
        self.style = self.style.bold();
        self
    }

    pub fn dim(mut self) -> Self {
        self.style = self.style.dim();
        self
    }

    pub fn no_wrap(mut self) -> Self {
        self.wrap = Wrap::NoWrap;
        self
    }

    pub fn truncate(mut self) -> Self {
        self.wrap = Wrap::Truncate;
        self
    }

    pub fn build(self) -> UiNode {
        UiNode::Text(TextNode {
            content: self.content,
            style: self.style,
            wrap: self.wrap,
        })
    }
}

impl From<Text> for UiNode {
    fn from(text: Text) -> Self {
        text.build()
    }
}

#[derive(Debug, Clone)]
pub struct Spacer {
    width: Option<u16>,
    height: Option<u16>,
}

impl Spacer {
    pub fn new() -> Self {
        Self {
            width: None,
            height: None,
        }
    }

    pub fn width(width: u16) -> Self {
        Self {
            width: Some(width),
            height: None,
        }
    }

    pub fn height(height: u16) -> Self {
        Self {
            width: None,
            height: Some(height),
        }
    }

    pub fn build(self) -> UiNode {
        BoxComponent::new(Vec::new())
            .width_opt(self.width)
            .height_opt(self.height)
            .build()
    }
}

impl Default for Spacer {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Spacer> for UiNode {
    fn from(spacer: Spacer) -> Self {
        spacer.build()
    }
}

trait BoxComponentOptions {
    fn width_opt(self, width: Option<u16>) -> Self;
    fn height_opt(self, height: Option<u16>) -> Self;
}

impl BoxComponentOptions for BoxComponent {
    fn width_opt(mut self, width: Option<u16>) -> Self {
        self.width = width;
        self
    }

    fn height_opt(mut self, height: Option<u16>) -> Self {
        self.height = height;
        self
    }
}

#[derive(Debug, Clone)]
pub struct TruncatedText {
    content: String,
    max_chars: usize,
    suffix: String,
}

impl TruncatedText {
    pub fn new(content: impl Into<String>, max_chars: usize) -> Self {
        Self {
            content: content.into(),
            max_chars,
            suffix: String::new(),
        }
    }

    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }

    pub fn build(self) -> UiNode {
        UiNode::text(truncate_chars(&self.content, self.max_chars, &self.suffix)).wrap(Wrap::NoWrap)
    }
}

impl From<TruncatedText> for UiNode {
    fn from(text: TruncatedText) -> Self {
        text.build()
    }
}

#[derive(Debug, Clone)]
pub struct Loader {
    label: Option<String>,
    frame: usize,
}

impl Loader {
    pub fn new(frame: usize) -> Self {
        Self { label: None, frame }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn build(self) -> UiNode {
        let frames = ["|", "/", "-", "\\"];
        let spinner = UiNode::text(frames[self.frame % frames.len()]).color(palette::PRIMARY);
        match self.label {
            Some(label) => {
                UiNode::row(vec![spinner, UiNode::text(label).color(palette::MUTED)]).gap(1)
            }
            None => spinner,
        }
    }
}

impl From<Loader> for UiNode {
    fn from(loader: Loader) -> Self {
        loader.build()
    }
}

#[derive(Debug, Clone)]
pub struct CancellableLoader {
    label: String,
    cancel_hint: String,
    frame: usize,
}

impl CancellableLoader {
    pub fn new(label: impl Into<String>, frame: usize) -> Self {
        Self {
            label: label.into(),
            cancel_hint: "Esc to cancel".to_string(),
            frame,
        }
    }

    pub fn cancel_hint(mut self, hint: impl Into<String>) -> Self {
        self.cancel_hint = hint.into();
        self
    }

    pub fn build(self) -> UiNode {
        UiNode::row(vec![
            Loader::new(self.frame).label(self.label).build(),
            UiNode::text(self.cancel_hint).color(palette::MUTED).dim(),
        ])
        .gap(2)
    }
}

impl From<CancellableLoader> for UiNode {
    fn from(loader: CancellableLoader) -> Self {
        loader.build()
    }
}

#[derive(Debug, Clone)]
pub struct SelectItem {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

impl SelectItem {
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

#[derive(Debug, Clone)]
pub struct SelectList {
    items: Vec<SelectItem>,
    selected: usize,
    max_visible: usize,
}

impl SelectList {
    pub fn new(items: Vec<SelectItem>) -> Self {
        Self {
            items,
            selected: 0,
            max_visible: 8,
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

    pub fn build(self) -> UiNode {
        if self.items.is_empty() {
            return UiNode::text("No matching items").color(palette::MUTED);
        }

        let selected = self.selected.min(self.items.len().saturating_sub(1));
        let half = self.max_visible / 2;
        let start = selected
            .saturating_sub(half)
            .min(self.items.len().saturating_sub(self.max_visible));
        let end = (start + self.max_visible).min(self.items.len());

        let mut rows = Vec::new();
        for (index, item) in self.items[start..end].iter().enumerate() {
            let absolute = start + index;
            let prefix = if absolute == selected { "> " } else { "  " };
            let mut children = vec![UiNode::text(format!("{prefix}{}", item.label))];
            if let Some(description) = &item.description {
                children.push(UiNode::text(description).color(palette::MUTED));
            }
            let mut row = UiNode::row(children).gap(2);
            if absolute == selected {
                row = row.bg(palette::Color::new(35, 45, 65));
            }
            rows.push(row);
        }

        if self.items.len() > self.max_visible {
            rows.push(
                UiNode::text(format!("{} / {}", selected + 1, self.items.len()))
                    .color(palette::MUTED),
            );
        }

        UiNode::column(rows)
    }
}

impl From<SelectList> for UiNode {
    fn from(list: SelectList) -> Self {
        list.build()
    }
}

#[derive(Debug, Clone)]
pub struct SettingItem {
    pub label: String,
    pub value: String,
    pub description: Option<String>,
}

impl SettingItem {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            description: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct SettingsList {
    items: Vec<SettingItem>,
    selected: Option<usize>,
}

impl SettingsList {
    pub fn new(items: Vec<SettingItem>) -> Self {
        Self {
            items,
            selected: None,
        }
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = Some(selected);
        self
    }

    pub fn build(self) -> UiNode {
        let rows = self
            .items
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                let selected = self.selected == Some(index);
                let mut left = vec![UiNode::text(item.label).bold()];
                if let Some(description) = item.description {
                    left.push(UiNode::text(description).color(palette::MUTED));
                }
                let mut row = UiNode::row(vec![
                    UiNode::column(left),
                    UiNode::text(item.value).color(palette::PRIMARY),
                ])
                .justify(Justify::SpaceBetween)
                .align(Align::Center);
                if selected {
                    row = row.bg(palette::Color::new(35, 45, 65));
                }
                row
            })
            .collect();

        UiNode::column(rows).gap(1)
    }
}

impl From<SettingsList> for UiNode {
    fn from(list: SettingsList) -> Self {
        list.build()
    }
}

#[derive(Debug, Clone)]
pub struct Markdown {
    source: String,
}

impl Markdown {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
        }
    }

    pub fn build(self) -> UiNode {
        UiNode::column(
            self.source
                .lines()
                .map(markdown_line)
                .collect::<Vec<UiNode>>(),
        )
    }
}

impl From<Markdown> for UiNode {
    fn from(markdown: Markdown) -> Self {
        markdown.build()
    }
}

#[derive(Debug, Clone)]
pub struct Image {
    frame_id: String,
    width: u16,
    height: u16,
    bg: Color,
}

impl Image {
    pub fn new(frame_id: impl Into<String>, width: u16, height: u16) -> Self {
        Self {
            frame_id: frame_id.into(),
            width,
            height,
            bg: Color::BLACK,
        }
    }

    pub fn background(mut self, color: Color) -> Self {
        self.bg = color;
        self
    }

    pub fn build(self) -> UiNode {
        UiNode::Canvas(CanvasNode {
            width: self.width,
            height: self.height,
            frame_id: self.frame_id,
            bg: self.bg,
        })
    }
}

impl From<Image> for UiNode {
    fn from(image: Image) -> Self {
        image.build()
    }
}

pub fn input(value: impl Into<String>) -> TextInput {
    TextInput::new(value)
}

pub fn editor(lines: Vec<String>) -> Editor {
    Editor::from_lines(lines)
}

fn markdown_line(line: &str) -> UiNode {
    if let Some(text) = line.strip_prefix("# ") {
        return UiNode::text(text).bold().color(palette::PRIMARY);
    }
    if let Some(text) = line.strip_prefix("## ") {
        return UiNode::text(text).bold();
    }
    if let Some(text) = line.strip_prefix("> ") {
        return UiNode::text(text).color(palette::MUTED).italic();
    }
    if let Some(text) = line.strip_prefix("- ") {
        return UiNode::row(vec![
            UiNode::text("-").color(palette::MUTED),
            UiNode::text(text),
        ])
        .gap(1);
    }
    UiNode::text(line)
}

fn truncate_chars(text: &str, max_chars: usize, suffix: &str) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_string();
    }
    if max_chars == 0 {
        return suffix.to_string();
    }
    let keep = max_chars.saturating_sub(suffix.chars().count());
    let mut result: String = text.chars().take(keep).collect();
    result.push_str(suffix);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncated_text_applies_suffix() {
        let node = TruncatedText::new("abcdef", 4).suffix("..").build();

        match node {
            UiNode::Text(text) => assert_eq!(text.content, "ab.."),
            _ => panic!("expected text node"),
        }
    }

    #[test]
    fn select_list_marks_selected_item() {
        let node = SelectList::new(vec![
            SelectItem::new("one", "One"),
            SelectItem::new("two", "Two").description("Second"),
        ])
        .selected(1)
        .build();

        match node {
            UiNode::Column(column) => assert_eq!(column.children.len(), 2),
            _ => panic!("expected column node"),
        }
    }

    #[test]
    fn markdown_heading_is_styled_text() {
        let node = Markdown::new("# Title").build();

        match node {
            UiNode::Column(column) => match &column.children[0] {
                UiNode::Text(text) => assert_eq!(text.content, "Title"),
                _ => panic!("expected text node"),
            },
            _ => panic!("expected column node"),
        }
    }
}
