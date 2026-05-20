//! Component system for inline TUI.

use std::any::Any;

/// Component trait — all UI elements implement this.
pub trait Component: Any {
    /// Render component to a list of lines.
    fn render(&self, width: usize) -> Vec<String>;

    /// Invalidate cached rendering state.
    fn invalidate(&mut self) {}

    fn as_any_mut(&mut self) -> &mut dyn Any where Self: Sized {
        self
    }

    fn as_any(&self) -> &dyn Any where Self: Sized {
        self
    }
}

/// Vertical container that stacks child components.
#[derive(Default)]
pub struct Container {
    children: Vec<Box<dyn Component>>,
}

impl Container {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    pub fn push(&mut self, component: impl Component + 'static) {
        self.children.push(Box::new(component));
    }

    pub fn add_child(&mut self, component: Box<dyn Component>) {
        self.children.push(component);
    }

    pub fn clear(&mut self) {
        self.children.clear();
    }

    pub fn children(&self) -> &[Box<dyn Component>] {
        &self.children
    }

    pub fn children_mut(&mut self) -> &mut Vec<Box<dyn Component>> {
        &mut self.children
    }
}

impl Component for Container {
    fn render(&self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        for child in &self.children {
            lines.extend(child.render(width));
        }
        lines
    }

    fn invalidate(&mut self) {
        for child in &mut self.children {
            child.invalidate();
        }
    }
}
