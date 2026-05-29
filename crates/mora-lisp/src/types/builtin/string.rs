use crate::ns::Namespace;
use crate::types::Value;
use super::register_native;

pub fn register(ns: &mut Namespace) {
    // Core
    register_native(ns, "str", native_str);
    register_native(ns, "subs", native_subs);

    // mora.string namespace
    register_native(ns, "mora.string/blank?", native_is_blank);
    register_native(ns, "mora.string/capitalize", native_capitalize);
    register_native(ns, "mora.string/ends-with?", native_ends_with);
    register_native(ns, "mora.string/escape", native_escape);
    register_native(ns, "mora.string/includes?", native_includes);
    register_native(ns, "mora.string/index-of", native_index_of);
    register_native(ns, "mora.string/join", native_join);
    register_native(ns, "mora.string/last-index-of", native_last_index_of);
    register_native(ns, "mora.string/lower-case", native_lower_case);
    register_native(ns, "mora.string/re-quote-replacement", super::core::native_identity);
    register_native(ns, "mora.string/replace", native_replace_with_regex);
    register_native(ns, "mora.string/replace-first", native_replace_first_regex);
    register_native(ns, "mora.string/reverse", native_str_reverse);
    register_native(ns, "mora.string/split", native_split_with_limit);
    register_native(ns, "mora.string/split-lines", native_split_lines);
    register_native(ns, "mora.string/starts-with?", native_starts_with);
    register_native(ns, "mora.string/trim", native_trim);
    register_native(ns, "mora.string/triml", native_triml);
    register_native(ns, "mora.string/trim-newline", native_trim_newline);
    register_native(ns, "mora.string/trimr", native_trimr);
    register_native(ns, "mora.string/upper-case", native_upper_case);
    register_native(ns, "mora.string/re-matches", native_regex_matches);
    register_native(ns, "mora.string/re-find", native_re_find);
    register_native(ns, "mora.string/re-seq", native_re_seq);

    // s.el inspired
    register_native(ns, "mora.string/collapse-whitespace", native_collapse_whitespace);
    register_native(ns, "mora.string/word-wrap", native_word_wrap);
    register_native(ns, "mora.string/center", native_center);
    register_native(ns, "mora.string/pad-left", native_pad_left);
    register_native(ns, "mora.string/pad-right", native_pad_right);
    register_native(ns, "mora.string/truncate", native_truncate);
    register_native(ns, "mora.string/left", native_left);
    register_native(ns, "mora.string/right", native_right);
    register_native(ns, "mora.string/chop-left", native_chop_left);
    register_native(ns, "mora.string/chop-right", native_chop_right);
    register_native(ns, "mora.string/chop-suffix", native_chop_suffix);
    register_native(ns, "mora.string/chop-suffixes", native_chop_suffixes);
    register_native(ns, "mora.string/chop-prefix", native_chop_prefix);
    register_native(ns, "mora.string/chop-prefixes", native_chop_prefixes);
    register_native(ns, "mora.string/shared-start", native_shared_start);
    register_native(ns, "mora.string/shared-end", native_shared_end);
    register_native(ns, "mora.string/repeat", native_repeat);
    register_native(ns, "mora.string/prepend", native_prepend);
    register_native(ns, "mora.string/append", native_append);
    register_native(ns, "mora.string/presence", native_presence);
    register_native(ns, "mora.string/lines", native_lines);
    register_native(ns, "mora.string/split-up-to", native_split_up_to);
    register_native(ns, "mora.string/equals?", native_str_equals);
    register_native(ns, "mora.string/less?", native_str_less);
    register_native(ns, "mora.string/present?", native_present);
    register_native(ns, "mora.string/starts-with-p?", native_starts_with);
    register_native(ns, "mora.string/ends-with-p?", native_ends_with);
    register_native(ns, "mora.string/contains-p?", native_includes);
    register_native(ns, "mora.string/lowercase?", native_str_lowercase);
    register_native(ns, "mora.string/uppercase?", native_str_uppercase);
    register_native(ns, "mora.string/mixedcase?", native_str_mixedcase);
    register_native(ns, "mora.string/capitalized?", native_str_capitalized);
    register_native(ns, "mora.string/numeric?", native_str_numeric);
    register_native(ns, "mora.string/blank-str?", native_is_blank);
    register_native(ns, "mora.string/count-matches", native_count_matches);
    register_native(ns, "mora.string/wrap", native_wrap);
    register_native(ns, "mora.string/format", native_s_format);
    register_native(ns, "mora.string/split-words", native_split_words);
    register_native(ns, "mora.string/lower-camel-case", native_lower_camel_case);
    register_native(ns, "mora.string/upper-camel-case", native_upper_camel_case);
    register_native(ns, "mora.string/snake-case", native_snake_case);
    register_native(ns, "mora.string/dashed-words", native_dashed_words);
    register_native(ns, "mora.string/capitalized-words", native_capitalized_words);
    register_native(ns, "mora.string/word-initials", native_word_initials);
    register_native(ns, "mora.string/titleize", native_titleize);
    register_native(ns, "mora.string/replace-all", native_replace_all);
    register_native(ns, "mora.string/splice", native_splice);
}
// String operations
fn native_str(args: &[Value]) -> Result<Value, String> {
    let mut result = String::new();
    for arg in args {
        match arg {
            Value::Nil => {}
            Value::String(s) => result.push_str(s),
            _ => result.push_str(&format!("{:?}", arg)),
        }
    }
    Ok(Value::string(result))
}

fn native_subs(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err("subs requires 2-3 arguments".to_string());
    }
    let s = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("subs first arg must be a string".to_string()),
    };
    let start = match &args[1] {
        Value::Int(n) => *n as usize,
        _ => return Err("subs second arg must be an integer".to_string()),
    };
    let end = if args.len() == 3 {
        match &args[2] {
            Value::Int(n) => *n as usize,
            _ => return Err("subs third arg must be an integer".to_string()),
        }
    } else {
        s.len()
    };
    if start > s.len() || end > s.len() || start > end {
        return Err("subs indices out of bounds".to_string());
    }
    Ok(Value::string(&s[start..end]))
}

fn native_starts_with(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("starts-with? requires exactly 2 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(prefix)) => {
            Ok(Value::Bool(s.starts_with(prefix.as_str())))
        }
        _ => Err("starts-with? requires strings".to_string()),
    }
}

fn native_ends_with(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("ends-with? requires exactly 2 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(suffix)) => Ok(Value::Bool(s.ends_with(suffix.as_str()))),
        _ => Err("ends-with? requires strings".to_string()),
    }
}

fn native_includes(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("includes? requires exactly 2 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(sub)) => Ok(Value::Bool(s.contains(sub.as_str()))),
        _ => Err("includes? requires strings".to_string()),
    }
}

fn native_upper_case(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("upper-case requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => Ok(Value::string(s.to_uppercase())),
        _ => Err("upper-case requires a string".to_string()),
    }
}

fn native_lower_case(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("lower-case requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => Ok(Value::string(s.to_lowercase())),
        _ => Err("lower-case requires a string".to_string()),
    }
}

fn native_trim(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("trim requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => Ok(Value::string(s.trim())),
        _ => Err("trim requires a string".to_string()),
    }
}

fn native_triml(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("triml requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => Ok(Value::string(s.trim_start())),
        _ => Err("triml requires a string".to_string()),
    }
}

fn native_trimr(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("trimr requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => Ok(Value::string(s.trim_end())),
        _ => Err("trimr requires a string".to_string()),
    }
}

fn native_split(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("split requires exactly 2 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(pattern)) => {
            let parts: Vec<Value> = s.split(pattern.as_str()).map(Value::string).collect();
            Ok(Value::vector(parts))
        }
        _ => Err("split requires strings".to_string()),
    }
}

fn native_join(args: &[Value]) -> Result<Value, String> {
    if args.len() < 1 || args.len() > 2 {
        return Err("join requires 1-2 arguments".to_string());
    }
    let sep = if args.len() == 2 {
        match &args[0] {
            Value::String(s) => s.as_str(),
            _ => return Err("join first arg must be a string".to_string()),
        }
    } else {
        ""
    };
    let seq = if args.len() == 2 { &args[1] } else { &args[0] };
    match seq {
        Value::List(v) | Value::Vector(v) => {
            let parts: Vec<String> = v
                .iter()
                .map(|x| match x {
                    Value::String(s) => s.to_string(),
                    _ => format!("{:?}", x),
                })
                .collect();
            Ok(Value::string(parts.join(sep)))
        }
        _ => Err("join requires a sequence".to_string()),
    }
}

fn native_replace(args: &[Value]) -> Result<Value, String> {
    if args.len() != 3 {
        return Err("replace requires exactly 3 arguments".to_string());
    }
    match (&args[0], &args[1], &args[2]) {
        (Value::String(s), Value::String(from), Value::String(to)) => {
            Ok(Value::string(s.replace(from.as_str(), to.as_str())))
        }
        _ => Err("replace requires strings".to_string()),
    }
}

fn native_is_blank(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("blank? requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => Ok(Value::Bool(s.trim().is_empty())),
        Value::Nil => Ok(Value::Bool(true)),
        _ => Err("blank? requires a string or nil".to_string()),
    }
}

fn native_str_reverse(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("string reverse requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => Ok(Value::string(s.chars().rev().collect::<String>())),
        _ => Err("string reverse requires a string".to_string()),
    }
}

fn native_capitalize(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("capitalize requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => {
            let mut chars = s.chars();
            let result = match chars.next() {
                None => String::new(),
                Some(first) => {
                    let mut upper = first.to_uppercase().to_string();
                    upper.push_str(&chars.as_str().to_lowercase());
                    upper
                }
            };
            Ok(Value::string(result))
        }
        _ => Err("capitalize requires a string".to_string()),
    }
}

fn native_trim_newline(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("trim-newline requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => {
            let trimmed = s.trim_end_matches(|c| c == '\n' || c == '\r');
            Ok(Value::string(trimmed))
        }
        _ => Err("trim-newline requires a string".to_string()),
    }
}

fn native_escape(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("escape requires exactly 2 arguments".to_string());
    }
    match &args[0] {
        Value::String(s) => match &args[1] {
            Value::Map(cmap) => {
                let mut result = String::with_capacity(s.len());
                for c in s.chars() {
                    let key = Value::Char(c);
                    match cmap.get(&key) {
                        Some(replacement) => match replacement {
                            Value::String(r) => result.push_str(r),
                            Value::Char(r) => result.push(*r),
                            _ => result.push(c),
                        },
                        None => result.push(c),
                    }
                }
                Ok(Value::string(result))
            }
            _ => Err("escape second arg must be a map".to_string()),
        },
        _ => Err("escape first arg must be a string".to_string()),
    }
}

fn native_index_of(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err("index-of requires 2-3 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(sub)) => {
            let start = if args.len() == 3 {
                match &args[2] {
                    Value::Int(n) => *n as usize,
                    _ => return Err("index-of third arg must be an integer".to_string()),
                }
            } else {
                0
            };
            if start >= s.len() {
                Ok(Value::Nil)
            } else {
                match s[start..].find(sub.as_str()) {
                    Some(pos) => Ok(Value::Int((start + pos) as i64)),
                    None => Ok(Value::Nil),
                }
            }
        }
        _ => Err("index-of requires strings".to_string()),
    }
}

fn native_last_index_of(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err("last-index-of requires 2-3 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(sub)) => {
            let end = if args.len() == 3 {
                match &args[2] {
                    Value::Int(n) => *n as usize,
                    _ => return Err("last-index-of third arg must be an integer".to_string()),
                }
            } else {
                s.len()
            };
            let end = end.min(s.len());
            match s[..end].rfind(sub.as_str()) {
                Some(pos) => Ok(Value::Int(pos as i64)),
                None => Ok(Value::Nil),
            }
        }
        _ => Err("last-index-of requires strings".to_string()),
    }
}

fn native_split_with_limit(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err("split requires 2-3 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(pattern)) => {
            let limit = if args.len() == 3 {
                match &args[2] {
                    Value::Int(n) => *n as usize,
                    _ => return Err("split third arg must be an integer".to_string()),
                }
            } else {
                0
            };
            let parts: Vec<Value> = if limit > 0 {
                let mut splits = s.splitn(limit, pattern.as_str());
                let mut result = Vec::new();
                while let Some(part) = splits.next() {
                    result.push(Value::string(part));
                }
                result
            } else {
                s.split(pattern.as_str()).map(Value::string).collect()
            };
            Ok(Value::vector(parts))
        }
        _ => Err("split requires strings".to_string()),
    }
}

fn native_replace_with_regex(args: &[Value]) -> Result<Value, String> {
    if args.len() != 3 {
        return Err("replace requires exactly 3 arguments".to_string());
    }
    match &args[0] {
        Value::String(s) => match &args[1] {
            Value::String(pattern) => match &args[2] {
                Value::String(replacement) => {
                    match regex::Regex::new(pattern.as_str()) {
                        Ok(re) => Ok(Value::string(
                            re.replace_all(s, replacement.as_str()).into_owned(),
                        )),
                        Err(_) => Ok(Value::string(
                            s.replace(pattern.as_str(), replacement.as_str()),
                        )),
                    }
                }
                _ => Err("replace third arg must be a string".to_string()),
            },
            _ => Err("replace second arg must be a string".to_string()),
        },
        _ => Err("replace first arg must be a string".to_string()),
    }
}

fn native_regex_matches(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("re-matches requires exactly 2 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(pattern), Value::String(s)) => {
            let re = regex::Regex::new(pattern.as_str())
                .map_err(|e| format!("invalid regex: {}", e))?;
            match re.captures(s.as_str()) {
                Some(caps) => {
                    // Only match if the entire string is covered
                    let m = caps.get(0).unwrap();
                    if m.start() != 0 || m.end() != s.len() {
                        return Ok(Value::Nil);
                    }
                    if caps.len() == 1 {
                        Ok(Value::string(caps.get(0).unwrap().as_str()))
                    } else {
                        let groups: Vec<Value> = caps
                            .iter()
                            .skip(1)
                            .map(|m| match m {
                                Some(m) => Value::string(m.as_str()),
                                None => Value::Nil,
                            })
                            .collect();
                        Ok(Value::vector(groups))
                    }
                }
                None => Ok(Value::Nil),
            }
        }
        _ => Err("re-matches requires strings".to_string()),
    }
}

fn native_re_find(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("re-find requires exactly 2 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(pattern), Value::String(s)) => {
            let re = regex::Regex::new(pattern.as_str())
                .map_err(|e| format!("invalid regex: {}", e))?;
            match re.find(s.as_str()) {
                Some(m) => Ok(Value::string(m.as_str())),
                None => Ok(Value::Nil),
            }
        }
        _ => Err("re-find requires strings".to_string()),
    }
}

fn native_re_seq(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("re-seq requires exactly 2 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(pattern), Value::String(s)) => {
            let re = regex::Regex::new(pattern.as_str())
                .map_err(|e| format!("invalid regex: {}", e))?;
            let matches: Vec<Value> = re.find_iter(s.as_str()).map(|m| Value::string(m.as_str())).collect();
            if matches.is_empty() {
                Ok(Value::Nil)
            } else {
                Ok(Value::list(matches))
            }
        }
        _ => Err("re-seq requires strings".to_string()),
    }
}


fn native_replace_first_regex(args: &[Value]) -> Result<Value, String> {
    if args.len() != 3 {
        return Err("replace-first requires exactly 3 arguments".to_string());
    }
    match &args[0] {
        Value::String(s) => match &args[1] {
            Value::String(pattern) => match &args[2] {
                Value::String(replacement) => {
                    match regex::Regex::new(pattern.as_str()) {
                        Ok(re) => Ok(Value::string(
                            re.replace(s, replacement.as_str()).into_owned(),
                        )),
                        Err(_) => {
                            // Fallback: literal string replace first
                            if let Some(pos) = s.find(pattern.as_str()) {
                                let mut result = String::with_capacity(
                                    s.len() - pattern.len() + replacement.len(),
                                );
                                result.push_str(&s[..pos]);
                                result.push_str(replacement.as_str());
                                result.push_str(&s[pos + pattern.len()..]);
                                Ok(Value::string(result))
                            } else {
                                Ok(Value::String(s.clone()))
                            }
                        }
                    }
                }
                _ => Err("replace-first third arg must be a string".to_string()),
            },
            _ => Err("replace-first second arg must be a string".to_string()),
        },
        _ => Err("replace-first first arg must be a string".to_string()),
    }
}

fn native_split_lines(args: &[Value]) -> Result<Value, String> {
    if args.len() < 1 || args.len() > 2 {
        return Err("split-lines requires 1-2 arguments".to_string());
    }
    match &args[0] {
        Value::String(s) => {
            let limit = if args.len() == 2 {
                match &args[1] {
                    Value::Int(n) => *n as usize,
                    _ => return Err("split-lines second arg must be an integer".to_string()),
                }
            } else {
                0
            };
            let parts: Vec<Value> = if limit > 0 {
                s.splitn(limit, '\n')
                    .map(|line| Value::string(line.trim_end_matches('\r')))
                    .collect()
            } else {
                s.split('\n')
                    .map(|line| Value::string(line.trim_end_matches('\r')))
                    .collect()
            };
            Ok(Value::vector(parts))
        }
        _ => Err("split-lines requires a string".to_string()),
    }
}

// --- s.el inspired string functions ---

fn native_collapse_whitespace(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("collapse-whitespace requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => {
            let mut result = String::with_capacity(s.len());
            let mut in_ws = false;
            for ch in s.chars() {
                if ch.is_whitespace() {
                    if !in_ws {
                        result.push(' ');
                        in_ws = true;
                    }
                } else {
                    result.push(ch);
                    in_ws = false;
                }
            }
            Ok(Value::string(result.trim()))
        }
        _ => Err("collapse-whitespace requires a string".to_string()),
    }
}

fn native_word_wrap(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("word-wrap requires exactly 2 arguments".to_string());
    }
    let width = match &args[0] {
        Value::Int(n) => *n as usize,
        _ => return Err("word-wrap first arg must be an integer".to_string()),
    };
    match &args[1] {
        Value::String(s) => {
            if width == 0 {
                return Ok(Value::String(s.clone()));
            }
            let mut result = String::with_capacity(s.len());
            let mut line_len = 0;
            for word in s.split_whitespace() {
                if line_len == 0 {
                    result.push_str(word);
                    line_len = word.len();
                } else if line_len + 1 + word.len() <= width {
                    result.push(' ');
                    result.push_str(word);
                    line_len += 1 + word.len();
                } else {
                    result.push('\n');
                    result.push_str(word);
                    line_len = word.len();
                }
            }
            Ok(Value::string(result))
        }
        _ => Err("word-wrap second arg must be a string".to_string()),
    }
}

fn native_center(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("center requires exactly 2 arguments".to_string());
    }
    let width = match &args[0] {
        Value::Int(n) => *n as usize,
        _ => return Err("center first arg must be an integer".to_string()),
    };
    match &args[1] {
        Value::String(s) => {
            if s.len() >= width {
                return Ok(Value::String(s.clone()));
            }
            let extra = width - s.len();
            let left = (extra + 1) / 2;
            let right = extra / 2;
            let mut result = String::with_capacity(width);
            for _ in 0..left { result.push(' '); }
            result.push_str(s);
            for _ in 0..right { result.push(' '); }
            Ok(Value::string(result))
        }
        _ => Err("center second arg must be a string".to_string()),
    }
}

fn native_pad_left(args: &[Value]) -> Result<Value, String> {
    if args.len() != 3 {
        return Err("pad-left requires exactly 3 arguments".to_string());
    }
    let width = match &args[0] {
        Value::Int(n) => *n as usize,
        _ => return Err("pad-left first arg must be an integer".to_string()),
    };
    let pad = match &args[1] {
        Value::String(s) => s.clone(),
        _ => return Err("pad-left second arg must be a string".to_string()),
    };
    match &args[2] {
        Value::String(s) => {
            if s.len() >= width {
                return Ok(Value::String(s.clone()));
            }
            let pad_len = width - s.len();
            let pad_char = pad.chars().next().unwrap_or(' ');
            let mut result = String::with_capacity(width);
            for _ in 0..pad_len { result.push(pad_char); }
            result.push_str(s);
            Ok(Value::string(result))
        }
        _ => Err("pad-left third arg must be a string".to_string()),
    }
}

fn native_pad_right(args: &[Value]) -> Result<Value, String> {
    if args.len() != 3 {
        return Err("pad-right requires exactly 3 arguments".to_string());
    }
    let width = match &args[0] {
        Value::Int(n) => *n as usize,
        _ => return Err("pad-right first arg must be an integer".to_string()),
    };
    let pad = match &args[1] {
        Value::String(s) => s.clone(),
        _ => return Err("pad-right second arg must be a string".to_string()),
    };
    match &args[2] {
        Value::String(s) => {
            if s.len() >= width {
                return Ok(Value::String(s.clone()));
            }
            let pad_len = width - s.len();
            let pad_char = pad.chars().next().unwrap_or(' ');
            let mut result = String::with_capacity(width);
            result.push_str(s);
            for _ in 0..pad_len { result.push(pad_char); }
            Ok(Value::string(result))
        }
        _ => Err("pad-right third arg must be a string".to_string()),
    }
}

fn native_truncate(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err("truncate requires 2-3 arguments".to_string());
    }
    let width = match &args[0] {
        Value::Int(n) => *n as usize,
        _ => return Err("truncate first arg must be an integer".to_string()),
    };
    let ellipsis = if args.len() == 3 {
        match &args[1] {
            Value::String(s) => s.as_str(),
            _ => return Err("truncate second arg must be a string".to_string()),
        }
    } else {
        "..."
    };
    let s_idx = if args.len() == 3 { 2 } else { 1 };
    match &args[s_idx] {
        Value::String(s) => {
            if s.len() <= width {
                return Ok(Value::String(s.clone()));
            }
            if width <= ellipsis.len() {
                return Ok(Value::string(&s[..width]));
            }
            let mut result = String::with_capacity(width);
            result.push_str(&s[..width - ellipsis.len()]);
            result.push_str(ellipsis);
            Ok(Value::string(result))
        }
        _ => Err("truncate requires a string".to_string()),
    }
}

fn native_left(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("left requires exactly 2 arguments".to_string());
    }
    let n = match &args[0] {
        Value::Int(n) => *n as usize,
        _ => return Err("left first arg must be an integer".to_string()),
    };
    match &args[1] {
        Value::String(s) => Ok(Value::string(&s[..n.min(s.len())])),
        _ => Err("left second arg must be a string".to_string()),
    }
}

fn native_right(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("right requires exactly 2 arguments".to_string());
    }
    let n = match &args[0] {
        Value::Int(n) => *n as usize,
        _ => return Err("right first arg must be an integer".to_string()),
    };
    match &args[1] {
        Value::String(s) => {
            let start = s.len().saturating_sub(n);
            Ok(Value::string(&s[start..]))
        }
        _ => Err("right second arg must be a string".to_string()),
    }
}

fn native_chop_left(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("chop-left requires exactly 2 arguments".to_string());
    }
    let n = match &args[0] {
        Value::Int(n) => *n as usize,
        _ => return Err("chop-left first arg must be an integer".to_string()),
    };
    match &args[1] {
        Value::String(s) => {
            let start = n.min(s.len());
            Ok(Value::string(&s[start..]))
        }
        _ => Err("chop-left second arg must be a string".to_string()),
    }
}

fn native_chop_right(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("chop-right requires exactly 2 arguments".to_string());
    }
    let n = match &args[0] {
        Value::Int(n) => *n as usize,
        _ => return Err("chop-right first arg must be an integer".to_string()),
    };
    match &args[1] {
        Value::String(s) => {
            let end = s.len().saturating_sub(n);
            Ok(Value::string(&s[..end]))
        }
        _ => Err("chop-right second arg must be a string".to_string()),
    }
}

fn native_chop_suffix(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("chop-suffix requires exactly 2 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(suffix), Value::String(s)) => {
            if s.ends_with(suffix.as_str()) {
                Ok(Value::string(&s[..s.len() - suffix.len()]))
            } else {
                Ok(Value::String(s.clone()))
            }
        }
        _ => Err("chop-suffix requires strings".to_string()),
    }
}

fn native_chop_suffixes(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("chop-suffixes requires exactly 2 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::List(suffixes) | Value::Vector(suffixes), Value::String(s)) => {
            let mut result = s.as_ref().clone();
            for suffix in suffixes.iter() {
                if let Value::String(sf) = suffix {
                    if result.ends_with(sf.as_str()) {
                        result = result[..result.len() - sf.len()].to_string();

                    }
                }
            }
            Ok(Value::string(result))
        }
        _ => Err("chop-suffixes requires a list and a string".to_string()),
    }
}

fn native_chop_prefix(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("chop-prefix requires exactly 2 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(prefix), Value::String(s)) => {
            if s.starts_with(prefix.as_str()) {
                Ok(Value::string(&s[prefix.len()..]))
            } else {
                Ok(Value::String(s.clone()))
            }
        }
        _ => Err("chop-prefix requires strings".to_string()),
    }
}

fn native_chop_prefixes(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("chop-prefixes requires exactly 2 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::List(prefixes) | Value::Vector(prefixes), Value::String(s)) => {
            let mut result = s.as_ref().clone();
            for prefix in prefixes.iter() {
                if let Value::String(pf) = prefix {
                    if result.starts_with(pf.as_str()) {
                        result = result[pf.len()..].to_string();

                    }
                }
            }
            Ok(Value::string(result))
        }
        _ => Err("chop-prefixes requires a list and a string".to_string()),
    }
}

fn native_shared_start(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("shared-start requires exactly 2 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(a), Value::String(b)) => {
            let mut end = 0;
            for (ca, cb) in a.chars().zip(b.chars()) {
                if ca != cb { break; }
                end += ca.len_utf8();
            }
            Ok(Value::string(&a[..end]))
        }
        _ => Err("shared-start requires strings".to_string()),
    }
}

fn native_shared_end(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("shared-end requires exactly 2 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(a), Value::String(b)) => {
            let a_chars: Vec<char> = a.chars().collect();
            let b_chars: Vec<char> = b.chars().collect();
            let mut count = 0;
            for (ca, cb) in a_chars.iter().rev().zip(b_chars.iter().rev()) {
                if ca != cb { break; }
                count += 1;
            }
            if count == 0 {
                Ok(Value::string(""))
            } else {
                let start = a.len() - a_chars.iter().rev().take(count).map(|c| c.len_utf8()).sum::<usize>();
                Ok(Value::string(&a[start..]))
            }
        }
        _ => Err("shared-end requires strings".to_string()),
    }
}

fn native_repeat(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("repeat requires exactly 2 arguments".to_string());
    }
    let n = match &args[0] {
        Value::Int(n) => (*n as usize).min(1_000_000),
        _ => return Err("repeat first arg must be an integer".to_string()),
    };
    match &args[1] {
        Value::String(s) => Ok(Value::string(s.repeat(n))),
        _ => Err("repeat second arg must be a string".to_string()),
    }
}

fn native_prepend(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("prepend requires exactly 2 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(prefix), Value::String(s)) => {
            let mut result = String::with_capacity(prefix.len() + s.len());
            result.push_str(prefix);
            result.push_str(s);
            Ok(Value::string(result))
        }
        _ => Err("prepend requires strings".to_string()),
    }
}

fn native_append(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("append requires exactly 2 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(suffix), Value::String(s)) => {
            let mut result = String::with_capacity(suffix.len() + s.len());
            result.push_str(s);
            result.push_str(suffix);
            Ok(Value::string(result))
        }
        _ => Err("append requires strings".to_string()),
    }
}

fn native_presence(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("presence requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => {
            if s.is_empty() || s.trim().is_empty() {
                Ok(Value::Nil)
            } else {
                Ok(Value::String(s.clone()))
            }
        }
        Value::Nil => Ok(Value::Nil),
        _ => Err("presence requires a string or nil".to_string()),
    }
}

fn native_lines(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("lines requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => {
            let parts: Vec<Value> = s
                .split('\n')
                .map(|line| Value::string(line.trim_end_matches('\r')))
                .collect();
            Ok(Value::vector(parts))
        }
        _ => Err("lines requires a string".to_string()),
    }
}

fn native_split_up_to(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err("split-up-to requires 2-3 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(pattern), Value::String(s)) => {
            let n = if args.len() == 3 {
                match &args[2] {
                    Value::Int(n) => *n as usize,
                    _ => return Err("split-up-to third arg must be an integer".to_string()),
                }
            } else {
                usize::MAX
            };
            let parts: Vec<Value> = s.splitn(n, pattern.as_str()).map(Value::string).collect();
            Ok(Value::vector(parts))
        }
        _ => Err("split-up-to requires strings".to_string()),
    }
}

fn native_str_equals(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("equals? requires exactly 2 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(a), Value::String(b)) => Ok(Value::Bool(a == b)),
        _ => Err("equals? requires strings".to_string()),
    }
}

fn native_str_less(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("less? requires exactly 2 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(a), Value::String(b)) => Ok(Value::Bool(a < b)),
        _ => Err("less? requires strings".to_string()),
    }
}

fn native_present(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("present? requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => Ok(Value::Bool(!s.trim().is_empty())),
        Value::Nil => Ok(Value::Bool(false)),
        _ => Err("present? requires a string or nil".to_string()),
    }
}

fn native_str_lowercase(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("lowercase? requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => Ok(Value::Bool(!s.is_empty() && s.chars().all(|c| !c.is_alphabetic() || c.is_lowercase()))),
        _ => Err("lowercase? requires a string".to_string()),
    }
}

fn native_str_uppercase(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("uppercase? requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => Ok(Value::Bool(!s.is_empty() && s.chars().all(|c| !c.is_alphabetic() || c.is_uppercase()))),
        _ => Err("uppercase? requires a string".to_string()),
    }
}

fn native_str_mixedcase(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("mixedcase? requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => {
            let has_lower = s.chars().any(|c| c.is_lowercase());
            let has_upper = s.chars().any(|c| c.is_uppercase());
            Ok(Value::Bool(has_lower && has_upper))
        }
        _ => Err("mixedcase? requires a string".to_string()),
    }
}

fn native_str_capitalized(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("capitalized? requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => {
            if s.is_empty() { return Ok(Value::Bool(false)); }
            let mut chars = s.chars();
            let first = chars.next().unwrap();
            let rest_ok = chars.all(|c| !c.is_alphabetic() || c.is_lowercase());
            Ok(Value::Bool(first.is_uppercase() && rest_ok))
        }
        _ => Err("capitalized? requires a string".to_string()),
    }
}

fn native_str_numeric(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("numeric? requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => Ok(Value::Bool(!s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))),
        _ => Err("numeric? requires a string".to_string()),
    }
}

fn native_count_matches(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("count-matches requires exactly 2 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::String(pattern), Value::String(s)) => {
            match regex::Regex::new(pattern.as_str()) {
                Ok(re) => Ok(Value::Int(re.find_iter(s.as_str()).count() as i64)),
                Err(_) => {
                    // Fallback to literal count
                    Ok(Value::Int(s.matches(pattern.as_str()).count() as i64))
                }
            }
        }
        _ => Err("count-matches requires strings".to_string()),
    }
}

fn native_wrap(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err("wrap requires 2-3 arguments".to_string());
    }
    let prefix = match &args[1] {
        Value::String(s) => s.as_str(),
        _ => return Err("wrap second arg must be a string".to_string()),
    };
    let suffix = if args.len() == 3 {
        match &args[2] {
            Value::String(s) => s.as_str(),
            _ => return Err("wrap third arg must be a string".to_string()),
        }
    } else {
        prefix
    };
    match &args[0] {
        Value::String(s) => {
            let mut result = String::with_capacity(prefix.len() + s.len() + suffix.len());
            result.push_str(prefix);
            result.push_str(s);
            result.push_str(suffix);
            Ok(Value::string(result))
        }
        _ => Err("wrap first arg must be a string".to_string()),
    }
}

fn native_s_format(args: &[Value]) -> Result<Value, String> {
    if args.len() < 1 {
        return Err("format requires at least 1 argument".to_string());
    }
    let template = match &args[0] {
        Value::String(s) => s.clone(),
        _ => return Err("format first arg must be a string".to_string()),
    };
    let mut result = String::with_capacity(template.len());
    let mut arg_idx = 1;
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '%' && chars.peek() == Some(&'s') {
            chars.next(); // consume 's'
            if arg_idx < args.len() {
                match &args[arg_idx] {
                    Value::String(s) => result.push_str(s),
                    Value::Int(n) => result.push_str(&n.to_string()),
                    Value::Float(n) => result.push_str(&n.to_string()),
                    Value::Nil => result.push_str("nil"),
                    Value::Bool(b) => result.push_str(if *b { "true" } else { "false" }),
                    other => result.push_str(&format!("{:?}", other)),
                }
                arg_idx += 1;
            }
        } else {
            result.push(ch);
        }
    }
    Ok(Value::string(result))
}

fn native_split_words(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("split-words requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => {
            let mut words = Vec::new();
            let mut current = String::new();
            let mut prev_upper = false;
            for ch in s.chars() {
                if ch == '_' || ch == '-' || ch.is_whitespace() {
                    if !current.is_empty() {
                        words.push(Value::string(&current));
                        current.clear();
                    }
                    prev_upper = false;
                } else if ch.is_uppercase() && !prev_upper && !current.is_empty() {
                    words.push(Value::string(&current));
                    current.clear();
                    current.push(ch.to_lowercase().next().unwrap());
                    prev_upper = true;
                } else {
                    current.push(ch.to_lowercase().next().unwrap());
                    prev_upper = ch.is_uppercase();
                }
            }
            if !current.is_empty() {
                words.push(Value::string(&current));
            }
            Ok(Value::vector(words))
        }
        _ => Err("split-words requires a string".to_string()),
    }
}

fn native_lower_camel_case(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("lower-camel-case requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => {
            let words = split_words_helper(s);
            let mut result = String::new();
            for (i, word) in words.iter().enumerate() {
                if i == 0 {
                    result.push_str(&word.to_lowercase());
                } else {
                    result.push_str(&capitalize_first(word));
                }
            }
            Ok(Value::string(result))
        }
        _ => Err("lower-camel-case requires a string".to_string()),
    }
}

fn native_upper_camel_case(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("upper-camel-case requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => {
            let words = split_words_helper(s);
            let mut result = String::new();
            for word in &words {
                result.push_str(&capitalize_first(word));
            }
            Ok(Value::string(result))
        }
        _ => Err("upper-camel-case requires a string".to_string()),
    }
}

fn native_snake_case(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("snake-case requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => {
            let words = split_words_helper(s);
            Ok(Value::string(words.join("_")))
        }
        _ => Err("snake-case requires a string".to_string()),
    }
}

fn native_dashed_words(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("dashed-words requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => {
            let words = split_words_helper(s);
            Ok(Value::string(words.join("-")))
        }
        _ => Err("dashed-words requires a string".to_string()),
    }
}

fn native_capitalized_words(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("capitalized-words requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => {
            let words = split_words_helper(s);
            let parts: Vec<String> = words.iter().map(|w| capitalize_first(w)).collect();
            Ok(Value::string(parts.join(" ")))
        }
        _ => Err("capitalized-words requires a string".to_string()),
    }
}

fn native_word_initials(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("word-initials requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => {
            let words = split_words_helper(s);
            let initials: String = words.iter()
                .filter_map(|w| w.chars().next())
                .collect();
            Ok(Value::string(initials))
        }
        _ => Err("word-initials requires a string".to_string()),
    }
}

fn native_titleize(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("titleize requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => {
            let mut result = String::with_capacity(s.len());
            let mut capitalize_next = true;
            for ch in s.chars() {
                if ch.is_whitespace() {
                    result.push(ch);
                    capitalize_next = true;
                } else if capitalize_next {
                    for uch in ch.to_uppercase() { result.push(uch); }
                    capitalize_next = false;
                } else {
                    for lch in ch.to_lowercase() { result.push(lch); }
                }
            }
            Ok(Value::string(result))
        }
        _ => Err("titleize requires a string".to_string()),
    }
}

fn native_replace_all(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("replace-all requires exactly 2 arguments".to_string());
    }
    match &args[0] {
        Value::Map(replacements) => match &args[1] {
            Value::String(s) => {
                let mut result = s.as_ref().clone();
                for (key, val) in replacements.iter() {
                    if let (Value::String(old), Value::String(new)) = (key, val) {
                        result = result.replace(old.as_str(), new.as_str());
                    }
                }
                Ok(Value::string(result))
            }
            _ => Err("replace-all second arg must be a string".to_string()),
        },
        _ => Err("replace-all first arg must be a map".to_string()),
    }
}


fn native_splice(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err("splice requires 2-3 arguments".to_string());
    }
    match &args[0] {
        Value::String(needle) => {
            let n = if args.len() == 3 {
                match &args[1] {
                    Value::Int(n) => *n,
                    _ => return Err("splice second arg must be an integer".to_string()),
                }
            } else {
                0
            };
            let s_idx = if args.len() == 3 { 2 } else { 1 };
            match &args[s_idx] {
                Value::String(s) => {
                    let pos = if n >= 0 {
                        n as usize
                    } else {
                        // Negative: insert at end + n + 1 chars from right
                        (s.len() as i64 + n + 1).max(0) as usize
                    };
                    let pos = pos.min(s.len());
                    let mut result = String::with_capacity(s.len() + needle.len());
                    result.push_str(&s[..pos]);
                    result.push_str(needle);
                    result.push_str(&s[pos..]);
                    Ok(Value::string(result))
                }
                _ => Err("splice requires a string".to_string()),
            }
        }
        _ => Err("splice first arg must be a string".to_string()),
    }
}

fn split_words_helper(s: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut prev_upper = false;
    for ch in s.chars() {
        if ch == '_' || ch == '-' || ch.is_whitespace() {
            if !current.is_empty() {
                words.push(current.clone());
                current.clear();
            }
            prev_upper = false;
        } else if ch.is_uppercase() && !prev_upper && !current.is_empty() {
            words.push(current.clone());
            current.clear();
            current.push(ch.to_lowercase().next().unwrap());
            prev_upper = true;
        } else {
            current.push(ch.to_lowercase().next().unwrap());
            prev_upper = ch.is_uppercase();
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let mut result = first.to_uppercase().to_string();
            result.push_str(&chars.as_str().to_lowercase());
            result
        }
    }
}
