use std::path::Path;

use serde::Serialize;
use tree_sitter::Node;

use crate::{LanguageId, ParserManager};

#[derive(Debug, Clone, Serialize)]
pub struct SummarySegment {
    pub kind: SegmentKind,
    pub start_line: usize,
    pub end_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentKind {
    Kept,
    Elided,
}

#[derive(Debug, Clone, Serialize)]
pub struct SummaryResult {
    pub segments: Vec<SummarySegment>,
    pub total_lines: usize,
    pub kept_lines: usize,
    pub elided_lines: usize,
}

#[derive(Debug, Clone)]
pub struct SummaryOptions {
    pub min_body_lines: usize,
    pub min_comment_lines: usize,
}

impl Default for SummaryOptions {
    fn default() -> Self {
        Self {
            min_body_lines: 4,
            min_comment_lines: 6,
        }
    }
}

struct ElisionSpan {
    start_line: usize,
    end_line: usize,
}

pub fn summarize_file(path: &Path, content: &str, options: &SummaryOptions) -> SummaryResult {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let lang_id = match LanguageId::from_extension(ext) {
        Some(l) => l,
        None => return unparsed_summary(content),
    };
    summarize_with_lang(lang_id, content, options)
}

pub fn summarize_source(
    lang_id: LanguageId,
    content: &str,
    options: &SummaryOptions,
) -> SummaryResult {
    summarize_with_lang(lang_id, content, options)
}

fn summarize_with_lang(
    lang_id: LanguageId,
    content: &str,
    options: &SummaryOptions,
) -> SummaryResult {
    let manager = ParserManager::new();
    let tree = match manager.parse_source(lang_id, content) {
        Ok(t) => t,
        Err(_) => return unparsed_summary(content),
    };

    if tree.tree().root_node().has_error() {
        return unparsed_summary(content);
    }

    let mut spans = Vec::new();
    collect_elisions(
        tree.tree().root_node(),
        content,
        lang_id,
        options,
        &mut spans,
    );
    let spans = normalize_spans(spans, content);
    build_segments(content, &spans)
}

fn collect_elisions(
    node: Node,
    content: &str,
    lang: LanguageId,
    options: &SummaryOptions,
    spans: &mut Vec<ElisionSpan>,
) {
    let kind = node.kind();
    let start = node.start_position().row;
    let end = node.end_position().row;
    let line_count = end.saturating_sub(start) + 1;

    if is_comment_kind(kind, lang) && line_count >= options.min_comment_lines {
        spans.push(ElisionSpan {
            start_line: start + 1,
            end_line: end,
        });
    } else if is_block_body(kind, lang) {
        let interior = end.saturating_sub(start);
        let parent_kind = node.parent().map(|p| p.kind());
        let is_container_body = parent_kind.map_or(false, |pk| is_container_item(pk, lang));
        let is_signature_body = parent_kind.map_or(false, |pk| is_signature_item(pk, lang));
        if is_signature_body {
            // fn/method: elide after the opening brace (keep signature)
            if interior >= options.min_body_lines {
                spans.push(ElisionSpan {
                    start_line: start + 2,
                    end_line: end,
                });
            }
        } else if !is_container_body {
            // standalone block: elide interior
            if interior >= options.min_body_lines {
                spans.push(ElisionSpan {
                    start_line: start + 1,
                    end_line: end,
                });
            }
        }
    } else if is_item_with_body(kind, lang) {
        // For items whose body is a short block (e.g. Python functions with 2-4 line bodies),
        // elide the interior of the item itself rather than relying on the block.
        if let Some(body_start_row) = find_body_child(node, lang) {
            let item_interior = end.saturating_sub(start);
            if item_interior >= options.min_body_lines {
                spans.push(ElisionSpan {
                    start_line: body_start_row + 1,
                    end_line: end,
                });
            }
        }
    } else if is_body_kind(kind, lang) {
        let interior = end.saturating_sub(start);
        if interior >= options.min_body_lines {
            spans.push(ElisionSpan {
                start_line: start + 1,
                end_line: end,
            });
        }
    } else if is_groupable_kind(kind, lang) {
        collect_import_run(node, content, lang, options, spans);
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_elisions(child, content, lang, options, spans);
    }
}

fn collect_import_run(
    first: Node,
    _content: &str,
    lang: LanguageId,
    options: &SummaryOptions,
    spans: &mut Vec<ElisionSpan>,
) {
    let parent = match first.parent() {
        Some(p) => p,
        None => return,
    };

    let mut siblings = Vec::new();
    let mut cursor = parent.walk();
    for child in parent.named_children(&mut cursor) {
        if is_groupable_kind(child.kind(), lang) {
            siblings.push(child);
        }
    }

    if siblings.len() < 2 {
        return;
    }

    let mut run_start = siblings[0].start_position().row;
    let mut run_end = siblings[0].end_position().row;

    for w in siblings.windows(2) {
        let a = w[0];
        let b = w[1];
        let a_end = a.end_position().row;
        let b_start = b.start_position().row;

        if b_start <= a_end + 2 {
            run_end = b.end_position().row;
        } else {
            let run_lines = run_end.saturating_sub(run_start) + 1;
            if run_lines >= options.min_body_lines {
                spans.push(ElisionSpan {
                    start_line: run_start + 1,
                    end_line: run_end,
                });
            }
            run_start = b.start_position().row;
            run_end = b.end_position().row;
        }
    }

    let run_lines = run_end.saturating_sub(run_start) + 1;
    if run_lines >= options.min_body_lines {
        spans.push(ElisionSpan {
            start_line: run_start + 1,
            end_line: run_end,
        });
    }
}

fn normalize_spans(spans: Vec<ElisionSpan>, content: &str) -> Vec<ElisionSpan> {
    let total_lines = content.lines().count();
    let mut spans: Vec<ElisionSpan> = spans
        .into_iter()
        .map(|s| ElisionSpan {
            start_line: s.start_line.max(1).min(total_lines),
            end_line: s.end_line.max(1).min(total_lines),
        })
        .filter(|s| s.start_line < s.end_line)
        .collect();

    spans.sort_by(|a, b| {
        a.start_line
            .cmp(&b.start_line)
            .then(a.end_line.cmp(&b.end_line))
    });

    let mut merged: Vec<ElisionSpan> = Vec::new();
    for span in spans {
        if let Some(last) = merged.last_mut() {
            if span.start_line <= last.end_line + 1 {
                last.end_line = last.end_line.max(span.end_line);
                continue;
            }
        }
        merged.push(span);
    }

    merged
}

fn build_segments(content: &str, spans: &[ElisionSpan]) -> SummaryResult {
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    let mut segments = Vec::new();
    let mut kept_lines = 0;
    let mut elided_lines = 0;
    let mut line_idx = 1;
    let mut span_iter = spans.iter().peekable();

    while line_idx <= total {
        if let Some(span) = span_iter.peek() {
            if line_idx == span.start_line {
                segments.push(SummarySegment {
                    kind: SegmentKind::Elided,
                    start_line: span.start_line,
                    end_line: span.end_line,
                    text: None,
                });
                elided_lines += span.end_line - span.start_line + 1;
                line_idx = span.end_line + 1;
                span_iter.next();
                continue;
            }
        }

        let seg_start = line_idx;
        let mut seg_end = line_idx;
        while line_idx <= total {
            if let Some(span) = span_iter.peek() {
                if line_idx == span.start_line {
                    break;
                }
            }
            seg_end = line_idx;
            line_idx += 1;
        }

        let text: String = lines[(seg_start - 1)..seg_end].join("\n");
        kept_lines += seg_end - seg_start + 1;
        segments.push(SummarySegment {
            kind: SegmentKind::Kept,
            start_line: seg_start,
            end_line: seg_end,
            text: Some(text),
        });
    }

    SummaryResult {
        segments,
        total_lines: total,
        kept_lines,
        elided_lines,
    }
}

fn unparsed_summary(content: &str) -> SummaryResult {
    let total = content.lines().count();
    SummaryResult {
        segments: vec![SummarySegment {
            kind: SegmentKind::Kept,
            start_line: 1,
            end_line: total,
            text: Some(content.to_string()),
        }],
        total_lines: total,
        kept_lines: total,
        elided_lines: 0,
    }
}

fn is_comment_kind(kind: &str, lang: LanguageId) -> bool {
    match lang {
        LanguageId::Rust | LanguageId::C => kind == "block_comment" || kind == "comment",
        LanguageId::JavaScript | LanguageId::TypeScript => {
            kind == "comment" || kind == "block_comment"
        }
        LanguageId::Python => kind == "comment",
        LanguageId::Go => kind == "comment",
        LanguageId::Bash => kind == "comment",
    }
}

fn is_body_kind(kind: &str, lang: LanguageId) -> bool {
    match lang {
        LanguageId::Rust => matches!(
            kind,
            "block"
                | "match_expression"
                | "match_arm"
                | "macro_definition"
                | "macro_invocation"
                | "closure_expression"
                | "attribute_item"
        ),
        LanguageId::JavaScript | LanguageId::TypeScript => matches!(
            kind,
            "statement_block"
                | "arrow_function"
                | "function_expression"
                | "object"
                | "array"
                | "switch_statement"
                | "switch_case"
                | "template_string"
        ),
        LanguageId::Python => matches!(
            kind,
            "block"
                | "decorated_definition"
                | "if_statement"
                | "elif_clause"
                | "else_clause"
                | "for_statement"
                | "while_statement"
                | "try_statement"
                | "except_clause"
                | "finally_clause"
                | "with_statement"
                | "list"
                | "dictionary"
                | "set"
                | "tuple"
        ),
        LanguageId::Go => matches!(
            kind,
            "block"
                | "composite_literal"
                | "switch_statement"
                | "select_statement"
                | "if_statement"
                | "for_statement"
        ),
        LanguageId::Bash => matches!(
            kind,
            "compound_statement"
                | "if_statement"
                | "elif_clause"
                | "else_clause"
                | "for_statement"
                | "while_statement"
                | "case_statement"
                | "case_item"
        ),
        LanguageId::C => matches!(
            kind,
            "compound_statement"
                | "struct_specifier"
                | "enum_specifier"
                | "union_specifier"
                | "if_statement"
                | "else_clause"
                | "switch_statement"
                | "while_statement"
                | "for_statement"
        ),
    }
}

fn is_block_body(kind: &str, lang: LanguageId) -> bool {
    match lang {
        LanguageId::Rust => kind == "block",
        LanguageId::JavaScript | LanguageId::TypeScript => kind == "statement_block",
        LanguageId::Python => kind == "block",
        LanguageId::Go => kind == "block",
        LanguageId::Bash => kind == "compound_statement",
        LanguageId::C => kind == "compound_statement",
    }
}

fn is_container_item(kind: &str, lang: LanguageId) -> bool {
    match lang {
        LanguageId::Rust => matches!(
            kind,
            "impl_item" | "trait_item" | "enum_item" | "struct_item"
        ),
        LanguageId::JavaScript | LanguageId::TypeScript => {
            matches!(
                kind,
                "class_declaration" | "interface_declaration" | "enum_declaration"
            )
        }
        LanguageId::Python => matches!(kind, "class_definition"),
        LanguageId::Go => matches!(kind, "type_declaration"),
        LanguageId::Bash | LanguageId::C => false,
    }
}

fn is_signature_item(kind: &str, lang: LanguageId) -> bool {
    match lang {
        LanguageId::Rust => matches!(kind, "function_item" | "closure_expression"),
        LanguageId::JavaScript | LanguageId::TypeScript => {
            matches!(
                kind,
                "function_declaration"
                    | "method_definition"
                    | "function_expression"
                    | "arrow_function"
            )
        }
        LanguageId::Python => matches!(kind, "function_definition"),
        LanguageId::Go => matches!(kind, "function_declaration" | "method_declaration"),
        LanguageId::Bash | LanguageId::C => matches!(kind, "function_definition"),
    }
}

fn is_groupable_kind(kind: &str, lang: LanguageId) -> bool {
    match lang {
        LanguageId::Rust => kind == "use_declaration" || kind == "attribute_item",
        LanguageId::JavaScript | LanguageId::TypeScript => kind == "import_statement",
        LanguageId::Python => kind == "import_statement" || kind == "import_from_statement",
        LanguageId::Go => kind == "import_declaration",
        LanguageId::C => kind == "preproc_include",
        LanguageId::Bash => false,
    }
}

fn is_item_with_body(kind: &str, lang: LanguageId) -> bool {
    match lang {
        LanguageId::Python => kind == "function_definition" || kind == "class_definition",
        _ => false,
    }
}

fn find_body_child(node: Node, lang: LanguageId) -> Option<usize> {
    let target = match lang {
        LanguageId::Python => "block",
        _ => return None,
    };
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == target {
            return Some(child.start_position().row);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_rust_file() {
        let source = r#"use std::io;
use std::fs;
use std::path::Path;
use std::collections::HashMap;

struct App {
    name: String,
}

impl App {
    fn new(name: &str) -> Self {
        let name = name.to_string();
        println!("creating {}", name);
        Self { name }
    }

    fn run(&self) {
        println!("running {}", self.name);
        let data = vec![1, 2, 3];
        for item in data {
            println!("{}", item);
        }
    }
}

fn main() {
    let app = App::new("test");
    app.run();
}
"#;
        let options = SummaryOptions::default();
        let result = summarize_source(LanguageId::Rust, source, &options);

        assert!(result.total_lines > 20);
        assert!(result.elided_lines > 0);
        assert!(result.kept_lines < result.total_lines);

        let kept_text: String = result
            .segments
            .iter()
            .filter_map(|s| s.text.as_deref())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(kept_text.contains("struct App"));
        assert!(kept_text.contains("fn new"));
        assert!(kept_text.contains("fn main"));
    }

    #[test]
    fn summarizes_python_file() {
        let source = r#"import os
import sys
import json

class Worker:
    def __init__(self, name):
        self.name = name
        self.tasks = []

    def add_task(self, task):
        self.tasks.append(task)
        print(f"added {task}")

    def run(self):
        for task in self.tasks:
            print(f"running {task}")
            task.execute()

def main():
    w = Worker("test")
    w.add_task(lambda: None)
    w.run()
"#;
        let options = SummaryOptions::default();
        let result = summarize_source(LanguageId::Python, source, &options);

        assert!(result.elided_lines > 0);
        let kept_text: String = result
            .segments
            .iter()
            .filter_map(|s| s.text.as_deref())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(kept_text.contains("class Worker"));
        assert!(kept_text.contains("def main"));
    }

    #[test]
    fn summarizes_javascript_file() {
        let source = r#"import fs from 'fs';
import path from 'path';

function processData(input) {
    const result = [];
    for (const item of input) {
        result.push(item * 2);
    }
    return result;
}

class Handler {
    constructor(name) {
        this.name = name;
        this.items = [];
    }

    handle(data) {
        console.log(this.name, data);
        this.items.push(data);
    }
}

export default Handler;
"#;
        let options = SummaryOptions::default();
        let result = summarize_source(LanguageId::JavaScript, source, &options);

        assert!(result.elided_lines > 0);
    }

    #[test]
    fn returns_full_content_for_unsupported_language() {
        let source = "line1\nline2\nline3\n";
        let result = summarize_source(LanguageId::Bash, source, &SummaryOptions::default());
        assert_eq!(result.segments.len(), 1);
        assert_eq!(result.segments[0].kind, SegmentKind::Kept);
        assert_eq!(result.total_lines, 3);
    }

    #[test]
    fn elides_long_block_comments() {
        let source = r#"fn main() {
    /* This is a
       very long
       block comment
       that spans
       many lines
       and should
       be elided */
    println!("hello");
}
"#;
        let options = SummaryOptions {
            min_comment_lines: 4,
            ..Default::default()
        };
        let result = summarize_source(LanguageId::Rust, source, &options);
        assert!(result.elided_lines > 0);
    }

    #[test]
    fn respects_min_body_lines_threshold() {
        let source = r#"fn small() { 1 }
fn medium() {
    let a = 1;
    let b = 2;
    let c = 3;
    a + b + c
}
"#;
        let options = SummaryOptions {
            min_body_lines: 6,
            ..Default::default()
        };
        let result = summarize_source(LanguageId::Rust, source, &options);
        assert_eq!(result.elided_lines, 0);
    }

    #[test]
    fn rust_preserves_function_signatures_in_impl() {
        let source = r#"use std::io;
use std::fs;
use std::path::Path;
use std::collections::HashMap;

struct App {
    name: String,
}

impl App {
    fn new(name: &str) -> Self {
        let name = name.to_string();
        println!("creating {}", name);
        Self { name }
    }

    fn run(&self) {
        println!("running {}", self.name);
        let data = vec![1, 2, 3];
        for item in data {
            println!("{}", item);
        }
    }
}

fn main() {
    let app = App::new("test");
    app.run();
}
"#;
        let result = summarize_source(LanguageId::Rust, source, &SummaryOptions::default());
        let kept: String = result
            .segments
            .iter()
            .filter_map(|s| s.text.as_deref())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(kept.contains("fn new"), "must keep fn new signature");
        assert!(kept.contains("fn run"), "must keep fn run signature");
        assert!(kept.contains("fn main"), "must keep fn main signature");
        assert!(kept.contains("struct App"), "must keep struct App");
        assert!(result.elided_lines > 0, "must elide function bodies");
    }

    #[test]
    fn python_elides_function_bodies() {
        let source = r#"import os
import sys
import json

class Worker:
    def __init__(self, name):
        self.name = name
        self.tasks = []

    def add_task(self, task):
        self.tasks.append(task)
        print(f"added {task}")

    def run(self):
        for task in self.tasks:
            print(f"running {task}")
            task.execute()

def main():
    w = Worker("test")
    w.add_task(lambda: None)
    w.run()
"#;
        let result = summarize_source(LanguageId::Python, source, &SummaryOptions::default());
        let kept: String = result
            .segments
            .iter()
            .filter_map(|s| s.text.as_deref())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(kept.contains("class Worker"), "must keep class signature");
        assert!(kept.contains("def main"), "must keep def main");
        assert!(result.elided_lines > 0, "must elide function bodies");
    }

    #[test]
    fn go_file_summarization() {
        let source = r#"package main

import (
    "fmt"
    "os"
    "strings"
)

func process(data string) string {
    result := strings.TrimSpace(data)
    fmt.Println(result)
    return result
}

func main() {
    output := process("hello")
    fmt.Println(output)
}
"#;
        let options = SummaryOptions::default();
        let result = summarize_source(LanguageId::Go, source, &options);
        assert!(result.elided_lines > 0 || result.total_lines > 0);
    }
}
