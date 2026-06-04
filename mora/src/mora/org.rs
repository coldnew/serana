/// Basic org-mode support for mora.
///
/// Provides heading navigation, folding, TODO state toggling,
/// and code block evaluation (babel-like).
///
/// Org syntax recognized:
///   * Heading level 1
///   ** Heading level 2
///   #+BEGIN_SRC lang ... #+END_SRC — executable code blocks
///   - [ ] TODO item
///   - [X] DONE item

/// An org heading parsed from buffer text.
#[derive(Debug, Clone)]
pub struct OrgHeading {
    pub level: usize,
    pub title: String,
    pub todo_state: Option<TodoState>,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TodoState {
    Todo,
    Done,
}

/// A code block in the org file.
#[derive(Debug, Clone)]
pub struct OrgCodeBlock {
    pub language: String,
    pub body: String,
    pub begin_line: usize,
    pub end_line: usize,
}

/// Parse all headings from buffer lines.
pub fn parse_headings(lines: &[String]) -> Vec<OrgHeading> {
    let mut headings = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        // Count leading stars
        let mut level = 0;
        for ch in trimmed.chars() {
            if ch == '*' {
                level += 1;
            } else {
                break;
            }
        }
        if level == 0 {
            continue;
        }
        // Must have a space after stars
        if trimmed.len() <= level || !trimmed.as_bytes()[level].is_ascii_whitespace() {
            continue;
        }
        let title = trimmed[level..].trim().to_string();

        // Check for TODO state
        let (todo_state, title) = if title.starts_with("TODO ") {
            (Some(TodoState::Todo), title[5..].to_string())
        } else if title.starts_with("DONE ") {
            (Some(TodoState::Done), title[5..].to_string())
        } else {
            (None, title)
        };

        headings.push(OrgHeading {
            level,
            title,
            todo_state,
            line: i,
        });
    }
    headings
}

/// Parse all code blocks from buffer lines.
pub fn parse_code_blocks(lines: &[String]) -> Vec<OrgCodeBlock> {
    let mut blocks = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.to_uppercase().starts_with("#+BEGIN_SRC") {
            let lang = trimmed["#+BEGIN_SRC".len()..].trim().to_string();
            let begin = i;
            let mut body_lines = Vec::new();
            i += 1;
            while i < lines.len() {
                if lines[i].trim().to_uppercase().starts_with("#+END_SRC") {
                    break;
                }
                body_lines.push(lines[i].clone());
                i += 1;
            }
            let end = i;
            blocks.push(OrgCodeBlock {
                language: lang,
                body: body_lines.join("\n"),
                begin_line: begin,
                end_line: end,
            });
        }
        i += 1;
    }
    blocks
}

/// Toggle TODO state of a heading at the given line.
pub fn toggle_todo(lines: &mut [String], line: usize) -> bool {
    if line >= lines.len() {
        return false;
    }
    let l = &lines[line];
    let trimmed = l.trim_start();
    // Must start with *
    let mut level = 0;
    for ch in trimmed.chars() {
        if ch == '*' {
            level += 1;
        } else {
            break;
        }
    }
    if level == 0 || trimmed.len() <= level {
        return false;
    }
    let indent = &l[..l.len() - trimmed.len()];
    let after_stars = trimmed[level..].trim_start();

    let new_title = if after_stars.starts_with("TODO ") {
        format!("{}{} DONE {}", indent, "*".repeat(level), &after_stars[5..])
    } else if after_stars.starts_with("DONE ") {
        format!("{}{} {}", indent, "*".repeat(level), &after_stars[5..])
    } else {
        format!("{}{} TODO {}", indent, "*".repeat(level), after_stars)
    };
    lines[line] = new_title;
    true
}

/// Execute a code block body via sh -c.
pub fn execute_code_block(block: &OrgCodeBlock) -> Result<String, String> {
    match std::process::Command::new("sh")
        .arg("-c")
        .arg(&block.body)
        .output()
    {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            if !out.status.success() && !stderr.is_empty() {
                Ok(format!("{}\nstderr: {}", stdout, stderr))
            } else {
                Ok(stdout.into_owned())
            }
        }
        Err(e) => Err(format!("Execution error: {e}")),
    }
}

/// Jump to next heading from current line.
pub fn next_heading(headings: &[OrgHeading], current_line: usize) -> Option<usize> {
    headings
        .iter()
        .find(|h| h.line > current_line)
        .map(|h| h.line)
}

/// Jump to previous heading from current line.
pub fn prev_heading(headings: &[OrgHeading], current_line: usize) -> Option<usize> {
    headings
        .iter()
        .rev()
        .find(|h| h.line < current_line)
        .map(|h| h.line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_headings() {
        let lines: Vec<String> = vec![
            "* Heading 1".into(),
            "Some text".into(),
            "** Heading 2".into(),
            "*** TODO Heading 3".into(),
            "*** DONE Heading 4".into(),
        ];
        let headings = parse_headings(&lines);
        assert_eq!(headings.len(), 4);
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[0].title, "Heading 1");
        assert_eq!(headings[2].todo_state, Some(TodoState::Todo));
        assert_eq!(headings[3].todo_state, Some(TodoState::Done));
    }

    #[test]
    fn test_parse_code_blocks() {
        let lines: Vec<String> = vec![
            "#+BEGIN_SRC rust".into(),
            "fn main() {}".into(),
            "#+END_SRC".into(),
            "text between".into(),
            "#+BEGIN_SRC python".into(),
            "print('hello')".into(),
            "#+END_SRC".into(),
        ];
        let blocks = parse_code_blocks(&lines);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].language, "rust");
        assert_eq!(blocks[1].language, "python");
    }

    #[test]
    fn test_toggle_todo() {
        let mut lines: Vec<String> = vec![
            "* TODO Buy milk".into(),
            "** DONE Write code".into(),
            "* Plain heading".into(),
        ];
        assert!(toggle_todo(&mut lines, 0));
        assert!(lines[0].contains("DONE"));
        assert!(toggle_todo(&mut lines, 0));
        assert!(!lines[0].contains("TODO"));
        assert!(!lines[0].contains("DONE"));
        assert!(toggle_todo(&mut lines, 1));
        assert!(lines[1].contains("TODO"));
    }
}
