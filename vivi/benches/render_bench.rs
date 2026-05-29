use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::path::Path;

use vivi::buffer::Buffer;
use vivi::editor::Editor;
use vivi::render;
use vivi::syntax::SyntaxHighlighter;

fn make_editor_with_lines(n: usize) -> Editor {
    let mut buf = Buffer::new();
    for i in 0..n {
        if i == 0 {
            buf.replace_range(0, 0, 0, &format!("line {i}: let x = 42 + 1; // comment"));
        } else {
            buf.insert_line(i, format!("line {i}: let x = 42 + 1; // comment"));
        }
    }
    let mut ed = Editor::new(buf);
    ed.set_term_size(120, 40);
    ed
}

fn bench_render_small_buffer(c: &mut Criterion) {
    let mut ed = make_editor_with_lines(50);
    c.bench_function("render_50_lines_120x40", |b| {
        b.iter(|| render::render(black_box(&mut ed), 120, 40))
    });
}

fn bench_render_medium_buffer(c: &mut Criterion) {
    let mut ed = make_editor_with_lines(500);
    c.bench_function("render_500_lines_120x40", |b| {
        b.iter(|| render::render(black_box(&mut ed), 120, 40))
    });
}

fn bench_render_large_buffer(c: &mut Criterion) {
    let mut ed = make_editor_with_lines(10_000);
    c.bench_function("render_10k_lines_120x40", |b| {
        b.iter(|| render::render(black_box(&mut ed), 120, 40))
    });
}

fn bench_render_narrow(c: &mut Criterion) {
    let mut ed = make_editor_with_lines(200);
    c.bench_function("render_200_lines_80x24", |b| {
        b.iter(|| render::render(black_box(&mut ed), 80, 24))
    });
}

fn bench_syntax_highlight_visible_only(c: &mut Criterion) {
    let highlighter = SyntaxHighlighter::global();
    let lines: Vec<String> = (0..10_000)
        .map(|i| format!("fn func_{i}() {{ let x: i32 = {i}; x + 1 }}"))
        .collect();
    let line_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    let path = Path::new("bench.rs");

    // Benchmark: highlight only visible window (38 lines out of 10k)
    c.bench_function("syntax_highlight_range_38_of_10k", |b| {
        b.iter(|| highlighter.highlight_range(black_box(&line_refs), Some(path), 100, 38))
    });

    // Benchmark: highlight all 10k lines (the old way — regression guard)
    c.bench_function("syntax_highlight_buffer_10k_full", |b| {
        b.iter(|| highlighter.highlight_buffer(black_box(&line_refs), Some(path)))
    });
}

fn bench_syntax_highlight_plain(c: &mut Criterion) {
    let highlighter = SyntaxHighlighter::global();
    let lines: Vec<String> = (0..100)
        .map(|i| format!("plain text line number {i}"))
        .collect();
    let line_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();

    // No file path → plain fallback, no syntect work
    c.bench_function("syntax_highlight_plain_100", |b| {
        b.iter(|| highlighter.highlight_range(black_box(&line_refs), None, 0, 100))
    });
}

fn bench_cursor_movement(c: &mut Criterion) {
    let mut ed = make_editor_with_lines(1000);
    ed.set_term_size(120, 40);

    c.bench_function("cursor_j_1000_lines", |b| {
        b.iter(|| {
            let mut ed = make_editor_with_lines(1000);
            ed.set_term_size(120, 40);
            for _ in 0..50 {
                ed.handle_key(display_protocol::KeyEvent::char('j'));
            }
            for _ in 0..50 {
                ed.handle_key(display_protocol::KeyEvent::char('k'));
            }
        })
    });
}

criterion_group!(
    benches,
    bench_render_small_buffer,
    bench_render_medium_buffer,
    bench_render_large_buffer,
    bench_render_narrow,
    bench_syntax_highlight_visible_only,
    bench_syntax_highlight_plain,
    bench_cursor_movement,
);
criterion_main!(benches);
