use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mishell::shell::{is_builtin, Shell};
use mishell::syntax::SyntaxHighlighter;
use mishell_parser::{Lexer, Parser};

fn bench_lexer(c: &mut Criterion) {
    let simple = "echo hello world";
    let complex = "ls -la /tmp | grep -v foo | awk '{print $2}' && echo done || echo fail";
    let vars = "echo $HOME $USER $PATH $SHELL $TERM";
    let quoted = r#"echo "hello $USER world" 'single quoted' "double with $(cmd)""#;

    c.bench_function("lexer_simple", |b| {
        b.iter(|| Lexer::new(black_box(simple)).tokenize())
    });
    c.bench_function("lexer_complex", |b| {
        b.iter(|| Lexer::new(black_box(complex)).tokenize())
    });
    c.bench_function("lexer_vars", |b| {
        b.iter(|| Lexer::new(black_box(vars)).tokenize())
    });
    c.bench_function("lexer_quoted", |b| {
        b.iter(|| Lexer::new(black_box(quoted)).tokenize())
    });
}

fn bench_parser(c: &mut Criterion) {
    let simple = "echo hello world";
    let complex = "ls -la /tmp | grep -v foo | awk '{print $2}' && echo done || echo fail";
    let pipeline = "cat file | grep err | sort | uniq -c | sort -rn | head -20";

    c.bench_function("parser_simple", |b| {
        b.iter(|| Parser::new(black_box(simple)).parse())
    });
    c.bench_function("parser_complex", |b| {
        b.iter(|| Parser::new(black_box(complex)).parse())
    });
    c.bench_function("parser_pipeline", |b| {
        b.iter(|| Parser::new(black_box(pipeline)).parse())
    });
}

fn bench_highlight(c: &mut Criterion) {
    let h = SyntaxHighlighter::new();
    let simple = "echo hello world";
    let complex = "ls -la /tmp | grep -v foo | awk '{print $2}' && echo done || echo fail";
    let long = "git log --oneline --graph --all --decorate | head -50 && echo 'done' | tr a-z A-Z";

    c.bench_function("highlight_simple", |b| {
        b.iter(|| h.highlight(black_box(simple)))
    });
    c.bench_function("highlight_complex", |b| {
        b.iter(|| h.highlight(black_box(complex)))
    });
    c.bench_function("highlight_long", |b| {
        b.iter(|| h.highlight(black_box(long)))
    });
}

fn bench_execute(c: &mut Criterion) {
    c.bench_function("execute_echo", |b| {
        b.iter(|| {
            let mut shell = Shell::new().unwrap();
            shell.execute(black_box("echo hello world"))
        })
    });
    c.bench_function("execute_pipeline", |b| {
        b.iter(|| {
            let mut shell = Shell::new().unwrap();
            shell.execute(black_box("echo hello | tr a-z A-Z"))
        })
    });
    c.bench_function("execute_assign", |b| {
        b.iter(|| {
            let mut shell = Shell::new().unwrap();
            shell.execute(black_box("FOO=bar"))
        })
    });
    c.bench_function("execute_true", |b| {
        b.iter(|| {
            let mut shell = Shell::new().unwrap();
            shell.execute(black_box("true"))
        })
    });
    c.bench_function("execute_false", |b| {
        b.iter(|| {
            let mut shell = Shell::new().unwrap();
            shell.execute(black_box("false"))
        })
    });
    c.bench_function("execute_pwd", |b| {
        b.iter(|| {
            let mut shell = Shell::new().unwrap();
            shell.execute(black_box("pwd"))
        })
    });
}

fn bench_glob(c: &mut Criterion) {
    c.bench_function("glob_star", |b| {
        b.iter(|| Shell::glob_match(black_box("*.rs"), black_box("main.rs")))
    });
    c.bench_function("glob_question", |b| {
        b.iter(|| Shell::glob_match(black_box("?.txt"), black_box("a.txt")))
    });
    c.bench_function("glob_char_class", |b| {
        b.iter(|| Shell::glob_match(black_box("[A-Z]*.rs"), black_box("Main.rs")))
    });
}

fn bench_is_builtin(c: &mut Criterion) {
    c.bench_function("is_builtin_hit", |b| {
        b.iter(|| is_builtin(black_box("echo")))
    });
    c.bench_function("is_builtin_miss", |b| {
        b.iter(|| is_builtin(black_box("notacommand")))
    });
}

criterion_group!(
    benches,
    bench_lexer,
    bench_parser,
    bench_highlight,
    bench_execute,
    bench_glob,
    bench_is_builtin
);
criterion_main!(benches);
