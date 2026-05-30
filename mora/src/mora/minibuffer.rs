#[derive(Debug, Clone, Default)]
pub struct Minibuffer {
    active: bool,
    prompt: String,
    input: String,
    completions: Vec<String>,
}

impl Minibuffer {
    pub fn activate(&mut self, prompt: impl Into<String>) {
        self.active = true;
        self.prompt = prompt.into();
        self.input.clear();
        self.completions.clear();
    }

    pub fn clear(&mut self) {
        self.active = false;
        self.prompt.clear();
        self.input.clear();
        self.completions.clear();
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn set_input(&mut self, input: impl Into<String>) {
        self.input = input.into();
    }

    pub fn push(&mut self, ch: char) {
        self.input.push(ch);
    }

    pub fn pop(&mut self) {
        self.input.pop();
    }

    pub fn set_completions(&mut self, completions: Vec<String>) {
        self.completions = completions;
    }

    pub fn completions(&self) -> &[String] {
        &self.completions
    }

    pub fn complete_prefix(&mut self) -> CompletionResult {
        let input = self.input.trim();
        if input.is_empty() {
            return CompletionResult::None;
        }

        // Orderless-style: split input on spaces, each component must
        // match somewhere in the candidate (substring matching).
        // e.g. "ser load" matches "server-load", "load-server", etc.
        // For single-component input, prefer prefix matching (fast path).
        let components: Vec<&str> = input.split_whitespace().collect();

        let matches: Vec<&str> = self
            .completions
            .iter()
            .map(String::as_str)
            .filter(|candidate| {
                if components.len() == 1 {
                    candidate.starts_with(components[0])
                } else {
                    components.iter().all(|comp| candidate.contains(comp))
                }
            })
            .collect();

        match matches.as_slice() {
            [] => CompletionResult::None,
            [single] => {
                self.input = (*single).to_string();
                CompletionResult::Completed
            }
            _ => {
                let common = common_prefix(&matches);
                if common.len() > input.len() {
                    self.input = common;
                    CompletionResult::Completed
                } else {
                    CompletionResult::Matches(matches.into_iter().map(str::to_string).collect())
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionResult {
    None,
    Completed,
    Matches(Vec<String>),
}

fn common_prefix(items: &[&str]) -> String {
    items.iter().fold(items[0].to_string(), |acc, item| {
        acc.chars()
            .zip(item.chars())
            .take_while(|(a, b)| a == b)
            .map(|(a, _)| a)
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activate_and_edit_input() {
        let mut minibuffer = Minibuffer::default();

        minibuffer.activate("M-x ");
        minibuffer.push('s');
        minibuffer.push('a');
        minibuffer.pop();

        assert!(minibuffer.is_active());
        assert_eq!(minibuffer.prompt(), "M-x ");
        assert_eq!(minibuffer.input(), "s");
    }

    #[test]
    fn complete_unique_candidate() {
        let mut minibuffer = Minibuffer::default();
        minibuffer.activate("M-x ");
        minibuffer.set_input("save-b");
        minibuffer.set_completions(vec!["save-buffer".to_string(), "switch-buffer".to_string()]);

        assert_eq!(minibuffer.complete_prefix(), CompletionResult::Completed);
        assert_eq!(minibuffer.input(), "save-buffer");
    }

    #[test]
    fn complete_common_prefix_or_returns_matches() {
        let mut minibuffer = Minibuffer::default();
        minibuffer.activate("M-x ");
        minibuffer.set_input("s");
        minibuffer.set_completions(vec!["save-buffer".to_string(), "save-file".to_string()]);

        assert_eq!(minibuffer.complete_prefix(), CompletionResult::Completed);
        assert_eq!(minibuffer.input(), "save-");

        assert_eq!(
            minibuffer.complete_prefix(),
            CompletionResult::Matches(vec!["save-buffer".to_string(), "save-file".to_string()])
        );
    }
}
