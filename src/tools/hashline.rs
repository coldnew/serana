//! Hashline edit system - line-anchored patch language
//!
//! This module implements Oh My Pi's hashline patch format for safe file edits.
//! Lines are identified by `<line_number><2-char-hash>` anchors (e.g., `41th`).
//!
//! ## Format
//! - Section header: `@@ PATH`
//! - Insert after: `+ ANCHOR`
//! - Insert before: `< ANCHOR`
//! - Delete range: `- A..B`
//! - Replace range: `= A..B`
//! - Payload line: `~TEXT`
//! - Special anchors: `BOF`, `EOF`

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{anyhow, bail, Result};


/// Maximum number of paths cached per session for stale-anchor recovery
const MAX_CACHED_PATHS: usize = 30;

/// Hash function constants
const HL_BIGRAMS: &[&str] = &[
    "aa", "ab", "ac", "ad", "ae", "af", "ag", "ah", "ai", "aj",
    "ak", "al", "am", "an", "ao", "ap", "aq", "ar", "as", "at",
    "au", "av", "aw", "ax", "ay", "az", "ba", "bb", "bc", "bd",
    "be", "bf", "bg", "bh", "bi", "bj", "bk", "bl", "bm", "bn",
    "bo", "bp", "bq", "br", "bs", "bt", "bu", "bv", "bw", "bx",
    "by", "bz", "ca", "cb", "cc", "cd", "ce", "cf", "cg", "ch",
    "ci", "cj", "ck", "cl", "cm", "cn", "co", "cp", "cq", "cr",
    "cs", "ct", "cu", "cv", "cw", "cx", "cy", "cz", "da", "db",
    "dc", "dd", "de", "df", "dg", "dh", "di", "dj", "dk", "dl",
    "dm", "dn", "do", "dp", "dq", "dr", "ds", "dt", "du", "dv",
    "dw", "dx", "dy", "dz", "ea", "eb", "ec", "ed", "ee", "ef",
    "eg", "eh", "ei", "ej", "ek", "el", "em", "en", "eo", "ep",
    "eq", "er", "es", "et", "eu", "ev", "ew", "ex", "ey", "ez",
    "fa", "fb", "fc", "fd", "fe", "ff", "fg", "fh", "fi", "fj",
    "fk", "fl", "fm", "fn", "fo", "fp", "fq", "fr", "fs", "ft",
    "fu", "fv", "fw", "fx", "fy", "fz", "ga", "gb", "gc", "gd",
    "ge", "gf", "gg", "gh", "gi", "gj", "gk", "gl", "gm", "gn",
    "go", "gp", "gq", "gr", "gs", "gt", "gu", "gv", "gw", "gx",
    "gy", "gz", "ha", "hb", "hc", "hd", "he", "hf", "hg", "hh",
    "hi", "hj", "hk", "hl", "hm", "hn", "ho", "hp", "hq", "hr",
    "hs", "ht", "hu", "hv", "hw", "hx", "hy", "hz", "ia", "ib",
    "ic", "id", "ie", "if", "ig", "ih", "ii", "ij", "ik", "il",
    "im", "in", "io", "ip", "iq", "ir", "is", "it", "iu", "iv",
    "iw", "ix", "iy", "iz", "ja", "jb", "jc", "jd", "je", "jf",
    "jg", "jh", "ji", "jj", "jk", "jl", "jm", "jn", "jo", "jp",
    "jq", "jr", "js", "jt", "ju", "jv", "jw", "jx", "jy", "jz",
    "ka", "kb", "kc", "kd", "ke", "kf", "kg", "kh", "ki", "kj",
    "kk", "kl", "km", "kn", "ko", "kp", "kq", "kr", "ks", "kt",
    "ku", "kv", "kw", "kx", "ky", "kz", "la", "lb", "lc", "ld",
    "le", "lf", "lg", "lh", "li", "lj", "lk", "ll", "lm", "ln",
    "lo", "lp", "lq", "lr", "ls", "lt", "lu", "lv", "lw", "lx",
    "ly", "lz", "ma", "mb", "mc", "md", "me", "mf", "mg", "mh",
    "mi", "mj", "mk", "ml", "mm", "mn", "mo", "mp", "mq", "mr",
    "ms", "mt", "mu", "mv", "mw", "mx", "my", "mz", "na", "nb",
    "nc", "nd", "ne", "nf", "ng", "nh", "ni", "nj", "nk", "nl",
    "nm", "nn", "no", "np", "nq", "nr", "ns", "nt", "nu", "nv",
    "nw", "nx", "ny", "nz", "oa", "ob", "oc", "od", "oe", "of",
    "og", "oh", "oi", "oj", "ok", "ol", "om", "on", "oo", "op",
    "oq", "or", "os", "ot", "ou", "ov", "ow", "ox", "oy", "oz",
    "pa", "pb", "pc", "pd", "pe", "pf", "pg", "ph", "pi", "pj",
    "pk", "pl", "pm", "pn", "po", "pp", "pq", "pr", "ps", "pt",
    "pu", "pv", "pw", "px", "py", "pz", "qa", "qb", "qc", "qd",
    "qe", "qf", "qg", "qh", "qi", "qj", "qk", "ql", "qm", "qn",
    "qo", "qp", "qq", "qr", "qs", "qt", "qu", "qv", "qw", "qx",
    "qy", "qz", "ra", "rb", "rc", "rd", "re", "rf", "rg", "rh",
    "ri", "rj", "rk", "rl", "rm", "rn", "ro", "rp", "rq", "rr",
    "rs", "rt", "ru", "rv", "rw", "rx", "ry", "rz", "sa", "sb",
    "sc", "sd", "se", "sf", "sg", "sh", "si", "sj", "sk", "sl",
    "sm", "sn", "so", "sp", "sq", "sr", "ss", "st", "su", "sv",
    "sw", "sx", "sy", "sz", "ta", "tb", "tc", "td", "te", "tf",
    "tg", "th", "ti", "tj", "tk", "tl", "tm", "tn", "to", "tp",
    "tq", "tr", "ts", "tt", "tu", "tv", "tw", "tx", "ty", "tz",
    "ua", "ub", "uc", "ud", "ue", "uf", "ug", "uh", "ui", "uj",
    "uk", "ul", "um", "un", "uo", "up", "uq", "ur", "us", "ut",
    "uu", "uv", "uw", "ux", "uy", "uz", "va", "vb", "vc", "vd",
    "ve", "vf", "vg", "vh", "vi", "vj", "vk", "vl", "vm", "vn",
    "vo", "vp", "vq", "vr", "vs", "vt", "vu", "vv", "vw", "vx",
    "vy", "vz", "wa", "wb", "wc", "wd", "we", "wf", "wg", "wh",
    "wi", "wj", "wk", "wl", "wm", "wn", "wo", "wp", "wq", "wr",
    "ws", "wt", "wu", "wv", "ww", "wx", "wy", "wz", "xa", "xb",
    "xc", "xd", "xe", "xf", "xg", "xh", "xi", "xj", "xk", "xl",
    "xm", "xn", "xo", "xp", "xq", "xr", "xs", "xt", "xu", "xv",
    "xw", "xx", "xy", "xz", "ya", "yb", "yc", "yd", "ye", "yf",
    "yg", "yh", "yi", "yj", "yk", "yl", "ym", "yn", "yo", "yp",
    "yq", "yr", "ys", "yt", "yu", "yv", "yw", "yx", "yy", "yz",
    "za", "zb", "zc", "zd", "ze", "zf", "zg", "zh", "zi", "zj",
    "zk", "zl", "zm", "zn", "zo", "zp", "zq", "zr", "zs", "zt",
    "zu", "zv", "zw", "zx", "zy", "zz",
];

/// Compute 2-char hash for a line
/// Uses a simple but stable hash based on line content and line number
pub fn compute_line_hash(line: &str, line_num: usize) -> &'static str {
    // Trim trailing whitespace for stable hashing
    let trimmed = line.trim_end();
    
    // Mix in line number for punctuation-only lines
    let mut hash: usize = 0;
    for (i, c) in trimmed.chars().enumerate() {
        hash = hash.wrapping_add((c as usize).wrapping_mul(i.wrapping_add(31)));
    }
    // Mix in line number to differentiate identical lines
    hash = hash.wrapping_add(line_num.wrapping_mul(17));
    
    let idx = hash % HL_BIGRAMS.len();
    HL_BIGRAMS[idx]
}

/// Format a line with hashline anchor
pub fn format_hash_line(line: &str, line_num: usize) -> String {
    let hash = compute_line_hash(line, line_num);
    format!("{}{}|{}", line_num, hash, line)
}

/// Format multiple lines with hashline anchors
pub fn format_hash_lines<'a>(lines: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    lines
        .into_iter()
        .enumerate()
        .map(|(i, line)| format_hash_line(line, i + 1))
        .collect()
}

/// Parsed anchor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchor {
    pub line: usize,
    pub hash: &'static str,
}

impl std::fmt::Display for Anchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.line, self.hash)
    }
}

impl std::str::FromStr for Anchor {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        if s == "BOF" || s == "EOF" {
            bail!("BOF/EOF are special anchors, not regular anchors");
        }
        
        // Find where the hash starts (last 2 chars should be the hash)
        if s.len() < 3 {
            bail!("Anchor too short: {}", s);
        }
        
        let hash_start = s.len() - 2;
        let line_str = &s[..hash_start];
        let hash = &s[hash_start..];
        
        let line: usize = line_str.parse()
            .map_err(|_| anyhow!("Invalid line number in anchor: {}", s))?;
        
        // Validate hash is lowercase letters
        if !hash.chars().all(|c| c.is_ascii_lowercase()) {
            bail!("Invalid hash in anchor: {}", s);
        }
        
        // Find the static hash string
        let hash_static = HL_BIGRAMS.iter()
            .find(|&&h| h == hash)
            .copied()
            .ok_or_else(|| anyhow!("Unknown hash: {}", hash))?;
        
        Ok(Anchor { line, hash: hash_static })
    }
}

/// Special anchor types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialAnchor {
    BOF,
    EOF,
}

/// Edit operation
#[derive(Debug, Clone)]
pub enum EditOp {
    /// Insert lines after anchor
    InsertAfter { anchor: Anchor, lines: Vec<String> },
    /// Insert lines before anchor
    InsertBefore { anchor: Anchor, lines: Vec<String> },
    /// Insert at special position (BOF/EOF)
    InsertSpecial { position: SpecialAnchor, lines: Vec<String> },
    /// Delete range of lines
    Delete { start: Anchor, end: Anchor },
    /// Replace range of lines
    Replace { start: Anchor, end: Anchor, lines: Vec<String> },
}

/// Parsed hashline section
#[derive(Debug, Clone)]
pub struct HashlineSection {
    pub path: PathBuf,
    pub ops: Vec<EditOp>,
}

/// Parse hashline input into sections
pub fn parse_hashline(input: &str) -> Result<Vec<HashlineSection>> {
    let mut sections = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_ops: Vec<EditOp> = Vec::new();
    let mut pending_op: Option<(char, String)> = None;
    let mut pending_payload: Vec<String> = Vec::new();
    
    for line in input.lines() {
        let trimmed = line.trim();
        
        // Skip empty lines and patch markers
        if trimmed.is_empty() || trimmed == "*** Begin Patch" || trimmed == "*** End Patch" {
            continue;
        }
        
        // Check for section header
        if trimmed.starts_with("@@") || trimmed.starts_with('@') {
            // Flush previous section
            if let Some(path) = current_path.take() {
                if !current_ops.is_empty() {
                    sections.push(HashlineSection { path, ops: current_ops });
                    current_ops = Vec::new();
                }
            }
            
            // Parse path
            let path_str = trimmed.trim_start_matches('@').trim();
            if path_str.is_empty() {
                bail!("Empty path in section header");
            }
            current_path = Some(PathBuf::from(path_str));
            continue;
        }
        
        // Check for operation start
        if let Some(first_char) = trimmed.chars().next() {
            match first_char {
                '+' | '<' | '-' | '=' => {
                    // Flush any pending op
                    if let Some((op_char, anchor_str)) = pending_op.take() {
                        flush_op(op_char, &anchor_str, &pending_payload, &mut current_ops)?;
                        pending_payload.clear();
                    }
                    
                    // Parse new op
                    let op_str = trimmed[1..].trim();
                    pending_op = Some((first_char, op_str.to_string()));
                }
                '~' if pending_op.is_some() => {
                    // Payload line
                    pending_payload.push(trimmed[1..].to_string());
                }
                _ => {
                    // Unknown line - could be stray payload
                    if pending_op.is_some() && trimmed.starts_with('~') {
                        pending_payload.push(trimmed[1..].to_string());
                    }
                }
            }
        }
    }
    
    // Flush final pending op
    if let Some((op_char, anchor_str)) = pending_op {
        flush_op(op_char, &anchor_str, &pending_payload, &mut current_ops)?;
    }
    
    // Add final section
    if let Some(path) = current_path {
        if !current_ops.is_empty() {
            sections.push(HashlineSection { path, ops: current_ops });
        }
    }
    
    if sections.is_empty() {
        bail!("No valid sections found in hashline input");
    }
    
    Ok(sections)
}

fn flush_op(op_char: char, anchor_str: &str, payload: &[String], ops: &mut Vec<EditOp>) -> Result<()> {
    match op_char {
        '+' => {
            if anchor_str == "EOF" {
                ops.push(EditOp::InsertSpecial {
                    position: SpecialAnchor::EOF,
                    lines: payload.to_vec(),
                });
            } else if anchor_str == "BOF" {
                ops.push(EditOp::InsertSpecial {
                    position: SpecialAnchor::BOF,
                    lines: payload.to_vec(),
                });
            } else {
                let anchor: Anchor = anchor_str.parse()?;
                ops.push(EditOp::InsertAfter {
                    anchor,
                    lines: payload.to_vec(),
                });
            }
        }
        '<' => {
            if anchor_str == "BOF" {
                ops.push(EditOp::InsertSpecial {
                    position: SpecialAnchor::BOF,
                    lines: payload.to_vec(),
                });
            } else if anchor_str == "EOF" {
                ops.push(EditOp::InsertSpecial {
                    position: SpecialAnchor::EOF,
                    lines: payload.to_vec(),
                });
            } else {
                let anchor: Anchor = anchor_str.parse()?;
                ops.push(EditOp::InsertBefore {
                    anchor,
                    lines: payload.to_vec(),
                });
            }
        }
        '-' => {
            let parts: Vec<&str> = anchor_str.split("..").collect();
            if parts.len() != 2 {
                bail!("Delete range must be A..B format");
            }
            let start: Anchor = parts[0].parse()?;
            let end: Anchor = parts[1].parse()?;
            if start.line > end.line {
                bail!("Range {}..{} ends before it starts", start, end);
            }
            ops.push(EditOp::Delete { start, end });
        }
        '=' => {
            let parts: Vec<&str> = anchor_str.split("..").collect();
            if parts.len() != 2 {
                bail!("Replace range must be A..B format");
            }
            let start: Anchor = parts[0].parse()?;
            let end: Anchor = parts[1].parse()?;
            if start.line > end.line {
                bail!("Range {}..{} ends before it starts", start, end);
            }
            ops.push(EditOp::Replace {
                start,
                end,
                lines: payload.to_vec(),
            });
        }
        _ => bail!("Unknown operation: {}", op_char),
    }
    Ok(())
}

/// Apply hashline edits to file content
/// Apply hashline edits to file content
pub fn apply_hashline(content: &str, ops: &[EditOp]) -> Result<String> {
    // Convert to owned lines upfront to avoid lifetime issues
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    // Group ops by target line and sort bottom-up
    let mut inserts_after: HashMap<usize, Vec<Vec<String>>> = HashMap::new();
    let mut inserts_before: HashMap<usize, Vec<Vec<String>>> = HashMap::new();
    let mut deletes: Vec<(usize, usize)> = Vec::new();
    let mut replacements: Vec<(usize, usize, Vec<String>)> = Vec::new();
    let mut bof_inserts: Vec<Vec<String>> = Vec::new();
    let mut eof_inserts: Vec<Vec<String>> = Vec::new();

    for op in ops {
        match op {
            EditOp::InsertAfter { anchor, lines: new_lines } => {
                validate_anchor_str(&lines, anchor)?;
                inserts_after.entry(anchor.line).or_default().push(new_lines.clone());
            }
            EditOp::InsertBefore { anchor, lines: new_lines } => {
                validate_anchor_str(&lines, anchor)?;
                inserts_before.entry(anchor.line).or_default().push(new_lines.clone());
            }
            EditOp::InsertSpecial { position, lines: new_lines } => {
                match position {
                    SpecialAnchor::BOF => bof_inserts.push(new_lines.clone()),
                    SpecialAnchor::EOF => eof_inserts.push(new_lines.clone()),
                }
            }
            EditOp::Delete { start, end } => {
                validate_anchor_str(&lines, start)?;
                validate_anchor_str(&lines, end)?;
                deletes.push((start.line, end.line));
            }
            EditOp::Replace { start, end, lines: new_lines } => {
                validate_anchor_str(&lines, start)?;
                validate_anchor_str(&lines, end)?;
                replacements.push((start.line, end.line, new_lines.clone()));
            }
        }
    }

    // Apply in order: deletes/replacements bottom-up, then inserts

    // Sort deletes/replacements by start line descending (bottom-up)
    deletes.sort_by_key(|b| std::cmp::Reverse(b.0));
    replacements.sort_by_key(|b| std::cmp::Reverse(b.0));

    // Apply replacements
    for (start, end, new_lines) in replacements {
        if start >= 1 && end <= lines.len() && start <= end {
            lines.splice((start - 1)..end, new_lines);
        }
    }

    // Apply deletes
    for (start, end) in deletes {
        if start >= 1 && end <= lines.len() && start <= end {
            lines.splice((start - 1)..end, std::iter::empty());
        }
    }

    // Apply inserts before (sorted by line descending)
    let mut before_entries: Vec<_> = inserts_before.into_iter().collect();
    before_entries.sort_by_key(|b| std::cmp::Reverse(b.0));
    for (line_num, new_line_groups) in before_entries {
        for new_lines in new_line_groups {
            let insert_pos = if line_num <= lines.len() { line_num - 1 } else { lines.len() };
            for (i, new_line) in new_lines.into_iter().enumerate() {
                lines.insert(insert_pos + i, new_line);
            }
        }
    }

    // Apply inserts after (sorted by line descending)
    let mut after_entries: Vec<_> = inserts_after.into_iter().collect();
    after_entries.sort_by_key(|b| std::cmp::Reverse(b.0));
    for (line_num, new_line_groups) in after_entries {
        for new_lines in new_line_groups {
            let insert_pos = if line_num < lines.len() { line_num } else { lines.len() };
            for (i, new_line) in new_lines.into_iter().enumerate() {
                lines.insert(insert_pos + i, new_line);
            }
        }
    }

    // Apply BOF inserts
    for new_lines in bof_inserts {
        for (i, new_line) in new_lines.into_iter().enumerate() {
            lines.insert(i, new_line);
        }
    }

    // Apply EOF inserts
    for new_lines in eof_inserts {
        lines.extend(new_lines);
    }

    Ok(lines.join("\n"))
}

fn validate_anchor_str(lines: &[String], anchor: &Anchor) -> Result<()> {
    if anchor.line < 1 || anchor.line > lines.len() {
        bail!("Line {} does not exist (file has {} lines)", anchor.line, lines.len());
    }

    let actual_hash = compute_line_hash(&lines[anchor.line - 1], anchor.line);
    if actual_hash != anchor.hash {
        bail!(
            "Stale anchor: line {} hash mismatch (expected {}, got {}). File has changed.",
            anchor.line, anchor.hash, actual_hash
        );
    }

    Ok(())
}


/// File read cache for stale-anchor recovery
#[derive(Debug, Default)]
pub struct FileReadCache {
    snapshots: HashMap<PathBuf, Vec<String>>,
}

impl FileReadCache {
    pub fn new() -> Self {
        Self {
            snapshots: HashMap::new(),
        }
    }
    
    /// Record a file read in the cache
    pub fn record(&mut self, path: PathBuf, lines: Vec<String>) {
        // Evict oldest if at capacity
        if self.snapshots.len() >= MAX_CACHED_PATHS {
            // Simple eviction: just remove one random entry
            if let Some(key) = self.snapshots.keys().next().cloned() {
                self.snapshots.remove(&key);
            }
        }
        self.snapshots.insert(path, lines);
    }
    
    /// Get cached snapshot for a path
    pub fn get(&self, path: &PathBuf) -> Option<&Vec<String>> {
        self.snapshots.get(path)
    }
    
    /// Clear cache for a specific path
    pub fn invalidate(&mut self, path: &PathBuf) {
        self.snapshots.remove(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_line_hash() {
        let h1 = compute_line_hash("fn main() {}", 1);
        let h2 = compute_line_hash("fn main() {}", 2);
        let h3 = compute_line_hash("fn main() {}", 1);
        
        // Same content + line = same hash
        assert_eq!(h1, h3);
        // Different line = different hash
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_format_hash_line() {
        let formatted = format_hash_line("fn main() {}", 1);
        assert!(formatted.starts_with("1"));
        assert!(formatted.contains("|fn main()"));
    }

    #[test]
    fn test_parse_anchor() {
        let anchor: Anchor = "41th".parse().unwrap();
        assert_eq!(anchor.line, 41);
        assert_eq!(anchor.hash, "th");
    }

    #[test]
    fn test_parse_hashline_simple_insert() {
        let input = "@@ src/test.rs\n+ 1ab\n~const x = 1;";
        let sections = parse_hashline(input).unwrap();
        
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].path, PathBuf::from("src/test.rs"));
        assert_eq!(sections[0].ops.len(), 1);
        
        match &sections[0].ops[0] {
            EditOp::InsertAfter { anchor, lines } => {
                assert_eq!(anchor.line, 1);
                assert_eq!(lines, &["const x = 1;"]);
            }
            _ => panic!("Expected InsertAfter"),
        }
    }

    #[test]
    fn test_parse_hashline_delete() {
        let input = "@@ src/test.rs\n- 1ab..3cd";
        let sections = parse_hashline(input).unwrap();
        
        assert_eq!(sections.len(), 1);
        match &sections[0].ops[0] {
            EditOp::Delete { start, end } => {
                assert_eq!(start.line, 1);
                assert_eq!(end.line, 3);
            }
            _ => panic!("Expected Delete"),
        }
    }

    #[test]
    fn test_parse_hashline_replace() {
        let input = "@@ src/test.rs\n= 1ab..2cd\n~new line 1\n~new line 2";
        let sections = parse_hashline(input).unwrap();
        
        assert_eq!(sections.len(), 1);
        match &sections[0].ops[0] {
            EditOp::Replace { start, end, lines } => {
                assert_eq!(start.line, 1);
                assert_eq!(end.line, 2);
                assert_eq!(lines, &["new line 1", "new line 2"]);
            }
            _ => panic!("Expected Replace"),
        }
    }

    #[test]
    fn test_apply_hashline_insert() {
        let content = "line 1\nline 2\nline 3";
        let ops = vec![EditOp::InsertAfter {
            anchor: Anchor { line: 1, hash: compute_line_hash("line 1", 1) },
            lines: vec!["inserted".to_string()],
        }];
        
        let result = apply_hashline(content, &ops).unwrap();
        assert_eq!(result, "line 1\ninserted\nline 2\nline 3");
    }

    #[test]
    fn test_apply_hashline_delete() {
        let content = "line 1\nline 2\nline 3";
        let ops = vec![EditOp::Delete {
            start: Anchor { line: 1, hash: compute_line_hash("line 1", 1) },
            end: Anchor { line: 2, hash: compute_line_hash("line 2", 2) },
        }];
        
        let result = apply_hashline(content, &ops).unwrap();
        assert_eq!(result, "line 3");
    }

    #[test]
    fn test_file_read_cache() {
        let mut cache = FileReadCache::new();
        let path = PathBuf::from("test.rs");
        
        cache.record(path.clone(), vec!["line 1".to_string()]);
        assert!(cache.get(&path).is_some());
        
        cache.invalidate(&path);
        assert!(cache.get(&path).is_none());
    }
}
