/// Emacs-like undo-tree.
///
/// Unlike linear undo stacks, undo-tree preserves all branches.
/// When you undo then make a new edit, the old redo branch is kept
/// as an alternate history you can switch back to.

/// A snapshot of editor state at one point in history.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
}

/// A node in the undo tree. Each node has one parent and N children.
#[derive(Debug, Clone)]
struct Node {
    snapshot: Snapshot,
    parent: Option<usize>,
    children: Vec<usize>,
}

/// Undo-tree: a tree of editor snapshots with branching history.
#[derive(Debug)]
pub struct UndoTree {
    nodes: Vec<Option<Node>>,
    active: usize,
    next_id: usize,
}

impl UndoTree {
    pub fn new(initial: Snapshot) -> Self {
        let root = Node {
            snapshot: initial,
            parent: None,
            children: Vec::new(),
        };
        Self {
            nodes: vec![Some(root)],
            active: 0,
            next_id: 1,
        }
    }

    /// Record current state as a child of the active node.
    /// If active already has a child matching this snapshot, reuse it.
    /// Otherwise create a new branch.
    pub fn record(&mut self, snapshot: Snapshot) {
        // Check if active already has a child with identical snapshot
        let active = self.active;
        if let Some(node) = &self.nodes[active] {
            for &child_id in &node.children {
                if let Some(child) = &self.nodes[child_id] {
                    if child.snapshot.lines == snapshot.lines {
                        self.active = child_id;
                        return;
                    }
                }
            }
        }

        // Create new child node
        let id = self.next_id;
        self.next_id += 1;

        let node = Node {
            snapshot,
            parent: Some(active),
            children: Vec::new(),
        };

        // Grow nodes vec to fit
        while self.nodes.len() <= id {
            self.nodes.push(None);
        }
        self.nodes[id] = Some(node);

        // Link as child of active
        if let Some(parent) = &mut self.nodes[active] {
            parent.children.push(id);
        }

        self.active = id;
    }

    /// Move to parent (undo). Returns true if moved.
    pub fn undo(&mut self) -> bool {
        if let Some(node) = &self.nodes[self.active] {
            if let Some(parent_id) = node.parent {
                self.active = parent_id;
                return true;
            }
        }
        false
    }

    /// Move to most recent child (redo). Returns true if moved.
    pub fn redo(&mut self) -> bool {
        if let Some(node) = &self.nodes[self.active] {
            if let Some(&last_child) = node.children.last() {
                self.active = last_child;
                return true;
            }
        }
        false
    }

    /// Switch to branch N at the current node. Returns true if successful.
    pub fn switch_branch(&mut self, n: usize) -> bool {
        if let Some(node) = &self.nodes[self.active] {
            if let Some(&child_id) = node.children.get(n) {
                self.active = child_id;
                return true;
            }
        }
        false
    }

    /// Get the snapshot at the active node.
    pub fn current(&self) -> &Snapshot {
        &self.nodes[self.active].as_ref().unwrap().snapshot
    }

    /// Number of children (branches forward) at the current node.
    pub fn branch_count(&self) -> usize {
        self.nodes[self.active]
            .as_ref()
            .map_or(0, |n| n.children.len())
    }

    /// Can we undo from the current position?
    pub fn can_undo(&self) -> bool {
        self.nodes[self.active]
            .as_ref()
            .map_or(false, |n| n.parent.is_some())
    }

    /// Can we redo from the current position?
    pub fn can_redo(&self) -> bool {
        self.nodes[self.active]
            .as_ref()
            .map_or(false, |n| !n.children.is_empty())
    }

    /// Total number of nodes in the tree.
    pub fn node_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_some()).count()
    }

    /// Serialize the tree structure for visualization.
    pub fn dump(&self) -> Vec<(usize, bool, usize)> {
        let mut result = Vec::new();
        self.dump_node(0, 0, &mut result);
        result
    }

    fn dump_node(&self, id: usize, depth: usize, out: &mut Vec<(usize, bool, usize)>) {
        if let Some(node) = &self.nodes[id] {
            out.push((depth, id == self.active, node.snapshot.lines.len()));
            for &child in &node.children {
                self.dump_node(child, depth + 1, out);
            }
        }
    }

    /// Format the tree as a human-readable string for visualization.
    pub fn visualize(&self) -> String {
        let mut buf = String::new();
        self.fmt_node(0, "", true, &mut buf);
        buf
    }

    fn fmt_node(&self, id: usize, prefix: &str, is_last: bool, buf: &mut String) {
        if let Some(node) = &self.nodes[id] {
            let marker = if id == self.active { "●" } else { "○" };
            let preview = node
                .snapshot
                .lines
                .first()
                .map(|s| s.as_str())
                .unwrap_or("");
            let label = format!(
                "{} {} [{} lines]",
                marker,
                preview,
                node.snapshot.lines.len()
            );
            buf.push_str(prefix);
            buf.push_str(if is_last { "└── " } else { "├── " });
            buf.push_str(&label);
            buf.push('\n');

            let child_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
            let child_count = node.children.len();
            for (i, &child) in node.children.iter().enumerate() {
                self.fmt_node(child, &child_prefix, i == child_count - 1, buf);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(lines: &[&str]) -> Snapshot {
        Snapshot {
            lines: lines.iter().map(|s| s.to_string()).collect(),
            cursor_row: 0,
            cursor_col: 0,
        }
    }

    #[test]
    fn test_basic_undo_redo() {
        let mut tree = UndoTree::new(snap(&["initial"]));
        assert_eq!(tree.current().lines, vec!["initial"]);

        tree.record(snap(&["edit1"]));
        assert_eq!(tree.current().lines, vec!["edit1"]);

        tree.record(snap(&["edit2"]));
        assert_eq!(tree.current().lines, vec!["edit2"]);

        // Undo goes to edit1
        assert!(tree.undo());
        assert_eq!(tree.current().lines, vec!["edit1"]);

        // Undo goes to initial
        assert!(tree.undo());
        assert_eq!(tree.current().lines, vec!["initial"]);

        // Undo at root returns false
        assert!(!tree.undo());
        assert_eq!(tree.current().lines, vec!["initial"]);

        // Redo goes to edit1
        assert!(tree.redo());
        assert_eq!(tree.current().lines, vec!["edit1"]);

        // Redo goes to edit2
        assert!(tree.redo());
        assert_eq!(tree.current().lines, vec!["edit2"]);

        // Redo at leaf returns false
        assert!(!tree.redo());
    }

    #[test]
    fn test_branching() {
        let mut tree = UndoTree::new(snap(&["initial"]));

        // Make edits A -> B
        tree.record(snap(&["A"]));
        tree.record(snap(&["B"]));

        // Undo to A
        tree.undo();
        assert_eq!(tree.current().lines, vec!["A"]);

        // Make a different edit C (creates a branch)
        tree.record(snap(&["C"]));
        assert_eq!(tree.current().lines, vec!["C"]);

        // The old branch (A->B) is preserved
        // Go back to A
        tree.undo();
        assert_eq!(tree.current().lines, vec!["A"]);
        assert_eq!(tree.branch_count(), 2);

        // Switch to branch 0
        tree.switch_branch(0);
        let first = tree.current().lines[0].clone();
        tree.undo(); // back to A
        tree.switch_branch(1);
        let second = tree.current().lines[0].clone();

        // Both branches should be accessible
        assert_ne!(first, second);
        assert!(
            (first == "B" && second == "C") || (first == "C" && second == "B"),
            "expected B and C, got {} and {}",
            first,
            second
        );
    }

    #[test]
    fn test_reuse_identical_state() {
        let mut tree = UndoTree::new(snap(&["initial"]));
        tree.record(snap(&["edit"]));
        assert_eq!(tree.node_count(), 2); // root + edit

        tree.undo();
        // Record same edit again — should reuse existing child
        tree.record(snap(&["edit"]));
        assert_eq!(tree.node_count(), 2); // still just root + edit (reused)
    }

    #[test]
    fn test_switch_branch() {
        let mut tree = UndoTree::new(snap(&["root"]));
        tree.record(snap(&["A"]));
        tree.undo();
        tree.record(snap(&["B"]));
        tree.undo();
        tree.record(snap(&["C"]));

        // At root's child C
        assert_eq!(tree.current().lines, vec!["C"]);
        // Root has 3 children: A, B, C
        tree.undo();
        assert_eq!(tree.branch_count(), 3);
        assert!(tree.switch_branch(0));
        assert!(tree.undo());
        assert!(tree.switch_branch(2));
        assert_eq!(tree.current().lines, vec!["C"]);
    }

    #[test]
    fn test_can_undo_redo() {
        let mut tree = UndoTree::new(snap(&["root"]));
        assert!(!tree.can_undo());
        assert!(!tree.can_redo());

        tree.record(snap(&["edit"]));
        assert!(tree.can_undo());
        assert!(!tree.can_redo());

        tree.undo();
        assert!(!tree.can_undo());
        assert!(tree.can_redo());
    }

    #[test]
    fn test_visualize() {
        let mut tree = UndoTree::new(snap(&["root"]));
        tree.record(snap(&["A"]));
        tree.record(snap(&["B"]));
        tree.undo();
        tree.record(snap(&["C"]));

        let vis = tree.visualize();
        assert!(vis.contains("●"), "should contain active marker");
        assert!(vis.contains("○"), "should contain inactive marker");
        assert!(vis.contains("root"), "should show root content");
    }

    #[test]
    fn test_three_way_branch() {
        let mut tree = UndoTree::new(snap(&["start"]));

        // Branch 1: start -> X
        tree.record(snap(&["X"]));
        assert_eq!(tree.current().lines, vec!["X"]);

        // Undo, create branch 2: start -> A
        tree.undo();
        tree.record(snap(&["A"]));
        assert_eq!(tree.current().lines, vec!["A"]);

        // Undo, create branch 3: start -> P
        tree.undo();
        tree.record(snap(&["P"]));
        assert_eq!(tree.current().lines, vec!["P"]);

        // All 3 branches from root should exist
        tree.undo(); // back to start
        assert_eq!(tree.branch_count(), 3);

        // Navigate each branch
        tree.switch_branch(0);
        let v0 = tree.current().lines[0].clone();
        tree.undo();
        tree.switch_branch(1);
        let v1 = tree.current().lines[0].clone();
        tree.undo();
        tree.switch_branch(2);
        let v2 = tree.current().lines[0].clone();

        let mut vals = vec![v0, v1, v2];
        vals.sort();
        assert_eq!(vals, vec!["A", "P", "X"]);
    }
}
