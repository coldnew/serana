#[derive(Debug, Clone, Default)]
pub struct Minibuffer {
    active: bool,
    prompt: String,
    input: String,
    completions: Vec<CompletionItem>,
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
        self.completions = completions
            .into_iter()
            .map(CompletionItem::command)
            .collect();
    }

    pub fn set_completion_items(&mut self, completions: Vec<CompletionItem>) {
        self.completions = completions;
    }

    pub fn completions(&self) -> &[CompletionItem] {
        &self.completions
    }

    pub fn complete_prefix(&mut self) -> CompletionResult {
        let input = self.input.trim();
        if input.is_empty() {
            return CompletionResult::None;
        }

        let components: Vec<&str> = input.split_whitespace().collect();

        let mut matches: Vec<CompletionMatch> = self
            .completions
            .iter()
            .filter_map(|candidate| {
                let score = completion_score(candidate, &components)?;
                Some(CompletionMatch {
                    label: candidate.label.clone(),
                    insert_text: candidate.insert_text.clone(),
                    sort_text: candidate.sort_text.clone(),
                    score,
                })
            })
            .collect();
        if components.len() == 1 && matches.iter().any(|m| m.score >= 900) {
            matches.retain(|m| m.score >= 900);
        }
        matches.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| a.sort_text.cmp(&b.sort_text))
                .then_with(|| a.label.cmp(&b.label))
        });

        match matches.as_slice() {
            [] => CompletionResult::None,
            [single] => {
                self.input = single.insert_text.clone();
                CompletionResult::Completed
            }
            _ => {
                let insert_texts: Vec<&str> =
                    matches.iter().map(|m| m.insert_text.as_str()).collect();
                let common = common_prefix(&insert_texts);
                if common.len() > input.len() {
                    self.input = common;
                    CompletionResult::Completed
                } else {
                    CompletionResult::Matches(matches.into_iter().map(|m| m.label).collect())
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: String,
    pub filter_text: String,
    pub sort_text: String,
}

impl CompletionItem {
    pub fn new(label: impl Into<String>, kind: CompletionKind) -> Self {
        let label = label.into();
        Self {
            insert_text: label.clone(),
            filter_text: label.clone(),
            sort_text: label.clone(),
            label,
            kind,
            detail: None,
            documentation: None,
        }
    }

    pub fn command(label: impl Into<String>) -> Self {
        Self::new(label, CompletionKind::Command)
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Command,
    File,
    Buffer,
    Function,
    Variable,
    Keyword,
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionResult {
    None,
    Completed,
    Matches(Vec<String>),
}

#[derive(Debug)]
struct CompletionMatch {
    label: String,
    insert_text: String,
    sort_text: String,
    score: i32,
}

fn completion_score(item: &CompletionItem, components: &[&str]) -> Option<i32> {
    components
        .iter()
        .enumerate()
        .try_fold(0, |score, (idx, component)| {
            match_component(item, component)
                .map(|component_score| score + component_score - idx as i32)
        })
}

fn match_component(item: &CompletionItem, component: &str) -> Option<i32> {
    let needle = component.to_ascii_lowercase();
    let label = item.label.to_ascii_lowercase();
    let filter_text = item.filter_text.to_ascii_lowercase();

    if label == needle || filter_text == needle {
        return Some(1000);
    }
    if label.starts_with(&needle) || filter_text.starts_with(&needle) {
        return Some(900);
    }
    if label.contains(&needle) || filter_text.contains(&needle) {
        return Some(700);
    }
    fuzzy_score(&filter_text, &needle).map(|score| 500 + score)
}

fn fuzzy_score(haystack: &str, needle: &str) -> Option<i32> {
    if needle.is_empty() {
        return Some(0);
    }

    let mut score = 0;
    let mut search_from = 0;
    let mut previous_match = None;
    for needle_ch in needle.chars() {
        let offset = haystack[search_from..].find(needle_ch)?;
        let idx = search_from + offset;
        score += if previous_match == Some(idx.saturating_sub(1)) {
            8
        } else if idx == 0
            || haystack[..idx]
                .chars()
                .last()
                .is_some_and(|ch| matches!(ch, '-' | '_' | '/' | ' ' | '.'))
        {
            6
        } else {
            1
        };
        search_from = idx + needle_ch.len_utf8();
        previous_match = Some(idx);
    }
    Some(score)
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

    #[test]
    fn completion_items_support_fuzzy_matching() {
        let mut minibuffer = Minibuffer::default();
        minibuffer.activate("M-x ");
        minibuffer.set_input("fif");
        minibuffer.set_completion_items(vec![
            CompletionItem::command("find-file"),
            CompletionItem::command("format-buffer"),
        ]);

        assert_eq!(minibuffer.complete_prefix(), CompletionResult::Completed);
        assert_eq!(minibuffer.input(), "find-file");
    }

    #[test]
    fn completion_items_support_orderless_components() {
        let mut minibuffer = Minibuffer::default();
        minibuffer.activate("M-x ");
        minibuffer.set_input("buf save");
        minibuffer.set_completion_items(vec![
            CompletionItem::command("save-buffer"),
            CompletionItem::command("switch-buffer"),
        ]);

        assert_eq!(minibuffer.complete_prefix(), CompletionResult::Completed);
        assert_eq!(minibuffer.input(), "save-buffer");
    }
}
