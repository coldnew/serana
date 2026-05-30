use super::core::*;
use super::editor_state::*;
use crate::lisp::types::Value;

#[test]
fn editor_primitives_remain_available_unqualified() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();

    bridge.eval("(editor-message \"legacy\")").unwrap();

    with_editor_state(|state| {
        assert_eq!(state.status_message, "legacy");
    });
    take_editor_state();
}
#[test]
fn buffer_symbols_are_available() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    // Test buffer-name works
    let name = bridge.eval("(buffer-name)").unwrap();
    assert_eq!(name, Value::string("*scratch*"));
    // Test buffer-line-count works
    let count = bridge.eval("(buffer-line-count)").unwrap();
    assert_eq!(count, Value::Int(1));
    // Test buffer-content works
    let content = bridge.eval("(buffer-content)").unwrap();
    assert_eq!(content, Value::string(""));
    // Test buffer-set-content works
    bridge.eval("(buffer-set-content \"hello\")").unwrap();
    let content = bridge.eval("(buffer-content)").unwrap();
    assert_eq!(content, Value::string("hello"));
    take_editor_state();
}

#[test]
fn editor_primitives_are_available_through_namespace_aliases() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();

    bridge
        .eval(
            r#"
                (ns coldnew.init)
                (require [mora.editor :as editor])
                (require [mora.buffer :as buffer])
                (editor/message (str "Buffer: " (buffer/name)))
                "#,
        )
        .unwrap();

    with_editor_state(|state| {
        assert_eq!(state.status_message, "Buffer: *scratch*");
    });
    take_editor_state();
}

#[test]
fn defcommand_registers_and_executes_editor_command() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();

    bridge
        .eval(
            r#"
                (ns coldnew.commands)
                (require [mora.editor :as editor])
                (defcommand say-hello
                  "Say hello from a user command."
                  []
                  (editor/message "hello from command"))
                "#,
        )
        .unwrap();

    assert!(bridge.has_command("say-hello"));
    assert!(bridge
        .command_names()
        .contains(&"coldnew.commands/say-hello".to_string()));
    assert_eq!(
        bridge.eval("(mora.command/doc \"say-hello\")").unwrap(),
        Value::string("Say hello from a user command.")
    );

    bridge.execute_command("say-hello").unwrap();
    with_editor_state(|state| {
        assert_eq!(state.status_message, "hello from command");
    });
    take_editor_state();
}

#[test]
fn interactive_defn_registers_and_executes_editor_command() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();

    bridge
        .eval(
            r#"
                (ns coldnew.commands)
                (require [mora.editor :as editor])
                (defn say-hello
                  "Say hello through interactive defn."
                  []
                  (interactive)
                  (editor/message "hello from interactive defn"))
                "#,
        )
        .unwrap();

    assert!(bridge.has_command("say-hello"));
    assert_eq!(
        bridge.eval("(mora.command/doc \"say-hello\")").unwrap(),
        Value::string("Say hello through interactive defn.")
    );

    bridge.execute_command("say-hello").unwrap();
    with_editor_state(|state| {
        assert_eq!(state.status_message, "hello from interactive defn");
    });
    take_editor_state();
}
// --- Kill Ring Tests ---
#[test]
fn kill_ring_push_and_yank() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    bridge
        .eval("(kill-ring-push \"hello world\")")
        .unwrap();
    let result = bridge.eval("(kill-ring-yank)").unwrap();
    assert_eq!(result, Value::string("hello world"));
    assert_eq!(
        bridge.eval("(kill-ring-count)").unwrap(),
        Value::Int(1)
    );
    take_editor_state();
}
#[test]
fn kill_ring_pop_cycling() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    bridge.eval("(kill-ring-push \"first\")").unwrap();
    bridge.eval("(kill-ring-push \"second\")").unwrap();
    bridge.eval("(kill-ring-push \"third\")").unwrap();
    // yank returns most recent
    assert_eq!(bridge.eval("(kill-ring-yank)").unwrap(), Value::string("third"));
    // pop forward cycles
    assert_eq!(bridge.eval("(kill-ring-pop)").unwrap(), Value::string("first"));
    // pop back cycles backward
    assert_eq!(bridge.eval("(kill-ring-pop-back)").unwrap(), Value::string("third"));
    let contents = bridge.eval("(kill-ring-contents)").unwrap();
    match contents {
        Value::Vector(v) => assert_eq!(v.len(), 3),
        _ => panic!("expected vector"),
    }
    take_editor_state();
}
#[test]
fn kill_ring_empty_returns_nil() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    assert_eq!(bridge.eval("(kill-ring-yank)").unwrap(), Value::Nil);
    assert_eq!(bridge.eval("(kill-ring-count)").unwrap(), Value::Int(0));
    take_editor_state();
}
// --- Mark Ring Tests ---
// --- Mark Ring Tests ---
#[test]
fn set_mark_and_goto() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    // Set up buffer with enough lines
    bridge.eval("(buffer-set-content \"line0\nline1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10\nline11\")").unwrap();
    // Move cursor to line 5, col 3
    bridge.eval("(cursor-set! 5 3)").unwrap();
    bridge.eval("(set-mark)").unwrap();
    assert_eq!(bridge.eval("(mark-active?)").unwrap(), Value::Bool(true));
    let pos = bridge.eval("(mark-position)").unwrap();
    match pos {
        Value::Vector(v) => {
            assert_eq!(v[0], Value::Int(5));
            assert_eq!(v[1], Value::Int(3));
        }
        _ => panic!("expected vector"),
    }
    // Move cursor elsewhere
    bridge.eval("(cursor-set! 10 0)").unwrap();
    // Goto mark
    bridge.eval("(goto-mark)").unwrap();
    assert_eq!(bridge.eval("(cursor-row)").unwrap(), Value::Int(5));
    assert_eq!(bridge.eval("(cursor-col)").unwrap(), Value::Int(3));
    take_editor_state();
}
#[test]
fn pop_mark_ring() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    // Set up buffer with enough lines
    bridge.eval("(buffer-set-content \"line0\nline1\nline2\nline3\nline4\nline5\")").unwrap();
    bridge.eval("(cursor-set! 1 0)").unwrap();
    bridge.eval("(set-mark)").unwrap();
    bridge.eval("(cursor-set! 2 0)").unwrap();
    bridge.eval("(set-mark)").unwrap();
    bridge.eval("(cursor-set! 3 0)").unwrap();
    // Pop mark -> goes to row 2
    bridge.eval("(pop-mark)").unwrap();
    assert_eq!(bridge.eval("(cursor-row)").unwrap(), Value::Int(2));
    // Deactivate mark
    bridge.eval("(deactivate-mark)").unwrap();
    assert_eq!(bridge.eval("(mark-active?)").unwrap(), Value::Bool(false));
    take_editor_state();
}
// --- Register Tests ---
#[test]
fn register_set_and_get() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    bridge.eval("(register-set \"a\" \"hello\")").unwrap();
    assert_eq!(
        bridge.eval("(register-get \"a\")").unwrap(),
        Value::string("hello")
    );
    assert_eq!(bridge.eval("(register-get \"z\")").unwrap(), Value::Nil);
    take_editor_state();
}
// --- Buffer-Local Variable Tests ---
#[test]
fn var_set_and_get() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    bridge.eval("(var-set \"tab-width\" 4)").unwrap();
    assert_eq!(
        bridge.eval("(var-get \"tab-width\")").unwrap(),
        Value::Int(4)
    );
    assert_eq!(bridge.eval("(var-bound? \"tab-width\")").unwrap(), Value::Bool(true));
    assert_eq!(bridge.eval("(var-bound? \"unknown\")").unwrap(), Value::Bool(false));
    assert_eq!(bridge.eval("(var-get \"unknown\")").unwrap(), Value::Nil);
    take_editor_state();
}
#[test]
fn var_local_sets_default_only() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    bridge.eval("(var-local \"indent-tabs-mode\" true)").unwrap();
    assert_eq!(
        bridge.eval("(var-get \"indent-tabs-mode\")").unwrap(),
        Value::Bool(true)
    );
    // var-local should not overwrite existing value
    bridge.eval("(var-local \"indent-tabs-mode\" false)").unwrap();
    assert_eq!(
        bridge.eval("(var-get \"indent-tabs-mode\")").unwrap(),
        Value::Bool(true)
    );
    take_editor_state();
}
// --- Region Tests ---
#[test]
fn region_operations() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    assert_eq!(bridge.eval("(region-active?)").unwrap(), Value::Bool(false));
    bridge.eval("(set-mark)").unwrap();
    assert_eq!(bridge.eval("(region-active?)").unwrap(), Value::Bool(true));
    // Region beginning should be at cursor (0,0)
    assert_eq!(bridge.eval("(region-beginning)").unwrap(), Value::Int(0));
    assert_eq!(bridge.eval("(region-end)").unwrap(), Value::Int(0));
    take_editor_state();
}
#[test]
fn delete_region_removes_text() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    // Set up content: "hello world"
    bridge.eval("(buffer-set-content \"hello world\")").unwrap();
    bridge.eval("(cursor-set! 0 5)").unwrap(); // cursor at space
    bridge.eval("(set-mark)").unwrap();
    bridge.eval("(cursor-set! 0 0)").unwrap(); // mark at start
    bridge.eval("(delete-region)").unwrap();
    assert_eq!(
        bridge.eval("(buffer-content)").unwrap(),
        Value::string(" world")
    );
    take_editor_state();
}
// --- Undo Tests ---
#[test]
fn undo_redo_cycle() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    // Record initial state
    bridge.eval("(undo-boundary)").unwrap();
    // Make a change and record
    bridge.eval("(buffer-set-content \"modified\")").unwrap();
    bridge.eval("(undo-boundary)").unwrap();
    // Undo should restore previous state (empty)
    let result = bridge.eval("(undo)").unwrap();
    assert_eq!(result, Value::Bool(true));
    assert_eq!(bridge.eval("(buffer-content)").unwrap(), Value::string(""));
    // Redo should restore the modification
    let result = bridge.eval("(redo)").unwrap();
    assert_eq!(result, Value::Bool(true));
    assert_eq!(bridge.eval("(buffer-content)").unwrap(), Value::string("modified"));
    // Undo again
    let result = bridge.eval("(undo)").unwrap();
    assert_eq!(result, Value::Bool(true));
    assert_eq!(bridge.eval("(buffer-content)").unwrap(), Value::string(""));
    // Make a DIFFERENT edit (creates branch instead of overwriting)
    bridge.eval("(buffer-set-content \"alternate\")").unwrap();
    bridge.eval("(undo-boundary)").unwrap();
    assert_eq!(
        bridge.eval("(buffer-content)").unwrap(),
        Value::string("alternate")
    );
    // Go back — should have 2 branches now
    bridge.eval("(undo)").unwrap();
    assert_eq!(
        bridge.eval("(undo-tree-branches)").unwrap(),
        Value::Int(2)
    );
    take_editor_state();
}
// --- Hook Extension Tests ---
#[test]
fn hook_query_operations() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    // Initially no hooks
    assert_eq!(bridge.eval("(hook-bound? \"after-save\")").unwrap(), Value::Bool(false));
    assert_eq!(bridge.eval("(hooks-for \"after-save\")").unwrap(), Value::Int(0));
    // Add a hook
    bridge.eval("(add-hook \"after-save\" (fn [] nil))").unwrap();
    assert_eq!(bridge.eval("(hook-bound? \"after-save\")").unwrap(), Value::Bool(true));
    assert_eq!(bridge.eval("(hooks-for \"after-save\")").unwrap(), Value::Int(1));
    // Remove by index
    bridge.eval("(remove-hook \"after-save\" 0)").unwrap();
    assert_eq!(bridge.eval("(hook-bound? \"after-save\")").unwrap(), Value::Bool(false));
    take_editor_state();
}
// --- Minibuffer Tests ---
#[test]
fn minibuffer_read_string() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    // In headless mode, read-string returns default
    let result = bridge
        .eval("(read-string \"Name: \" \"default-name\")")
        .unwrap();
    assert_eq!(result, Value::string("default-name"));
    // Without default, returns empty
    let result = bridge.eval("(read-string \"Query: \")").unwrap();
    assert_eq!(result, Value::string(""));
    take_editor_state();
}
#[test]
fn minibuffer_y_or_n() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    // In headless mode, y-or-n? defaults to true
    let result = bridge.eval("(y-or-n? \"Save? \")").unwrap();
    assert_eq!(result, Value::Bool(true));
    take_editor_state();
}
// --- Narrowing Tests ---
#[test]
fn narrow_and_widen() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    bridge.eval("(buffer-set-content \"line1\nline2\nline3\nline4\")").unwrap();
    assert_eq!(bridge.eval("(buffer-narrowed?)").unwrap(), Value::Bool(false));
    bridge.eval("(narrow-to-region 1 3)").unwrap();
    assert_eq!(bridge.eval("(buffer-narrowed?)").unwrap(), Value::Bool(true));
    bridge.eval("(widen)").unwrap();
    assert_eq!(bridge.eval("(buffer-narrowed?)").unwrap(), Value::Bool(false));
    take_editor_state();
}
// --- Text Search Tests ---
#[test]
fn search_forward_finds_pattern() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    bridge.eval("(buffer-set-content \"hello world foo bar\")").unwrap();
    bridge.eval("(cursor-set! 0 0)").unwrap();
    let result = bridge.eval("(search-forward \"world\")").unwrap();
    assert_eq!(result, Value::Int(6)); // "world" starts at col 6
    assert_eq!(bridge.eval("(cursor-col)").unwrap(), Value::Int(6));
    // Search for missing pattern returns nil
    bridge.eval("(cursor-set! 0 0)").unwrap();
    let result = bridge.eval("(search-forward \"notfound\")").unwrap();
    assert_eq!(result, Value::Nil);
    take_editor_state();
}
#[test]
fn search_backward_finds_pattern() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    bridge.eval("(buffer-set-content \"hello world hello\")").unwrap();
    bridge.eval("(cursor-set! 0 15)").unwrap(); // near end
    let result = bridge.eval("(search-backward \"hello\")").unwrap();
    assert_eq!(result, Value::Int(0)); // first "hello" at col 0
    take_editor_state();
}
#[test]
fn looking_at_checks_at_cursor() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    bridge.eval("(buffer-set-content \"hello world\")").unwrap();
    bridge.eval("(cursor-set! 0 0)").unwrap();
    assert_eq!(
        bridge.eval("(looking-at \"hello\")").unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        bridge.eval("(looking-at \"world\")").unwrap(),
        Value::Bool(false)
    );
    bridge.eval("(cursor-set! 0 6)").unwrap();
    assert_eq!(
        bridge.eval("(looking-at \"world\")").unwrap(),
        Value::Bool(true)
    );
    take_editor_state();
}
// --- Buffer List Test ---
#[test]
fn buffer_list_returns_current() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    let result = bridge.eval("(buffer-list)").unwrap();
    match result {
        Value::Vector(v) => {
            assert_eq!(v.len(), 1);
            assert_eq!(v[0], Value::string("*scratch*"));
        }
        _ => panic!("expected vector"),
    }
    take_editor_state();
}
// --- Integration: Emacs-like init.mora pattern ---
#[test]
fn emacs_like_init_config() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    // Simulate a user's init.mora using emacs-like primitives
    bridge
        .eval(
            r#"
                ;; Set buffer-local variables
                (var-set "tab-width" 4)
                (var-set "indent-tabs-mode" false)
                ;; Define a command using mark and region
                (defn delete-line
                  "Delete current line."
                  []
                  (interactive)
                  (set-mark)
                  (cursor-end-of-line)
                  (delete-region))
                ;; Set up hooks
                (add-hook "before-save"
                  (fn []
                    (editor-message (str "Saving: " (buffer-name)))))
                ;; Store a snippet in register
                (register-set "s" "fn main() {\n    \n}")
                ;; Push to kill ring
                (kill-ring-push "import std;")
                ;; Undo boundary before major change
                (undo-boundary)
                (buffer-set-content "new content")
                (undo-boundary)
                "#,
        )
        .unwrap();
    // Verify everything worked
    assert_eq!(bridge.eval("(var-get \"tab-width\")").unwrap(), Value::Int(4));
    assert_eq!(
        bridge.eval("(register-get \"s\")").unwrap(),
        Value::string("fn main() {\n    \n}")
    );
    assert_eq!(bridge.eval("(kill-ring-yank)").unwrap(), Value::string("import std;"));
    assert!(bridge.has_command("delete-line"));
    assert_eq!(bridge.eval("(hook-bound? \"before-save\")").unwrap(), Value::Bool(true));
    take_editor_state();
}
// --- Undo-Tree Tests ---
#[test]
fn undo_tree_branching_preserves_alternate_history() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    // Initial state
    bridge.eval("(undo-boundary)").unwrap();
    // Make edit A
    bridge.eval("(buffer-set-content \"A\")").unwrap();
    bridge.eval("(undo-boundary)").unwrap();
    // Make edit B
    bridge.eval("(buffer-set-content \"B\")").unwrap();
    bridge.eval("(undo-boundary)").unwrap();
    // Undo to A
    bridge.eval("(undo)").unwrap();
    assert_eq!(bridge.eval("(buffer-content)").unwrap(), Value::string("A"));
    // Make edit C (creates branch instead of destroying B)
    bridge.eval("(buffer-set-content \"C\")").unwrap();
    bridge.eval("(undo-boundary)").unwrap();
    assert_eq!(bridge.eval("(buffer-content)").unwrap(), Value::string("C"));
    // Go back to A — should have 2 branches
    bridge.eval("(undo)").unwrap();
    assert_eq!(bridge.eval("(buffer-content)").unwrap(), Value::string("A"));
    assert_eq!(
        bridge.eval("(undo-tree-branches)").unwrap(),
        Value::Int(2)
    );
    // Switch to branch 0
    bridge.eval("(undo-tree-switch-branch 0)").unwrap();
    let branch0 = bridge.eval("(buffer-content)").unwrap();
    // Go back, switch to branch 1
    bridge.eval("(undo)").unwrap();
    bridge.eval("(undo-tree-switch-branch 1)").unwrap();
    let branch1 = bridge.eval("(buffer-content)").unwrap();
    // Both branches accessible
    assert_ne!(branch0, branch1);
    take_editor_state();
}
#[test]
fn undo_tree_visualize_shows_structure() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    bridge.eval("(undo-boundary)").unwrap();
    bridge.eval("(buffer-set-content \"A\")").unwrap();
    bridge.eval("(undo-boundary)").unwrap();
    bridge.eval("(undo)").unwrap();
    bridge.eval("(buffer-set-content \"B\")").unwrap();
    bridge.eval("(undo-boundary)").unwrap();
    let vis = bridge.eval("(undo-tree-visualize)").unwrap();
    match vis {
        Value::String(s) => {
            assert!(s.contains("●"), "should show active node");
            assert!(s.contains("○"), "should show inactive nodes");
        }
        _ => panic!("expected string"),
    }
    let count = bridge.eval("(undo-tree-node-count)").unwrap();
    // root + boundary-of-A + A + boundary-of-B + B = 5 nodes
    // (each boundary records a new node in tree)
    assert!(matches!(count, Value::Int(n) if n >= 3));
    take_editor_state();
}
#[test]
fn undo_tree_can_undo_redo() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    // Fresh tree at root — nothing to undo
    assert_eq!(
        bridge.eval("(undo-tree-can-undo?)").unwrap(),
        Value::Bool(false)
    );
    assert_eq!(
        bridge.eval("(undo-tree-can-redo?)").unwrap(),
        Value::Bool(false)
    );
    // Record an edit
    bridge.eval("(undo-boundary)").unwrap();
    bridge.eval("(buffer-set-content \"edit\")").unwrap();
    bridge.eval("(undo-boundary)").unwrap();
    assert_eq!(
        bridge.eval("(undo-tree-can-undo?)").unwrap(),
        Value::Bool(true)
    );
    // Undo to previous state
    bridge.eval("(undo)").unwrap();
    assert_eq!(
        bridge.eval("(undo-tree-can-redo?)").unwrap(),
        Value::Bool(true)
    );
    take_editor_state();
}
// --- TRAMP Tests ---
#[test]
fn tramp_parse_path_works() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    let result = bridge
        .eval(r#"(tramp-parse-path "/ssh:user@host:/home/user/file.txt")"#)
        .unwrap();
    match result {
        Value::Map(m) => {
            let method = m.get(&Value::keyword("method")).unwrap();
            assert_eq!(*method, Value::string("ssh"));
            let host = m.get(&Value::keyword("host")).unwrap();
            assert_eq!(*host, Value::string("host"));
            let user = m.get(&Value::keyword("user")).unwrap();
            assert_eq!(*user, Value::string("user"));
            let path = m.get(&Value::keyword("path")).unwrap();
            assert_eq!(*path, Value::string("/home/user/file.txt"));
        }
        _ => panic!("expected map"),
    }
    take_editor_state();
}
#[test]
fn tramp_parse_path_with_port() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    let result = bridge
        .eval(r#"(tramp-parse-path "/ssh:admin@server#2222:/etc/config")"#)
        .unwrap();
    match result {
        Value::Map(m) => {
            let port = m.get(&Value::keyword("port")).unwrap();
            assert_eq!(*port, Value::Int(2222));
        }
        _ => panic!("expected map"),
    }
    take_editor_state();
}
#[test]
fn tramp_parse_path_no_user() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    let result = bridge
        .eval(r#"(tramp-parse-path "/scp:example.com:/tmp/data")"#)
        .unwrap();
    match result {
        Value::Map(m) => {
            assert_eq!(*m.get(&Value::keyword("method")).unwrap(), Value::string("scp"));
            assert!(!m.contains_key(&Value::keyword("user")));
        }
        _ => panic!("expected map"),
    }
    take_editor_state();
}
#[test]
fn tramp_connections_empty_initially() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    let result = bridge.eval("(tramp-connections)").unwrap();
    match result {
        Value::Vector(v) => assert!(v.is_empty()),
        _ => panic!("expected vector"),
    }
    take_editor_state();
}
#[test]
fn tramp_invalid_path_errors() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    let result = bridge.eval(r#"(tramp-parse-path "/not-a-tramp-path")"#);
    assert!(result.is_err());
    let result = bridge.eval(r#"(tramp-parse-path "/ssh:")"#);
    assert!(result.is_err());
    take_editor_state();
}
#[test]
fn describe_function_returns_doc() {
    set_editor_state(EditorState::new());
    let mut bridge = MoraLispBridge::new();
    // Describe a known function with doc
    let doc = bridge.eval("(describe-function \"buffer-name\")").unwrap();
    match doc {
        Value::String(s) => {
            assert!(s.contains("buffer"), "doc should mention buffer: {}", s);
        }
        _ => panic!("expected string, got {:?}", doc),
    }
    // Describe cursor function
    let doc = bridge.eval("(describe-function \"cursor-set!\")").unwrap();
    match doc {
        Value::String(s) => {
            assert!(s.contains("cursor") || s.contains("Cursor"), "doc should mention cursor: {}", s);
        }
        _ => panic!("expected string, got {:?}", doc),
    }
    // Unknown function returns nil
    let result = bridge.eval("(describe-function \"nonexistent-function\")").unwrap();
    assert_eq!(result, Value::Nil);
    take_editor_state();
}
    // --- Editing Features ---
    #[test]
    fn expand_region_works() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        bridge.eval("(buffer-set-content \"hello world\")").unwrap();
        bridge.eval("(cursor-set! 0 3)").unwrap(); // cursor in "hello"
        // Expand to word
        bridge.eval("(expand-region)").unwrap();
        assert_eq!(bridge.eval("(mark-active?)").unwrap(), Value::Bool(true));
        // Expand to line
        bridge.eval("(expand-region)").unwrap();
        // Contract back
        bridge.eval("(contract-region)").unwrap();
        assert_eq!(bridge.eval("(mark-active?)").unwrap(), Value::Bool(true));
        take_editor_state();
    }
    #[test]
    fn hungry_delete_removes_whitespace() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        bridge.eval("(buffer-set-content \"hello   world\")").unwrap();
        bridge.eval("(cursor-set! 0 5)").unwrap(); // cursor after "hello", in spaces
        bridge.eval("(hungry-delete-forward)").unwrap();
        assert_eq!(
            bridge.eval("(buffer-content)").unwrap(),
            Value::string("helloworld")
        );
        take_editor_state();
    }
    #[test]
    fn cleanup_buffer_removes_trailing_whitespace() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        bridge.eval("(buffer-set-content \"hello   \nworld\t\n\")").unwrap();
        bridge.eval("(cleanup-buffer)").unwrap();
        let content = bridge.eval("(buffer-content)").unwrap();
        match content {
            Value::String(s) => {
                // Trailing whitespace should be removed
                assert!(!s.contains("hello   "), "trailing spaces removed");
            }
            _ => panic!("expected string"),
        }
        take_editor_state();
    }
    #[test]
    fn insert_empty_line_adds_line() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        bridge.eval("(buffer-set-content \"line1\nline2\")").unwrap();
        bridge.eval("(cursor-set! 0 0)").unwrap();
        bridge.eval("(insert-empty-line)").unwrap();
        let count = bridge.eval("(buffer-line-count)").unwrap();
        assert_eq!(count, Value::Int(3));
        take_editor_state();
    }
    // --- History Features ---
    #[test]
    fn recentf_operations() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        assert_eq!(
            bridge.eval("(recentf-list)").unwrap(),
            Value::vector(vec![])
        );
        bridge.eval("(recentf-add \"/home/user/file1.txt\")").unwrap();
        bridge.eval("(recentf-add \"/home/user/file2.rs\")").unwrap();
        let list = bridge.eval("(recentf-list)").unwrap();
        match list {
            Value::Vector(v) => assert_eq!(v.len(), 2),
            _ => panic!("expected vector"),
        }
        bridge.eval("(recentf-clear)").unwrap();
        assert_eq!(
            bridge.eval("(recentf-list)").unwrap(),
            Value::vector(vec![])
        );
        take_editor_state();
    }
    // --- Visual Features ---
    #[test]
    fn which_key_for_prefix() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        // Define some keybindings under a prefix
        bridge.eval("(define-key \"C-x C-f\" \"find-file\")").unwrap();
        bridge.eval("(define-key \"C-x C-s\" \"save-buffer\")").unwrap();
        bridge.eval("(define-key \"C-x b\" \"switch-to-buffer\")").unwrap();
        // Query bindings under C-x
        let result = bridge.eval("(which-key-for-prefix \"C-x\")").unwrap();
        match result {
            Value::Vector(v) => {
                assert!(v.len() >= 3, "should have at least 3 C-x bindings");
            }
            _ => panic!("expected vector"),
        }
        take_editor_state();
    }
    #[test]
    fn query_replace_pattern() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        bridge.eval("(buffer-set-content \"hello world hello\")").unwrap();
        bridge.eval("(query-replace-pattern \"hello\" \"hi\")").unwrap();
        assert_eq!(
            bridge.eval("(buffer-content)").unwrap(),
            Value::string("hi world hi")
        );
        take_editor_state();
    }
    #[test]
    fn focus_mode_toggle() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        assert_eq!(bridge.eval("(focus-mode?)").unwrap(), Value::Bool(false));
        bridge.eval("(focus-mode-toggle)").unwrap();
        assert_eq!(bridge.eval("(focus-mode?)").unwrap(), Value::Bool(true));
        bridge.eval("(focus-mode-toggle)").unwrap();
        assert_eq!(bridge.eval("(focus-mode?)").unwrap(), Value::Bool(false));
        take_editor_state();
    }
    // --- Smartparens ---

    #[test]
    fn smartparens_wrap_region() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();

        bridge.eval("(buffer-set-content \"hello world\")").unwrap();
        bridge.eval("(cursor-set! 0 5)").unwrap();
        bridge.eval("(set-mark)").unwrap();
        bridge.eval("(cursor-set! 0 0)").unwrap();
        bridge.eval("(smartparens-wrap \"(\" \")\")").unwrap();
        let content = bridge.eval("(buffer-content)").unwrap();
        match content {
            Value::String(s) => assert_eq!(*s, "(hello) world"),
            _ => panic!("expected string"),
        }
        take_editor_state();
    }

    #[test]
    fn smartparens_insert_pair() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();

        bridge.eval("(buffer-set-content \"hello\")").unwrap();
        bridge.eval("(cursor-set! 0 5)").unwrap();
        bridge.eval("(smartparens-insert-pair \"[\" \"]\")").unwrap();
        let content = bridge.eval("(buffer-content)").unwrap();
        match content {
            Value::String(s) => assert_eq!(*s, "hello[]"),
            _ => panic!("expected string"),
        }
        take_editor_state();
    }

    #[test]
    fn smartparens_unwrap() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();

        bridge.eval("(buffer-set-content \"(hello)\")").unwrap();
        bridge.eval("(cursor-set! 0 3)").unwrap();
        bridge.eval("(smartparens-unwrap)").unwrap();
        let content = bridge.eval("(buffer-content)").unwrap();
        match content {
            Value::String(s) => assert_eq!(*s, "hello"),
            _ => panic!("expected string"),
        }
        take_editor_state();
    }

    // --- Leader Key ---

    #[test]
    fn leader_set_key_and_bindings() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();

        bridge.eval("(leader-set-key \"f\" \"find-file\")").unwrap();
        bridge.eval("(leader-set-key \"s\" \"save-buffer\")").unwrap();
        let bindings = bridge.eval("(leader-bindings)").unwrap();
        match bindings {
            Value::Vector(v) => assert!(v.len() >= 2),
            _ => panic!("expected vector"),
        }
        take_editor_state();
    }

    // --- Grep ---

    #[test]
    fn grep_finds_pattern() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();

        let result = bridge.eval(r#"(grep "Cargo.toml" "/data/Workspace/serana-new")"#).unwrap();
        match result {
            Value::String(s) => assert!(s.contains("Cargo.toml")),
            _ => panic!("expected string"),
        }
        take_editor_state();
    }

    // --- Org ---

    #[test]
    fn org_heading_level() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();

        bridge.eval("(buffer-set-content \"* Heading 1\n** Heading 2\n*** Heading 3\")").unwrap();
        bridge.eval("(cursor-set! 0 0)").unwrap();
        assert_eq!(bridge.eval("(org-heading-level)").unwrap(), Value::Int(1));
        bridge.eval("(cursor-set! 1 0)").unwrap();
        assert_eq!(bridge.eval("(org-heading-level)").unwrap(), Value::Int(2));
        bridge.eval("(cursor-set! 2 0)").unwrap();
        assert_eq!(bridge.eval("(org-heading-level)").unwrap(), Value::Int(3));
        take_editor_state();
    }

    #[test]
    fn org_todo_state() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();

        bridge.eval("(buffer-set-content \"* TODO Fix bug\n* DONE Write tests\")").unwrap();
        bridge.eval("(cursor-set! 0 0)").unwrap();
        assert_eq!(bridge.eval("(org-todo-state)").unwrap(), Value::string("TODO"));
        bridge.eval("(cursor-set! 1 0)").unwrap();
        assert_eq!(bridge.eval("(org-todo-state)").unwrap(), Value::string("DONE"));
        take_editor_state();
    }

    #[test]
    fn org_agenda_lists_todos() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();

        bridge.eval("(buffer-set-content \"* TODO A\n* DONE B\n* TODO C\")").unwrap();
        let agenda = bridge.eval("(org-agenda-list)").unwrap();
        match agenda {
            Value::Vector(v) => assert_eq!(v.len(), 3),
            _ => panic!("expected vector"),
        }
        take_editor_state();
    }

    // --- Session ---

    #[test]
    fn session_save_and_load() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();

        bridge.eval("(buffer-set-content \"test content\")").unwrap();
        bridge.eval("(cursor-set! 0 4)").unwrap();

        let path = "/tmp/mora_test_session.mora";
        bridge.eval(&format!("(session-save \"{}\")", path)).unwrap();

        set_editor_state(EditorState::new());
        let mut bridge2 = MoraLispBridge::new();
        bridge2.eval(&format!("(session-load \"{}\")", path)).unwrap();

        let content = bridge2.eval("(buffer-content)").unwrap();
        match content {
            Value::String(s) => assert_eq!(*s, "test content"),
            _ => panic!("expected string"),
        }
        take_editor_state();
    }
