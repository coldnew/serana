use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::ns::Namespace;
use crate::types::Value;

macro_rules! builtin_fn {
    ($name:expr, $func:expr) => {{
        let sym = Symbol {
            ns: Some(Arc::new("mora.core".to_string())),
            name: Arc::new($name.to_string()),
        };
        Var::new(
            sym,
            Value::Fn(crate::types::FnValue {
                name: Some($name.to_string()),
                params: vec![],
                body: Arc::new(vec![]),
                closure: Arc::new(parking_lot::Mutex::new(std::collections::HashMap::new())),
                is_macro: false,
                meta: None,
            }),
        )
    }};
}

pub fn register_builtins(ns: &mut Namespace) {
    // --- Arithmetic ---
    register_native(ns, "+", native_add);
    register_native(ns, "-", native_sub);
    register_native(ns, "*", native_mul);
    register_native(ns, "/", native_div);
    register_native(ns, "mod", native_mod);
    register_native(ns, "inc", native_inc);
    register_native(ns, "dec", native_dec);
    register_native(ns, "abs", native_abs);
    register_native(ns, "max", native_max);
    register_native(ns, "min", native_min);

    // --- Comparison ---
    register_native(ns, "=", native_eq);
    register_native(ns, "not=", native_neq);
    register_native(ns, "<", native_lt);
    register_native(ns, ">", native_gt);
    register_native(ns, "<=", native_lte);
    register_native(ns, ">=", native_gte);
    register_native(ns, "compare", native_compare);

    // --- Logic ---
    register_native(ns, "not", native_not);

    // --- Type Predicates ---
    register_native(ns, "nil?", native_is_nil);
    register_native(ns, "true?", native_is_true);
    register_native(ns, "false?", native_is_false);
    register_native(ns, "boolean?", native_is_boolean);
    register_native(ns, "int?", native_is_int);
    register_native(ns, "float?", native_is_float);
    register_native(ns, "number?", native_is_number);
    register_native(ns, "string?", native_is_string);
    register_native(ns, "keyword?", native_is_keyword);
    register_native(ns, "symbol?", native_is_symbol);
    register_native(ns, "list?", native_is_list);
    register_native(ns, "vector?", native_is_vector);
    register_native(ns, "map?", native_is_map);
    register_native(ns, "set?", native_is_set);
    register_native(ns, "fn?", native_is_fn);
    register_native(ns, "macro?", native_is_macro);
    register_native(ns, "atom?", native_is_atom);
    register_native(ns, "agent?", native_is_agent);
    register_native(ns, "future?", native_is_future);
    register_native(ns, "ref?", native_is_ref);
    register_native(ns, "coll?", native_is_coll);
    register_native(ns, "seq?", native_is_seq);
    register_native(ns, "empty?", native_is_empty);
    register_native(ns, "zero?", native_is_zero);
    register_native(ns, "pos?", native_is_pos);
    register_native(ns, "neg?", native_is_neg);
    register_native(ns, "even?", native_is_even);
    register_native(ns, "odd?", native_is_odd);

    // --- Collection Operations ---
    register_native(ns, "count", native_count);
    register_native(ns, "conj", native_conj);
    register_native(ns, "cons", native_cons);
    register_native(ns, "first", native_first);
    register_native(ns, "second", native_second);
    register_native(ns, "last", native_last);
    register_native(ns, "rest", native_rest);
    register_native(ns, "next", native_next);
    register_native(ns, "nth", native_nth);
    register_native(ns, "get", native_get);
    register_native(ns, "get-in", native_get_in);
    register_native(ns, "assoc", native_assoc);
    register_native(ns, "dissoc", native_dissoc);
    register_native(ns, "assoc-in", native_assoc_in);
    register_native(ns, "update", native_update);
    register_native(ns, "update-in", native_update_in);
    register_native(ns, "contains?", native_contains);
    register_native(ns, "keys", native_keys);
    register_native(ns, "vals", native_vals);
    register_native(ns, "merge", native_merge);
    register_native(ns, "select-keys", native_select_keys);
    register_native(ns, "into", native_into);
    register_native(ns, "concat", native_concat);
    register_native(ns, "flatten", native_flatten);
    register_native(ns, "reverse", native_reverse);
    register_native(ns, "sort", native_sort);
    register_native(ns, "sort-by", native_sort_by);
    register_native(ns, "distinct", native_distinct);
    register_native(ns, "take", native_take);
    register_native(ns, "drop", native_drop);
    register_native(ns, "take-while", native_take_while);
    register_native(ns, "drop-while", native_drop_while);
    register_native(ns, "filter", native_filter);
    register_native(ns, "remove", native_remove);
    register_native(ns, "map", native_map_fn);
    register_native(ns, "mapcat", native_mapcat);
    register_native(ns, "reduce", native_reduce);
    register_native(ns, "reduce-kv", native_reduce_kv);
    register_native(ns, "apply", native_apply);
    register_native(ns, "some", native_some);
    register_native(ns, "every?", native_every);
    register_native(ns, "not-every?", native_not_every);
    register_native(ns, "not-any?", native_not_any);
    register_native(ns, "zipmap", native_zipmap);
    register_native(ns, "interleave", native_interleave);
    register_native(ns, "interpose", native_interpose);
    register_native(ns, "partition", native_partition);
    register_native(ns, "partition-all", native_partition_all);
    register_native(ns, "group-by", native_group_by);
    register_native(ns, "frequencies", native_frequencies);
    register_native(ns, "peek", native_peek);
    register_native(ns, "pop", native_pop);
    register_native(ns, "subvec", native_subvec);
    register_native(ns, "vec", native_vec);
    register_native(ns, "list", native_list);
    register_native(ns, "vector", native_vector);
    register_native(ns, "hash-map", native_hash_map);
    register_native(ns, "hash-set", native_hash_set);
    register_native(ns, "seq", native_seq);
    register_native(ns, "set", native_set_fn);

    // --- String Operations ---
    register_native(ns, "str", native_str);
    register_native(ns, "subs", native_subs);
    register_native(ns, "mora.string/blank?", native_is_blank);
    register_native(ns, "mora.string/capitalize", native_capitalize);
    register_native(ns, "mora.string/ends-with?", native_ends_with);
    register_native(ns, "mora.string/escape", native_escape);
    register_native(ns, "mora.string/includes?", native_includes);
    register_native(ns, "mora.string/index-of", native_index_of);
    register_native(ns, "mora.string/join", native_join);
    register_native(ns, "mora.string/last-index-of", native_last_index_of);
    register_native(ns, "mora.string/lower-case", native_lower_case);
    register_native(ns, "mora.string/re-quote-replacement", native_identity);
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

    // --- s.el inspired functions ---
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

    // --- IO ---
    register_native(ns, "print", native_print);
    register_native(ns, "println", native_println);
    register_native(ns, "pr-str", native_pr_str);
    register_native(ns, "prn", native_prn);
    register_native(ns, "read-string", native_read_string);

    // --- Keyword/Symbol Construction ---
    register_native(ns, "keyword", native_keyword);
    register_native(ns, "symbol", native_symbol);
    register_native(ns, "name", native_name);
    register_native(ns, "namespace", native_namespace);

    // --- Misc ---
    register_native(ns, "identity", native_identity);
    register_native(ns, "constantly", native_constantly);
    register_native(ns, "comp", native_comp);
    register_native(ns, "partial", native_partial);
    register_native(ns, "complement", native_complement);
    register_native(ns, "fn?", native_is_fn);
    register_native(ns, "true", native_true);
    register_native(ns, "false", native_false);
    register_native(ns, "nil", native_nil_val);
    register_native(ns, "type", native_type);
    register_native(ns, "hash", native_hash);

    // --- GC ---
    register_native(ns, "gc-collect", native_gc_collect);
    register_native(ns, "gc-count", native_gc_count);
    register_native(ns, "gc-alloc", native_gc_alloc);
    register_native(ns, "gc-deref", native_gc_deref);

    // --- UI DSL (Reagent-like) ---
    register_native(ns, "text", native_ui_text);
    register_native(ns, "span", native_ui_span);
    register_native(ns, "column", native_ui_column);
    register_native(ns, "row", native_ui_row);
    register_native(ns, "box", native_ui_box);
    register_native(ns, "divider", native_ui_divider);
    register_native(ns, "progress", native_ui_progress);
    register_native(ns, "ui-list", native_ui_list);
    register_native(ns, "show", native_ui_show);
    register_native(ns, "for-each", native_ui_for_each);
    register_native(ns, "scroll-view", native_ui_scroll_view);
    register_native(ns, "button", native_ui_button);
}

fn register_native(ns: &mut Namespace, name: &str, func: fn(&[Value]) -> Result<Value, String>) {
    ns.intern(name, Value::Native(func));
}

use parking_lot::Mutex;
use std::sync::OnceLock;

type NativeFn = fn(&[Value]) -> Result<Value, String>;

static NATIVE_REGISTRY: OnceLock<Mutex<HashMap<String, NativeFn>>> = OnceLock::new();

fn get_registry() -> &'static Mutex<HashMap<String, NativeFn>> {
    NATIVE_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn register_native_global(name: &str, func: NativeFn) {
    get_registry().lock().insert(name.to_string(), func);
}

pub fn call_native(name: &str, args: &[Value]) -> Result<Value, String> {
    let registry = get_registry().lock();
    registry
        .get(name)
        .map(|f| f(args))
        .unwrap_or_else(|| Err(format!("unknown native function: {}", name)))
}

pub fn invoke_fn(func: &Value, args: &[Value]) -> Result<Value, String> {
    crate::eval::with_evaluator(|eval| match func {
        Value::Fn(f) => eval
            .call_fn(f.clone(), args.to_vec())
            .map_err(|e| e.to_string()),
        Value::Native(f) => f(args),
        _ => Err("not a function".to_string()),
    })
}

// --- Implementations ---

fn native_add(args: &[Value]) -> Result<Value, String> {
    let mut result = 0i64;
    let mut result_f = 0.0f64;
    let mut is_float = false;
    for arg in args {
        match arg {
            Value::Int(n) => {
                if is_float {
                    result_f += *n as f64;
                } else {
                    result += n;
                }
            }
            Value::Float(n) => {
                if !is_float {
                    result_f = result as f64;
                    is_float = true;
                }
                result_f += n;
            }
            _ => return Err(format!("+ requires numbers, got {}", arg.type_name())),
        }
    }
    if is_float {
        Ok(Value::Float(result_f))
    } else {
        Ok(Value::Int(result))
    }
}

fn native_sub(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("- requires at least 1 argument".to_string());
    }
    let first = &args[0];
    let (mut result_i, mut result_f, mut is_float) = match first {
        Value::Int(n) => (*n, 0.0f64, false),
        Value::Float(n) => (0, *n, true),
        _ => return Err(format!("- requires numbers, got {}", first.type_name())),
    };
    if args.len() == 1 {
        return if is_float {
            Ok(Value::Float(-result_f))
        } else {
            Ok(Value::Int(-result_i))
        };
    }
    for arg in &args[1..] {
        match arg {
            Value::Int(n) => {
                if is_float {
                    result_f -= *n as f64;
                } else {
                    result_i -= n;
                }
            }
            Value::Float(n) => {
                if !is_float {
                    result_f = result_i as f64;
                    is_float = true;
                }
                result_f -= n;
            }
            _ => return Err(format!("- requires numbers, got {}", arg.type_name())),
        }
    }
    if is_float {
        Ok(Value::Float(result_f))
    } else {
        Ok(Value::Int(result_i))
    }
}

fn native_mul(args: &[Value]) -> Result<Value, String> {
    let mut result = 1i64;
    let mut result_f = 1.0f64;
    let mut is_float = false;
    for arg in args {
        match arg {
            Value::Int(n) => {
                if is_float {
                    result_f *= *n as f64;
                } else {
                    result *= n;
                }
            }
            Value::Float(n) => {
                if !is_float {
                    result_f = result as f64;
                    is_float = true;
                }
                result_f *= n;
            }
            _ => return Err(format!("* requires numbers, got {}", arg.type_name())),
        }
    }
    if is_float {
        Ok(Value::Float(result_f))
    } else {
        Ok(Value::Int(result))
    }
}

fn native_div(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("/ requires at least 2 arguments".to_string());
    }
    let (mut result_i, mut result_f, mut is_float) = match &args[0] {
        Value::Int(n) => (*n, 0.0f64, false),
        Value::Float(n) => (0, *n, true),
        _ => return Err(format!("/ requires numbers, got {}", args[0].type_name())),
    };
    for arg in &args[1..] {
        match arg {
            Value::Int(n) => {
                if *n == 0 {
                    return Err("division by zero".to_string());
                }
                if is_float {
                    result_f /= *n as f64;
                } else {
                    result_i /= n;
                }
            }
            Value::Float(n) => {
                if *n == 0.0 {
                    return Err("division by zero".to_string());
                }
                if !is_float {
                    result_f = result_i as f64;
                    is_float = true;
                }
                result_f /= n;
            }
            _ => return Err(format!("/ requires numbers, got {}", arg.type_name())),
        }
    }
    if is_float {
        Ok(Value::Float(result_f))
    } else {
        Ok(Value::Int(result_i))
    }
}

fn native_mod(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("mod requires exactly 2 arguments".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => {
            if *b == 0 {
                return Err("mod by zero".to_string());
            }
            Ok(Value::Int(a % b))
        }
        _ => Err("mod requires integers".to_string()),
    }
}

fn native_inc(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("inc requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::Int(n) => Ok(Value::Int(n + 1)),
        Value::Float(n) => Ok(Value::Float(n + 1.0)),
        _ => Err("inc requires a number".to_string()),
    }
}

fn native_dec(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("dec requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::Int(n) => Ok(Value::Int(n - 1)),
        Value::Float(n) => Ok(Value::Float(n - 1.0)),
        _ => Err("dec requires a number".to_string()),
    }
}

fn native_abs(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("abs requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::Int(n) => Ok(Value::Int(n.abs())),
        Value::Float(n) => Ok(Value::Float(n.abs())),
        _ => Err("abs requires a number".to_string()),
    }
}

fn native_max(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("max requires at least 1 argument".to_string());
    }
    let mut max = &args[0];
    for arg in &args[1..] {
        if compare_values(arg, max) > 0 {
            max = arg;
        }
    }
    Ok(max.clone())
}

fn native_min(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("min requires at least 1 argument".to_string());
    }
    let mut min = &args[0];
    for arg in &args[1..] {
        if compare_values(arg, min) < 0 {
            min = arg;
        }
    }
    Ok(min.clone())
}

fn compare_values(a: &Value, b: &Value) -> i64 {
    match (a, b) {
        (Value::Int(a), Value::Int(b)) => a - b,
        (Value::Float(a), Value::Float(b)) => {
            if a < b {
                -1
            } else if a > b {
                1
            } else {
                0
            }
        }
        (Value::Int(a), Value::Float(b)) => {
            let a_f = *a as f64;
            if a_f < *b {
                -1
            } else if a_f > *b {
                1
            } else {
                0
            }
        }
        (Value::Float(a), Value::Int(b)) => {
            let b_f = *b as f64;
            if a < &b_f {
                -1
            } else if a > &b_f {
                1
            } else {
                0
            }
        }
        (Value::String(a), Value::String(b)) => a.cmp(b) as i64,
        _ => 0,
    }
}

fn native_eq(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("= requires at least 2 arguments".to_string());
    }
    for i in 1..args.len() {
        if args[0] != args[i] {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

fn native_neq(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("not= requires at least 2 arguments".to_string());
    }
    for i in 1..args.len() {
        if args[0] == args[i] {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

fn native_lt(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("< requires at least 2 arguments".to_string());
    }
    for i in 0..args.len() - 1 {
        if compare_values(&args[i], &args[i + 1]) >= 0 {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

fn native_gt(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("> requires at least 2 arguments".to_string());
    }
    for i in 0..args.len() - 1 {
        if compare_values(&args[i], &args[i + 1]) <= 0 {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

fn native_lte(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("<= requires at least 2 arguments".to_string());
    }
    for i in 0..args.len() - 1 {
        if compare_values(&args[i], &args[i + 1]) > 0 {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

fn native_gte(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err(">= requires at least 2 arguments".to_string());
    }
    for i in 0..args.len() - 1 {
        if compare_values(&args[i], &args[i + 1]) < 0 {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

fn native_compare(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("compare requires exactly 2 arguments".to_string());
    }
    Ok(Value::Int(compare_values(&args[0], &args[1])))
}

fn native_not(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("not requires exactly 1 argument".to_string());
    }
    Ok(Value::Bool(!args[0].is_truthy()))
}

// Type predicates
fn native_is_nil(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(args.get(0), Some(Value::Nil) | None)))
}

fn native_is_true(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(args.get(0), Some(Value::Bool(true)))))
}

fn native_is_false(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(args.get(0), Some(Value::Bool(false)))))
}

fn native_is_boolean(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(args.get(0), Some(Value::Bool(_)))))
}

fn native_is_int(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(args.get(0), Some(Value::Int(_)))))
}

fn native_is_float(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(args.get(0), Some(Value::Float(_)))))
}

fn native_is_number(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(
        args.get(0),
        Some(Value::Int(_) | Value::Float(_))
    )))
}

fn native_is_string(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(args.get(0), Some(Value::String(_)))))
}

fn native_is_keyword(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(args.get(0), Some(Value::Keyword(_)))))
}

fn native_is_symbol(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(args.get(0), Some(Value::Symbol(_)))))
}

fn native_is_list(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(args.get(0), Some(Value::List(_)))))
}

fn native_is_vector(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(args.get(0), Some(Value::Vector(_)))))
}

fn native_is_map(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(args.get(0), Some(Value::Map(_)))))
}

fn native_is_set(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(args.get(0), Some(Value::Set(_)))))
}

fn native_is_fn(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(args.get(0), Some(Value::Fn(_)))))
}

fn native_is_macro(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(args.get(0), Some(Value::Macro(_)))))
}

fn native_is_atom(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(args.get(0), Some(Value::Atom(_)))))
}

fn native_is_agent(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(args.get(0), Some(Value::Agent(_)))))
}

fn native_is_future(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(args.get(0), Some(Value::Future(_)))))
}

fn native_is_ref(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(args.get(0), Some(Value::Ref(_)))))
}

fn native_is_coll(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(
        args.get(0),
        Some(Value::List(_) | Value::Vector(_) | Value::Map(_) | Value::Set(_))
    )))
}

fn native_is_seq(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(matches!(
        args.get(0),
        Some(Value::List(_) | Value::Vector(_))
    )))
}

fn native_is_empty(args: &[Value]) -> Result<Value, String> {
    match args.get(0) {
        Some(Value::List(v)) => Ok(Value::Bool(v.is_empty())),
        Some(Value::Vector(v)) => Ok(Value::Bool(v.is_empty())),
        Some(Value::Map(m)) => Ok(Value::Bool(m.is_empty())),
        Some(Value::Set(s)) => Ok(Value::Bool(s.is_empty())),
        Some(Value::String(s)) => Ok(Value::Bool(s.is_empty())),
        _ => Ok(Value::Bool(true)),
    }
}

fn native_is_zero(args: &[Value]) -> Result<Value, String> {
    match args.get(0) {
        Some(Value::Int(0)) => Ok(Value::Bool(true)),
        Some(Value::Float(f)) if *f == 0.0 => Ok(Value::Bool(true)),
        _ => Ok(Value::Bool(false)),
    }
}

fn native_is_pos(args: &[Value]) -> Result<Value, String> {
    match args.get(0) {
        Some(Value::Int(n)) => Ok(Value::Bool(*n > 0)),
        Some(Value::Float(n)) => Ok(Value::Bool(*n > 0.0)),
        _ => Err("pos? requires a number".to_string()),
    }
}

fn native_is_neg(args: &[Value]) -> Result<Value, String> {
    match args.get(0) {
        Some(Value::Int(n)) => Ok(Value::Bool(*n < 0)),
        Some(Value::Float(n)) => Ok(Value::Bool(*n < 0.0)),
        _ => Err("neg? requires a number".to_string()),
    }
}

fn native_is_even(args: &[Value]) -> Result<Value, String> {
    match args.get(0) {
        Some(Value::Int(n)) => Ok(Value::Bool(n % 2 == 0)),
        _ => Err("even? requires an integer".to_string()),
    }
}

fn native_is_odd(args: &[Value]) -> Result<Value, String> {
    match args.get(0) {
        Some(Value::Int(n)) => Ok(Value::Bool(n % 2 != 0)),
        _ => Err("odd? requires an integer".to_string()),
    }
}

// Collection operations
fn native_count(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("count requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::List(v) => Ok(Value::Int(v.len() as i64)),
        Value::Vector(v) => Ok(Value::Int(v.len() as i64)),
        Value::Map(m) => Ok(Value::Int(m.len() as i64)),
        Value::Set(s) => Ok(Value::Int(s.len() as i64)),
        Value::String(s) => Ok(Value::Int(s.chars().count() as i64)),
        Value::Nil => Ok(Value::Int(0)),
        _ => Err(format!("count not supported on {}", args[0].type_name())),
    }
}

fn native_conj(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("conj requires at least 2 arguments".to_string());
    }
    match &args[0] {
        Value::List(v) => {
            let mut new_v = v.as_ref().clone();
            for arg in &args[1..] {
                new_v.insert(0, arg.clone());
            }
            Ok(Value::list(new_v))
        }
        Value::Vector(v) => {
            let mut new_v = v.as_ref().clone();
            for arg in &args[1..] {
                new_v.push(arg.clone());
            }
            Ok(Value::vector(new_v))
        }
        Value::Set(s) => {
            let mut new_s = s.as_ref().clone();
            for arg in &args[1..] {
                if !new_s.contains(arg) {
                    new_s.push(arg.clone());
                }
            }
            Ok(Value::set(new_s))
        }
        Value::Map(m) => {
            let mut new_m = m.as_ref().clone();
            for arg in &args[1..] {
                match arg {
                    Value::Map(other) => {
                        for (k, v) in other.iter() {
                            new_m.insert(k.clone(), v.clone());
                        }
                    }
                    Value::Vector(v) if v.len() == 2 => {
                        new_m.insert(v[0].clone(), v[1].clone());
                    }
                    Value::List(v) if v.len() == 2 => {
                        new_m.insert(v[0].clone(), v[1].clone());
                    }
                    _ => return Err("conj to map requires map or [k v] pair".to_string()),
                }
            }
            Ok(Value::Map(Arc::new(new_m)))
        }
        Value::Nil => {
            let mut new_v = Vec::new();
            for arg in &args[1..] {
                new_v.push(arg.clone());
            }
            Ok(Value::list(new_v))
        }
        _ => Err(format!("conj not supported on {}", args[0].type_name())),
    }
}

fn native_cons(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("cons requires exactly 2 arguments".to_string());
    }
    let head = args[0].clone();
    match &args[1] {
        Value::List(v) | Value::Vector(v) => {
            let mut new_v = vec![head];
            new_v.extend(v.iter().cloned());
            Ok(Value::list(new_v))
        }
        Value::Nil => Ok(Value::list(vec![head])),
        _ => Err("cons second arg must be a sequence".to_string()),
    }
}

fn native_first(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("first requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::List(v) | Value::Vector(v) => Ok(v.first().cloned().unwrap_or(Value::Nil)),
        Value::Nil => Ok(Value::Nil),
        _ => Err(format!("first not supported on {}", args[0].type_name())),
    }
}

fn native_second(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("second requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::List(v) | Value::Vector(v) => Ok(v.get(1).cloned().unwrap_or(Value::Nil)),
        Value::Nil => Ok(Value::Nil),
        _ => Err(format!("second not supported on {}", args[0].type_name())),
    }
}

fn native_last(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("last requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::List(v) | Value::Vector(v) => Ok(v.last().cloned().unwrap_or(Value::Nil)),
        Value::Nil => Ok(Value::Nil),
        _ => Err(format!("last not supported on {}", args[0].type_name())),
    }
}

fn native_rest(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("rest requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::List(v) | Value::Vector(v) => {
            if v.is_empty() {
                Ok(Value::list(vec![]))
            } else {
                Ok(Value::list(v[1..].to_vec()))
            }
        }
        Value::Nil => Ok(Value::list(vec![])),
        _ => Err(format!("rest not supported on {}", args[0].type_name())),
    }
}

fn native_next(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("next requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::List(v) | Value::Vector(v) => {
            if v.len() <= 1 {
                Ok(Value::Nil)
            } else {
                Ok(Value::list(v[1..].to_vec()))
            }
        }
        Value::Nil => Ok(Value::Nil),
        _ => Err(format!("next not supported on {}", args[0].type_name())),
    }
}

fn native_nth(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err("nth requires 2-3 arguments".to_string());
    }
    let idx = match &args[1] {
        Value::Int(n) => *n as usize,
        _ => return Err("nth second arg must be an integer".to_string()),
    };
    match &args[0] {
        Value::List(v) | Value::Vector(v) => {
            if idx < v.len() {
                Ok(v[idx].clone())
            } else if args.len() == 3 {
                Ok(args[2].clone())
            } else {
                Err(format!("nth index {} out of bounds (len {})", idx, v.len()))
            }
        }
        Value::String(s) => {
            if idx < s.len() {
                Ok(Value::Char(s.chars().nth(idx).unwrap()))
            } else if args.len() == 3 {
                Ok(args[2].clone())
            } else {
                Err(format!("nth index {} out of bounds", idx))
            }
        }
        _ => Err(format!("nth not supported on {}", args[0].type_name())),
    }
}

fn native_get(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err("get requires 2-3 arguments".to_string());
    }
    let default = args.get(2).cloned().unwrap_or(Value::Nil);
    match &args[0] {
        Value::Map(m) => Ok(m.get(&args[1]).cloned().unwrap_or(default)),
        Value::Set(s) => {
            if s.contains(&args[1]) {
                Ok(args[1].clone())
            } else {
                Ok(default)
            }
        }
        Value::Vector(v) => {
            if let Value::Int(idx) = &args[1] {
                let idx = *idx as usize;
                if idx < v.len() {
                    Ok(v[idx].clone())
                } else {
                    Ok(default)
                }
            } else {
                Ok(default)
            }
        }
        Value::Nil => Ok(default),
        _ => Err(format!("get not supported on {}", args[0].type_name())),
    }
}

fn native_get_in(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err("get-in requires 2-3 arguments".to_string());
    }
    let default = args.get(2).cloned().unwrap_or(Value::Nil);
    let keys = match &args[1] {
        Value::List(v) | Value::Vector(v) => v.as_ref(),
        _ => return Err("get-in second arg must be a sequence of keys".to_string()),
    };
    let mut current = args[0].clone();
    for key in keys {
        match native_get(&[current, key.clone(), Value::Nil]) {
            Ok(Value::Nil) => return Ok(default),
            Ok(val) => current = val,
            Err(e) => return Err(e),
        }
    }
    Ok(current)
}

fn native_assoc(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 || args.len() % 2 != 1 {
        return Err("assoc requires odd number of arguments".to_string());
    }
    match &args[0] {
        Value::Map(m) => {
            let mut new_m = m.as_ref().clone();
            let mut i = 1;
            while i < args.len() {
                new_m.insert(args[i].clone(), args[i + 1].clone());
                i += 2;
            }
            Ok(Value::Map(Arc::new(new_m)))
        }
        Value::Vector(v) => {
            let mut new_v = v.as_ref().clone();
            let mut i = 1;
            while i < args.len() {
                let idx = match &args[i] {
                    Value::Int(n) => *n as usize,
                    _ => return Err("assoc to vector requires integer keys".to_string()),
                };
                if idx >= new_v.len() {
                    new_v.resize(idx + 1, Value::Nil);
                }
                new_v[idx] = args[i + 1].clone();
                i += 2;
            }
            Ok(Value::vector(new_v))
        }
        Value::Nil => {
            let mut pairs = Vec::new();
            let mut i = 1;
            while i < args.len() {
                pairs.push((args[i].clone(), args[i + 1].clone()));
                i += 2;
            }
            Ok(Value::map(pairs))
        }
        _ => Err(format!("assoc not supported on {}", args[0].type_name())),
    }
}

fn native_dissoc(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("dissoc requires at least 2 arguments".to_string());
    }
    match &args[0] {
        Value::Map(m) => {
            let mut new_m = m.as_ref().clone();
            for key in &args[1..] {
                new_m.remove(key);
            }
            Ok(Value::Map(Arc::new(new_m)))
        }
        _ => Err(format!("dissoc not supported on {}", args[0].type_name())),
    }
}

fn native_assoc_in(args: &[Value]) -> Result<Value, String> {
    if args.len() != 3 {
        return Err("assoc-in requires exactly 3 arguments".to_string());
    }
    let keys = match &args[1] {
        Value::List(v) | Value::Vector(v) => v.as_ref().clone(),
        _ => return Err("assoc-in second arg must be a sequence of keys".to_string()),
    };
    if keys.is_empty() {
        return Ok(args[2].clone());
    }
    let last_key = keys.last().unwrap().clone();
    let path_keys = &keys[..keys.len() - 1];

    let mut current = args[0].clone();
    for key in path_keys {
        current = match current {
            Value::Map(m) => m
                .get(key)
                .cloned()
                .unwrap_or(Value::Map(Arc::new(HashMap::new()))),
            _ => Value::Map(Arc::new(HashMap::new())),
        };
    }

    match current {
        Value::Map(m) => {
            let mut new_m = m.as_ref().clone();
            new_m.insert(last_key, args[2].clone());
            Ok(Value::Map(Arc::new(new_m)))
        }
        _ => Err("assoc-in path traversal failed".to_string()),
    }
}

fn native_update(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("update requires at least 3 arguments".to_string());
    }
    let key = args[1].clone();
    let func = &args[2];
    let current_val = match &args[0] {
        Value::Map(m) => m.get(&key).cloned().unwrap_or(Value::Nil),
        _ => return Err(format!("update not supported on {}", args[0].type_name())),
    };
    let new_val = invoke_fn(func, &[current_val])?;
    match &args[0] {
        Value::Map(m) => {
            let mut new_m = m.as_ref().clone();
            new_m.insert(key, new_val.clone());
            Ok(Value::Map(Arc::new(new_m)))
        }
        _ => unreachable!(),
    }
}

fn native_update_in(args: &[Value]) -> Result<Value, String> {
    if args.len() != 3 {
        return Err("update-in requires exactly 3 arguments".to_string());
    }
    let keys = match &args[1] {
        Value::List(v) | Value::Vector(v) => v.as_ref().clone(),
        _ => return Err("update-in second arg must be a sequence of keys".to_string()),
    };
    if keys.is_empty() {
        return invoke_fn(&args[2], &[args[0].clone()]);
    }
    let last_key = keys.last().unwrap().clone();
    let path_keys = &keys[..keys.len() - 1];
    let mut current = args[0].clone();
    for key in path_keys {
        current = match current {
            Value::Map(m) => m
                .get(key)
                .cloned()
                .unwrap_or(Value::Map(Arc::new(HashMap::new()))),
            _ => Value::Map(Arc::new(HashMap::new())),
        };
    }
    let old_val = match &current {
        Value::Map(m) => m.get(&last_key).cloned().unwrap_or(Value::Nil),
        _ => Value::Nil,
    };
    let new_val = invoke_fn(&args[2], &[old_val])?;
    match current {
        Value::Map(m) => {
            let mut new_m = m.as_ref().clone();
            new_m.insert(last_key, new_val);
            Ok(Value::Map(Arc::new(new_m)))
        }
        _ => Err("update-in path traversal failed".to_string()),
    }
}

fn native_contains(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("contains? requires exactly 2 arguments".to_string());
    }
    match &args[0] {
        Value::Map(m) => Ok(Value::Bool(m.contains_key(&args[1]))),
        Value::Set(s) => Ok(Value::Bool(s.contains(&args[1]))),
        Value::Vector(v) => {
            if let Value::Int(idx) = &args[1] {
                Ok(Value::Bool(*idx >= 0 && (*idx as usize) < v.len()))
            } else {
                Ok(Value::Bool(false))
            }
        }
        _ => Err(format!(
            "contains? not supported on {}",
            args[0].type_name()
        )),
    }
}

fn native_keys(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("keys requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::Map(m) => {
            let keys: Vec<Value> = m.iter().map(|(k, _)| k.clone()).collect();
            Ok(Value::list(keys))
        }
        _ => Err("keys requires a map".to_string()),
    }
}

fn native_vals(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("vals requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::Map(m) => {
            let vals: Vec<Value> = m.iter().map(|(_, v)| v.clone()).collect();
            Ok(Value::list(vals))
        }
        _ => Err("vals requires a map".to_string()),
    }
}

fn native_merge(args: &[Value]) -> Result<Value, String> {
    let mut result = HashMap::new();
    for arg in args {
        match arg {
            Value::Map(m) => {
                for (k, v) in m.iter() {
                    result.insert(k.clone(), v.clone());
                }
            }
            Value::Nil => {}
            _ => return Err("merge requires maps".to_string()),
        }
    }
    Ok(Value::Map(Arc::new(result)))
}

fn native_select_keys(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("select-keys requires exactly 2 arguments".to_string());
    }
    let keys = match &args[1] {
        Value::List(v) | Value::Vector(v) => v.as_ref(),
        _ => return Err("select-keys second arg must be a sequence".to_string()),
    };
    match &args[0] {
        Value::Map(m) => {
            let result: HashMap<Value, Value> = m
                .iter()
                .filter(|(k, _)| keys.contains(k))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            Ok(Value::Map(Arc::new(result)))
        }
        _ => Err("select-keys first arg must be a map".to_string()),
    }
}

fn native_into(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("into requires exactly 2 arguments".to_string());
    }
    let from = &args[0];
    let to_add = &args[1];
    match (from, to_add) {
        (Value::List(_), Value::List(v)) | (Value::List(_), Value::Vector(v)) => {
            let mut result = match from {
                Value::List(existing) => existing.as_ref().clone(),
                _ => vec![],
            };
            for item in v.iter().rev() {
                result.insert(0, item.clone());
            }
            Ok(Value::list(result))
        }
        (Value::Vector(_), Value::List(v)) | (Value::Vector(_), Value::Vector(v)) => {
            let mut result = match from {
                Value::Vector(existing) => existing.as_ref().clone(),
                _ => vec![],
            };
            result.extend(v.iter().cloned());
            Ok(Value::vector(result))
        }
        (Value::Map(_), Value::Map(v)) => {
            let mut result = match from {
                Value::Map(existing) => existing.as_ref().clone(),
                _ => HashMap::new(),
            };
            for (k, val) in v.iter() {
                result.insert(k.clone(), val.clone());
            }
            Ok(Value::Map(Arc::new(result)))
        }
        (Value::Set(_), Value::List(v)) | (Value::Set(_), Value::Vector(v)) => {
            let mut result = match from {
                Value::Set(existing) => existing.as_ref().clone(),
                _ => vec![],
            };
            for item in v.iter() {
                if !result.contains(item) {
                    result.push(item.clone());
                }
            }
            Ok(Value::set(result))
        }
        (Value::Nil, Value::List(_)) => Ok(to_add.clone()),
        (Value::Nil, Value::Vector(_)) => Ok(to_add.clone()),
        _ => Err("into unsupported types".to_string()),
    }
}

fn native_concat(args: &[Value]) -> Result<Value, String> {
    let mut result = Vec::new();
    for arg in args {
        match arg {
            Value::List(v) | Value::Vector(v) => result.extend(v.iter().cloned()),
            Value::Nil => {}
            _ => return Err("concat requires sequences".to_string()),
        }
    }
    Ok(Value::list(result))
}

fn native_flatten(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("flatten requires exactly 1 argument".to_string());
    }
    let mut result = Vec::new();
    fn flatten_into(val: &Value, result: &mut Vec<Value>) {
        match val {
            Value::List(v) | Value::Vector(v) => {
                for item in v.iter() {
                    flatten_into(item, result);
                }
            }
            _ => result.push(val.clone()),
        }
    }
    flatten_into(&args[0], &mut result);
    Ok(Value::list(result))
}

fn native_reverse(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("reverse requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::List(v) => {
            let mut new_v = v.as_ref().clone();
            new_v.reverse();
            Ok(Value::list(new_v))
        }
        Value::Vector(v) => {
            let mut new_v = v.as_ref().clone();
            new_v.reverse();
            Ok(Value::vector(new_v))
        }
        _ => Err("reverse requires a sequence".to_string()),
    }
}

fn native_sort(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("sort requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::List(v) => {
            let mut new_v = v.as_ref().clone();
            new_v.sort_by(|a, b| {
                let cmp = compare_values(a, b);
                if cmp < 0 {
                    std::cmp::Ordering::Less
                } else if cmp > 0 {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            });
            Ok(Value::list(new_v))
        }
        Value::Vector(v) => {
            let mut new_v = v.as_ref().clone();
            new_v.sort_by(|a, b| {
                let cmp = compare_values(a, b);
                if cmp < 0 {
                    std::cmp::Ordering::Less
                } else if cmp > 0 {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            });
            Ok(Value::vector(new_v))
        }
        _ => Err("sort requires a sequence".to_string()),
    }
}

fn native_sort_by(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("sort-by requires exactly 2 arguments".to_string());
    }
    let keyfn = &args[0];
    match &args[1] {
        Value::List(v) | Value::Vector(v) => {
            let items: Vec<Value> = v.as_ref().clone();
            let mut keys: Vec<Value> = Vec::with_capacity(items.len());
            for item in items.iter() {
                keys.push(invoke_fn(keyfn, &[item.clone()])?);
            }
            let mut indices: Vec<usize> = (0..items.len()).collect();
            indices.sort_by(|&a, &b| {
                let cmp = compare_values(&keys[a], &keys[b]);
                if cmp < 0 {
                    std::cmp::Ordering::Less
                } else if cmp > 0 {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            });
            let sorted: Vec<Value> = indices.into_iter().map(|i| items[i].clone()).collect();
            Ok(Value::list(sorted))
        }
        _ => Err("sort-by second arg must be a sequence".to_string()),
    }
}

fn native_distinct(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("distinct requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::List(v) | Value::Vector(v) => {
            let mut seen = HashSet::new();
            let mut result = Vec::new();
            for item in v.iter() {
                if seen.insert(item.clone()) {
                    result.push(item.clone());
                }
            }
            Ok(Value::list(result))
        }
        _ => Err("distinct requires a sequence".to_string()),
    }
}

fn native_take(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("take requires exactly 2 arguments".to_string());
    }
    let n = match &args[0] {
        Value::Int(n) => *n as usize,
        _ => return Err("take first arg must be an integer".to_string()),
    };
    match &args[1] {
        Value::List(v) | Value::Vector(v) => {
            let take_n = n.min(v.len());
            Ok(Value::list(v[..take_n].to_vec()))
        }
        _ => Err("take second arg must be a sequence".to_string()),
    }
}

fn native_drop(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("drop requires exactly 2 arguments".to_string());
    }
    let n = match &args[0] {
        Value::Int(n) => *n as usize,
        _ => return Err("drop first arg must be an integer".to_string()),
    };
    match &args[1] {
        Value::List(v) | Value::Vector(v) => {
            let drop_n = n.min(v.len());
            Ok(Value::list(v[drop_n..].to_vec()))
        }
        _ => Err("drop second arg must be a sequence".to_string()),
    }
}

fn native_take_while(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("take-while requires exactly 2 arguments".to_string());
    }
    let pred = &args[0];
    match &args[1] {
        Value::List(v) | Value::Vector(v) => {
            let mut result = Vec::new();
            for item in v.iter() {
                match invoke_fn(pred, &[item.clone()])? {
                    Value::Bool(false) | Value::Nil => break,
                    _ => result.push(item.clone()),
                }
            }
            Ok(Value::list(result))
        }
        _ => Err("take-while second arg must be a sequence".to_string()),
    }
}

fn native_drop_while(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("drop-while requires exactly 2 arguments".to_string());
    }
    let pred = &args[0];
    match &args[1] {
        Value::List(v) | Value::Vector(v) => {
            let mut dropping = true;
            let mut result = Vec::new();
            for item in v.iter() {
                if dropping {
                    match invoke_fn(pred, &[item.clone()])? {
                        Value::Bool(false) | Value::Nil => dropping = false,
                        _ => continue,
                    }
                }
                if !dropping {
                    result.push(item.clone());
                }
            }
            Ok(Value::list(result))
        }
        _ => Err("drop-while second arg must be a sequence".to_string()),
    }
}

fn native_filter(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("filter requires exactly 2 arguments".to_string());
    }
    let pred = &args[0];
    match &args[1] {
        Value::List(v) | Value::Vector(v) => {
            let mut result = Vec::new();
            for item in v.iter() {
                match invoke_fn(pred, &[item.clone()])? {
                    Value::Bool(false) | Value::Nil => {}
                    _ => result.push(item.clone()),
                }
            }
            Ok(Value::list(result))
        }
        _ => Err("filter second arg must be a sequence".to_string()),
    }
}

fn native_remove(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("remove requires exactly 2 arguments".to_string());
    }
    let pred = &args[0];
    match &args[1] {
        Value::List(v) | Value::Vector(v) => {
            let mut result = Vec::new();
            for item in v.iter() {
                match invoke_fn(pred, &[item.clone()])? {
                    Value::Bool(false) | Value::Nil => result.push(item.clone()),
                    _ => {}
                }
            }
            Ok(Value::list(result))
        }
        _ => Err("remove second arg must be a sequence".to_string()),
    }
}

fn native_map_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("map requires at least 2 arguments".to_string());
    }
    let func = &args[0];
    match &args[1] {
        Value::List(v) | Value::Vector(v) => {
            let mut result = Vec::with_capacity(v.len());
            for item in v.iter() {
                result.push(invoke_fn(func, &[item.clone()])?);
            }
            Ok(Value::list(result))
        }
        _ => Err("map second arg must be a sequence".to_string()),
    }
}

fn native_mapcat(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("mapcat requires at least 2 arguments".to_string());
    }
    let func = &args[0];
    match &args[1] {
        Value::List(v) | Value::Vector(v) => {
            let mut result = Vec::new();
            for item in v.iter() {
                let mapped = invoke_fn(func, &[item.clone()])?;
                match mapped {
                    Value::List(seq) | Value::Vector(seq) => {
                        result.extend(seq.iter().cloned());
                    }
                    Value::Nil => {}
                    other => result.push(other),
                }
            }
            Ok(Value::list(result))
        }
        _ => Err("mapcat second arg must be a sequence".to_string()),
    }
}

fn native_reduce(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err("reduce requires 2-3 arguments".to_string());
    }
    let func = &args[0];
    let (init, seq) = if args.len() == 3 {
        (args[1].clone(), &args[2])
    } else {
        match &args[1] {
            Value::List(v) | Value::Vector(v) if !v.is_empty() => (v[0].clone(), &args[1]),
            Value::List(_) | Value::Vector(_) => return Ok(Value::Nil),
            _ => return Err("reduce second arg must be a sequence".to_string()),
        }
    };
    match seq {
        Value::List(v) | Value::Vector(v) => {
            let start = if args.len() == 3 { 0 } else { 1 };
            let mut acc = init;
            for item in v[start..].iter() {
                acc = invoke_fn(func, &[acc, item.clone()])?;
            }
            Ok(acc)
        }
        _ => Err("reduce requires a sequence".to_string()),
    }
}

fn native_reduce_kv(args: &[Value]) -> Result<Value, String> {
    if args.len() != 3 {
        return Err("reduce-kv requires exactly 3 arguments".to_string());
    }
    let func = &args[0];
    let mut acc = args[1].clone();
    match &args[2] {
        Value::Map(m) => {
            for (k, v) in m.iter() {
                acc = invoke_fn(func, &[acc, k.clone(), v.clone()])?;
            }
            Ok(acc)
        }
        Value::Vector(v) => {
            for (i, item) in v.iter().enumerate() {
                acc = invoke_fn(func, &[acc, Value::Int(i as i64), item.clone()])?;
            }
            Ok(acc)
        }
        _ => Err("reduce-kv third arg must be a map or vector".to_string()),
    }
}

fn native_apply(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("apply requires at least 2 arguments".to_string());
    }
    let func = &args[0];
    let mut call_args: Vec<Value> = Vec::new();
    for arg in &args[1..args.len() - 1] {
        call_args.push(arg.clone());
    }
    match &args[args.len() - 1] {
        Value::List(v) | Value::Vector(v) => {
            call_args.extend(v.iter().cloned());
        }
        Value::Nil => {}
        other => call_args.push(other.clone()),
    }
    invoke_fn(func, &call_args)
}

fn native_some(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("some requires exactly 2 arguments".to_string());
    }
    let pred = &args[0];
    match &args[1] {
        Value::List(v) | Value::Vector(v) => {
            for item in v.iter() {
                match invoke_fn(pred, &[item.clone()])? {
                    Value::Bool(false) | Value::Nil => {}
                    other => return Ok(other),
                }
            }
            Ok(Value::Nil)
        }
        _ => Err("some second arg must be a sequence".to_string()),
    }
}

fn native_every(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("every? requires exactly 2 arguments".to_string());
    }
    let pred = &args[0];
    match &args[1] {
        Value::List(v) | Value::Vector(v) => {
            for item in v.iter() {
                match invoke_fn(pred, &[item.clone()])? {
                    Value::Bool(false) | Value::Nil => return Ok(Value::Bool(false)),
                    _ => {}
                }
            }
            Ok(Value::Bool(true))
        }
        _ => Err("every? second arg must be a sequence".to_string()),
    }
}

fn native_not_every(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("not-every? requires exactly 2 arguments".to_string());
    }
    let pred = &args[0];
    match &args[1] {
        Value::List(v) | Value::Vector(v) => {
            for item in v.iter() {
                match invoke_fn(pred, &[item.clone()])? {
                    Value::Bool(false) | Value::Nil => return Ok(Value::Bool(true)),
                    _ => {}
                }
            }
            Ok(Value::Bool(false))
        }
        _ => Err("not-every? second arg must be a sequence".to_string()),
    }
}

fn native_not_any(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("not-any? requires exactly 2 arguments".to_string());
    }
    let pred = &args[0];
    match &args[1] {
        Value::List(v) | Value::Vector(v) => {
            for item in v.iter() {
                match invoke_fn(pred, &[item.clone()])? {
                    Value::Bool(false) | Value::Nil => {}
                    _ => return Ok(Value::Bool(false)),
                }
            }
            Ok(Value::Bool(true))
        }
        _ => Err("not-any? second arg must be a sequence".to_string()),
    }
}

fn native_zipmap(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("zipmap requires exactly 2 arguments".to_string());
    }
    let keys = match &args[0] {
        Value::List(v) | Value::Vector(v) => v.as_ref(),
        _ => return Err("zipmap first arg must be a sequence".to_string()),
    };
    let vals = match &args[1] {
        Value::List(v) | Value::Vector(v) => v.as_ref(),
        _ => return Err("zipmap second arg must be a sequence".to_string()),
    };
    let result: HashMap<Value, Value> = keys
        .iter()
        .zip(vals.iter())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    Ok(Value::Map(Arc::new(result)))
}

fn native_interleave(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("interleave requires at least 2 arguments".to_string());
    }
    let seqs: Vec<&[Value]> = args
        .iter()
        .map(|a| match a {
            Value::List(v) | Value::Vector(v) => v.as_ref() as &[Value],
            _ => &[] as &[Value],
        })
        .collect();
    let min_len = seqs.iter().map(|s| s.len()).min().unwrap_or(0);
    let mut result = Vec::with_capacity(min_len * args.len());
    for i in 0..min_len {
        for seq in &seqs {
            result.push(seq[i].clone());
        }
    }
    Ok(Value::list(result))
}

fn native_interpose(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("interpose requires exactly 2 arguments".to_string());
    }
    let sep = &args[0];
    match &args[1] {
        Value::List(v) | Value::Vector(v) => {
            let mut result = Vec::new();
            for (i, item) in v.iter().enumerate() {
                if i > 0 {
                    result.push(sep.clone());
                }
                result.push(item.clone());
            }
            Ok(Value::list(result))
        }
        _ => Err("interpose second arg must be a sequence".to_string()),
    }
}

fn native_partition(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err("partition requires 2-3 arguments".to_string());
    }
    let n = match &args[0] {
        Value::Int(n) => *n as usize,
        _ => return Err("partition first arg must be an integer".to_string()),
    };
    let step = if args.len() == 3 {
        match &args[1] {
            Value::Int(s) => *s as usize,
            _ => return Err("partition second arg must be an integer".to_string()),
        }
    } else {
        n
    };
    let seq_idx = if args.len() == 3 { 2 } else { 1 };
    match &args[seq_idx] {
        Value::List(v) | Value::Vector(v) => {
            let mut result = Vec::new();
            let mut i = 0;
            while i + n <= v.len() {
                result.push(Value::list(v[i..i + n].to_vec()));
                i += step;
            }
            Ok(Value::list(result))
        }
        _ => Err("partition requires a sequence".to_string()),
    }
}

fn native_partition_all(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err("partition-all requires 2-3 arguments".to_string());
    }
    let n = match &args[0] {
        Value::Int(n) => *n as usize,
        _ => return Err("partition-all first arg must be an integer".to_string()),
    };
    let step = if args.len() == 3 {
        match &args[1] {
            Value::Int(s) => *s as usize,
            _ => return Err("partition-all second arg must be an integer".to_string()),
        }
    } else {
        n
    };
    let seq_idx = if args.len() == 3 { 2 } else { 1 };
    match &args[seq_idx] {
        Value::List(v) | Value::Vector(v) => {
            let mut result = Vec::new();
            let mut i = 0;
            while i < v.len() {
                let end = (i + n).min(v.len());
                result.push(Value::list(v[i..end].to_vec()));
                i += step;
            }
            Ok(Value::list(result))
        }
        _ => Err("partition-all requires a sequence".to_string()),
    }
}

fn native_group_by(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("group-by requires exactly 2 arguments".to_string());
    }
    let func = &args[0];
    match &args[1] {
        Value::List(v) | Value::Vector(v) => {
            let mut result: HashMap<Value, Value> = HashMap::new();
            for item in v.iter() {
                let key = invoke_fn(func, &[item.clone()])?;
                match result.get_mut(&key) {
                    Some(Value::List(group)) => {
                        let mut group = group.as_ref().clone();
                        group.push(item.clone());
                        result.insert(key, Value::list(group));
                    }
                    _ => {
                        result.insert(key, Value::list(vec![item.clone()]));
                    }
                }
            }
            Ok(Value::Map(Arc::new(result)))
        }
        _ => Err("group-by second arg must be a sequence".to_string()),
    }
}

fn native_frequencies(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("frequencies requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::List(v) | Value::Vector(v) => {
            let mut counts: HashMap<Value, i64> = HashMap::new();
            for item in v.iter() {
                *counts.entry(item.clone()).or_insert(0) += 1;
            }
            let result: HashMap<Value, Value> = counts
                .into_iter()
                .map(|(k, v)| (k, Value::Int(v)))
                .collect();
            Ok(Value::Map(Arc::new(result)))
        }
        _ => Err("frequencies requires a sequence".to_string()),
    }
}

fn native_peek(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("peek requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::List(v) => Ok(v.last().cloned().unwrap_or(Value::Nil)),
        Value::Vector(v) => Ok(v.last().cloned().unwrap_or(Value::Nil)),
        _ => Err("peek requires a collection".to_string()),
    }
}

fn native_pop(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("pop requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::List(v) => {
            if v.is_empty() {
                Err("pop on empty list".to_string())
            } else {
                Ok(Value::list(v[..v.len() - 1].to_vec()))
            }
        }
        Value::Vector(v) => {
            if v.is_empty() {
                Err("pop on empty vector".to_string())
            } else {
                Ok(Value::vector(v[..v.len() - 1].to_vec()))
            }
        }
        _ => Err("pop requires a collection".to_string()),
    }
}

fn native_subvec(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err("subvec requires 2-3 arguments".to_string());
    }
    let start = match &args[1] {
        Value::Int(n) => *n as usize,
        _ => return Err("subvec second arg must be an integer".to_string()),
    };
    match &args[0] {
        Value::Vector(v) => {
            let end = if args.len() == 3 {
                match &args[2] {
                    Value::Int(n) => *n as usize,
                    _ => return Err("subvec third arg must be an integer".to_string()),
                }
            } else {
                v.len()
            };
            if start > v.len() || end > v.len() || start > end {
                return Err("subvec indices out of bounds".to_string());
            }
            Ok(Value::vector(v[start..end].to_vec()))
        }
        _ => Err("subvec requires a vector".to_string()),
    }
}

fn native_vec(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("vec requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::List(v) => Ok(Value::vector(v.as_ref().clone())),
        Value::Vector(_) => Ok(args[0].clone()),
        _ => Err("vec requires a sequence".to_string()),
    }
}

fn native_list(args: &[Value]) -> Result<Value, String> {
    Ok(Value::list(args.to_vec()))
}

fn native_vector(args: &[Value]) -> Result<Value, String> {
    Ok(Value::vector(args.to_vec()))
}

fn native_hash_map(args: &[Value]) -> Result<Value, String> {
    if args.len() % 2 != 0 {
        return Err("hash-map requires even number of arguments".to_string());
    }
    let mut pairs = Vec::new();
    let mut i = 0;
    while i < args.len() {
        pairs.push((args[i].clone(), args[i + 1].clone()));
        i += 2;
    }
    Ok(Value::map(pairs))
}

fn native_hash_set(args: &[Value]) -> Result<Value, String> {
    Ok(Value::set(args.to_vec()))
}

fn native_seq(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("seq requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::List(v) if v.is_empty() => Ok(Value::Nil),
        Value::Vector(v) if v.is_empty() => Ok(Value::Nil),
        Value::String(s) if s.is_empty() => Ok(Value::Nil),
        Value::List(_) | Value::Vector(_) => Ok(args[0].clone()),
        Value::String(s) => {
            let chars: Vec<Value> = s.chars().map(Value::Char).collect();
            Ok(Value::list(chars))
        }
        Value::Nil => Ok(Value::Nil),
        _ => Err(format!("seq not supported on {}", args[0].type_name())),
    }
}

fn native_set_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("set requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::List(v) | Value::Vector(v) => Ok(Value::set(v.as_ref().clone())),
        Value::Map(m) => {
            let vals: Vec<Value> = m.iter().map(|(k, _)| k.clone()).collect();
            Ok(Value::set(vals))
        }
        _ => Err(format!("set not supported on {}", args[0].type_name())),
    }
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

// IO
fn native_print(args: &[Value]) -> Result<Value, String> {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        match arg {
            Value::String(s) => print!("{}", s),
            _ => print!("{:?}", arg),
        }
    }
    Ok(Value::Nil)
}

fn native_println(args: &[Value]) -> Result<Value, String> {
    native_print(args)?;
    println!();
    Ok(Value::Nil)
}

fn native_pr_str(args: &[Value]) -> Result<Value, String> {
    let parts: Vec<String> = args.iter().map(|a| format!("{:?}", a)).collect();
    Ok(Value::string(parts.join(" ")))
}

fn native_prn(args: &[Value]) -> Result<Value, String> {
    let s = native_pr_str(args)?;
    if let Value::String(ref s) = s {
        println!("{}", s);
    }
    Ok(Value::Nil)
}

fn native_read_string(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("read-string requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => crate::reader::read_str(s).map_err(|e| format!("read error: {}", e)),
        _ => Err("read-string requires a string".to_string()),
    }
}

// Keyword/Symbol construction
fn native_keyword(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("keyword requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => Ok(Value::keyword(s.as_str())),
        Value::Keyword(_) => Ok(args[0].clone()),
        _ => Err("keyword requires a string or keyword".to_string()),
    }
}

fn native_symbol(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("symbol requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::String(s) => Ok(Value::symbol(s.as_str())),
        Value::Symbol(_) => Ok(args[0].clone()),
        _ => Err("symbol requires a string or symbol".to_string()),
    }
}

fn native_name(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("name requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::Keyword(k) => Ok(Value::string(k.name.as_str())),
        Value::Symbol(s) => Ok(Value::string(s.name.as_str())),
        Value::String(s) => Ok(Value::string(s.as_str())),
        _ => Err("name requires a keyword, symbol, or string".to_string()),
    }
}

fn native_namespace(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("namespace requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::Keyword(k) => Ok(k
            .ns
            .as_ref()
            .map(|ns| Value::string(ns.as_str()))
            .unwrap_or(Value::Nil)),
        Value::Symbol(s) => Ok(s
            .ns
            .as_ref()
            .map(|ns| Value::string(ns.as_str()))
            .unwrap_or(Value::Nil)),
        _ => Err("namespace requires a keyword or symbol".to_string()),
    }
}

// Misc
fn native_identity(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("identity requires exactly 1 argument".to_string());
    }
    Ok(args[0].clone())
}

fn native_constantly(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("constantly requires exactly 1 argument".to_string());
    }
    let val = args[0].clone();
    // Return a function that always returns val
    // Simplified - would need to create a closure
    Ok(val)
}

fn native_comp(_args: &[Value]) -> Result<Value, String> {
    // Simplified - would need to compose functions
    Ok(Value::Nil)
}

fn native_partial(_args: &[Value]) -> Result<Value, String> {
    // Simplified - would need to create partial application
    Ok(Value::Nil)
}

fn native_complement(_args: &[Value]) -> Result<Value, String> {
    // Simplified - would need to create complement function
    Ok(Value::Nil)
}

fn native_true(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(true))
}

fn native_false(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(false))
}

fn native_nil_val(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Nil)
}

fn native_type(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("type requires exactly 1 argument".to_string());
    }
    Ok(Value::symbol(args[0].type_name()))
}

fn native_hash(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("hash requires exactly 1 argument".to_string());
    }
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    args[0].hash(&mut hasher);
    Ok(Value::Int(hasher.finish() as i64))
}

fn native_gc_collect(_args: &[Value]) -> Result<Value, String> {
    crate::eval::with_evaluator(|eval| {
        let freed = eval.gc_heap.collect();
        Ok(Value::Int(freed as i64))
    })
}

fn native_gc_count(_args: &[Value]) -> Result<Value, String> {
    crate::eval::with_evaluator(|eval| Ok(Value::Int(eval.gc_heap.object_count() as i64)))
}

fn native_gc_alloc(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("gc-alloc requires exactly 1 argument".to_string());
    }
    crate::eval::with_evaluator(|eval| {
        let value = args[0].clone();
        let gc = crate::gc::Gc::new(&eval.gc_heap, value);
        Ok(Value::Gc(gc))
    })
}

fn native_gc_deref(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("gc-deref requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::Gc(gc) => gc.with(|v| Ok(v.clone())),
        other => Err(format!(
            "gc-deref expects a gc value, got {}",
            other.type_name()
        )),
    }
}

// --- UI DSL (Reagent-like hiccup) ---

const STYLE_KEYS: &[&str] = &[
    "bold", "italic", "underline", "dim", "strikethrough",
    "reverse", "blink", "fg", "color", "bg",
];

fn style_key(kw: &str) -> Value {
    Value::keyword(kw)
}

fn extract_style_from_map(map: &HashMap<Value, Value>) -> Option<Value> {
    let mut pairs = Vec::new();
    for &k in STYLE_KEYS {
        let key = style_key(k);
        if let Some(val) = map.get(&key) {
            let normalized_key = if k == "color" { style_key("fg") } else { key };
            pairs.push((normalized_key, val.clone()));
        }
    }
    if pairs.is_empty() {
        None
    } else {
        Some(Value::map(pairs))
    }
}

fn is_props_map(arg: &Value) -> bool {
    matches!(arg, Value::Map(_))
}

fn get_map(arg: &Value) -> Option<&HashMap<Value, Value>> {
    match arg {
        Value::Map(m) => Some(m.as_ref()),
        _ => None,
    }
}

fn map_get_str(map: &HashMap<Value, Value>, key: &str) -> Option<String> {
    map.get(&style_key(key)).and_then(|v| match v {
        Value::String(s) => Some(s.to_string()),
        _ => None,
    })
}

fn map_get_f64(map: &HashMap<Value, Value>, key: &str) -> Option<f64> {
    map.get(&style_key(key)).and_then(|v| match v {
        Value::Float(f) => Some(*f),
        Value::Int(i) => Some(*i as f64),
        _ => None,
    })
}

fn map_get_bool(map: &HashMap<Value, Value>, key: &str) -> Option<bool> {
    map.get(&style_key(key)).and_then(|v| match v {
        Value::Bool(b) => Some(*b),
        _ => None,
    })
}

fn map_get_u16(map: &HashMap<Value, Value>, key: &str) -> Option<u16> {
    map.get(&style_key(key)).and_then(|v| match v {
        Value::Int(i) => Some(*i as u16),
        _ => None,
    })
}

fn extract_common_props(map: &HashMap<Value, Value>) -> Vec<(Value, Value)> {
    let mut props = Vec::new();
    for &k in &["gap", "align", "justify", "padding", "width", "height",
                 "flex-grow", "flex-shrink", "border", "title",
                 "min-width", "min-height", "max-width", "max-height",
                 "char", "value", "max", "show-percent", "marker",
                 "scroll-top", "on-click", "on-change"] {
        let key = style_key(k);
        if let Some(val) = map.get(&key) {
            props.push((key, val.clone()));
        }
    }
    props
}

fn make_node(typ: &str, content: Option<String>, style: Option<Value>,
              props: Option<Value>, children: Option<Vec<Value>>) -> Value {
    let mut pairs = vec![
        (Value::keyword("type"), Value::keyword(typ)),
    ];
    if let Some(c) = content {
        pairs.push((Value::keyword("content"), Value::string(c)));
    }
    if let Some(s) = style {
        pairs.push((Value::keyword("style"), s));
    }
    if let Some(p) = props {
        pairs.push((Value::keyword("props"), p));
    }
    if let Some(ch) = children {
        pairs.push((Value::keyword("children"), Value::vector(ch)));
    }
    Value::map(pairs)
}

// (text "content" :bold true :color "#ff0000")
// (text {:bold true :color "#ff0000"} "content")
fn native_ui_text(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return make_node("text", Some(String::new()), None, None, None).into_result();
    }

    let mut style = None;
    let mut content = String::new();

    if let Some(map) = get_map(&args[0]) {
        style = extract_style_from_map(map);
        if args.len() > 1 {
            content = args[1].to_string();
        }
    } else {
        content = args[0].to_string();
        if args.len() > 1 {
            if let Some(map) = get_map(&args[1]) {
                style = extract_style_from_map(map);
            }
        }
    }

    Ok(make_node("text", Some(content), style, None, None))
}

// (span "content" :bold true :color "#ff0000")
fn native_ui_span(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Ok(make_node("span", Some(String::new()), None, None, None));
    }

    let mut style = None;
    let mut content = String::new();

    if let Some(map) = get_map(&args[0]) {
        style = extract_style_from_map(map);
        if args.len() > 1 {
            content = args[1].to_string();
        }
    } else {
        content = args[0].to_string();
        if args.len() > 1 {
            if let Some(map) = get_map(&args[1]) {
                style = extract_style_from_map(map);
            }
        }
    }

    Ok(make_node("span", Some(content), style, None, None))
}

// (column {:gap 1 :align "center"} child1 child2 ...)
// (column child1 child2 ...)
fn native_ui_column(args: &[Value]) -> Result<Value, String> {
    let (props_val, children) = if args.is_empty() {
        (None, vec![])
    } else if let Some(map) = get_map(&args[0]) {
        let style = extract_style_from_map(map);
        let common = extract_common_props(map);
        let props = if !common.is_empty() || style.is_some() {
            let mut p = common;
            if let Some(s) = style {
                p.push((Value::keyword("style"), s));
            }
            Some(Value::map(p))
        } else {
            None
        };
        (props, args[1..].to_vec())
    } else {
        (None, args.to_vec())
    };

    Ok(make_node("column", None, None, props_val, Some(children)))
}

// (row {:gap 1} child1 child2 ...)
fn native_ui_row(args: &[Value]) -> Result<Value, String> {
    let (props_val, children) = if args.is_empty() {
        (None, vec![])
    } else if let Some(map) = get_map(&args[0]) {
        let style = extract_style_from_map(map);
        let common = extract_common_props(map);
        let props = if !common.is_empty() || style.is_some() {
            let mut p = common;
            if let Some(s) = style {
                p.push((Value::keyword("style"), s));
            }
            Some(Value::map(p))
        } else {
            None
        };
        (props, args[1..].to_vec())
    } else {
        (None, args.to_vec())
    };

    Ok(make_node("row", None, None, props_val, Some(children)))
}

// (box {:title "hi" :border true :padding 1} child1 child2 ...)
fn native_ui_box(args: &[Value]) -> Result<Value, String> {
    let (props_val, children) = if args.is_empty() {
        (None, vec![])
    } else if let Some(map) = get_map(&args[0]) {
        let style = extract_style_from_map(map);
        let common = extract_common_props(map);
        let props = if !common.is_empty() || style.is_some() {
            let mut p = common;
            if let Some(s) = style {
                p.push((Value::keyword("style"), s));
            }
            Some(Value::map(p))
        } else {
            None
        };
        (props, args[1..].to_vec())
    } else {
        (None, args.to_vec())
    };

    Ok(make_node("box", None, None, props_val, Some(children)))
}

// (divider) or (divider "-") or (divider {:char "-"})
fn native_ui_divider(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Ok(make_node("divider", None, None, None, None));
    }

    if let Some(map) = get_map(&args[0]) {
        let props = extract_common_props(map);
        Ok(make_node("divider", None, None,
            if props.is_empty() { None } else { Some(Value::map(props)) }, None))
    } else {
        let ch = args[0].to_string().chars().next().unwrap_or('─');
        Ok(make_node("divider", None, None,
            Some(Value::map(vec![(Value::keyword("char"), Value::Char(ch))])), None))
    }
}

// (progress {:value 75 :max 100 :width 20})
fn native_ui_progress(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Ok(make_node("progress", None, None, None, None));
    }

    if let Some(map) = get_map(&args[0]) {
        let props = extract_common_props(map);
        let style = extract_style_from_map(map);
        let mut all_props = props;
        if let Some(s) = style {
            all_props.push((Value::keyword("style"), s));
        }
        Ok(make_node("progress", None, None,
            if all_props.is_empty() { None } else { Some(Value::map(all_props)) }, None))
    } else {
        Ok(make_node("progress", None, None, None, None))
    }
}

// (ui-list {:marker :bullet} item1 item2 ...)
fn native_ui_list(args: &[Value]) -> Result<Value, String> {
    let (props_val, children) = if args.is_empty() {
        (None, vec![])
    } else if let Some(map) = get_map(&args[0]) {
        let common = extract_common_props(map);
        let style = extract_style_from_map(map);
        let mut p = common;
        if let Some(s) = style {
            p.push((Value::keyword("style"), s));
        }
        (if p.is_empty() { None } else { Some(Value::map(p)) }, args[1..].to_vec())
    } else {
        (None, args.to_vec())
    };

    Ok(make_node("list", None, None, props_val, Some(children)))
}

// (show condition child)
fn native_ui_show(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("show requires 2 arguments: condition and child".to_string());
    }

    let when = match &args[0] {
        Value::Bool(b) => *b,
        Value::Nil => false,
        _ => true,
    };

    let mut pairs = vec![
        (Value::keyword("type"), Value::keyword("show")),
        (Value::keyword("when"), Value::Bool(when)),
        (Value::keyword("child"), args[1].clone()),
    ];
    Ok(Value::map(pairs))
}

// (for-each f coll) or (for-each coll f)
fn native_ui_for_each(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("for-each requires 2 arguments: function and collection".to_string());
    }

    // Determine which is fn and which is coll
    let (func, coll) = match (&args[0], &args[1]) {
        (Value::Fn(_), Value::Vector(_) | Value::List(_)) => (&args[0], &args[1]),
        (Value::Vector(_) | Value::List(_), Value::Fn(_)) => (&args[1], &args[0]),
        (Value::Native(_), Value::Vector(_) | Value::List(_)) => (&args[0], &args[1]),
        (Value::Vector(_) | Value::List(_), Value::Native(_)) => (&args[1], &args[0]),
        _ => return Err("for-each requires a function and a collection".to_string()),
    };

    // Store fn and coll as props, actual mapping happens at conversion time
    let mut pairs = vec![
        (Value::keyword("type"), Value::keyword("for")),
        (Value::keyword("func"), func.clone()),
        (Value::keyword("coll"), coll.clone()),
    ];
    Ok(Value::map(pairs))
}

// (scroll-view {:scroll-top 0 :height 10} child)
fn native_ui_scroll_view(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("scroll-view requires at least 1 argument".to_string());
    }

    if let Some(map) = get_map(&args[0]) {
        let common = extract_common_props(map);
        let child = if args.len() > 1 { args[1].clone() } else { Value::Nil };
        let mut pairs = vec![
            (Value::keyword("type"), Value::keyword("scroll-view")),
            (Value::keyword("child"), child),
        ];
        if !common.is_empty() {
            pairs.push((Value::keyword("props"), Value::map(common)));
        }
        Ok(Value::map(pairs))
    } else {
        let mut pairs = vec![
            (Value::keyword("type"), Value::keyword("scroll-view")),
            (Value::keyword("child"), args[0].clone()),
        ];
        Ok(Value::map(pairs))
    }
}

// (button {:on-click handler} label)
fn native_ui_button(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Ok(make_node("button", Some(String::new()), None, None, None));
    }

    let mut style = None;
    let mut props_pairs = Vec::new();
    let mut label = String::new();

    if let Some(map) = get_map(&args[0]) {
        style = extract_style_from_map(map);
        // Extract on-click and other non-style props
        if let Some(on_click) = map.get(&style_key("on-click")) {
            props_pairs.push((Value::keyword("on-click"), on_click.clone()));
        }
        if args.len() > 1 {
            label = args[1].to_string();
        }
    } else {
        label = args[0].to_string();
        if args.len() > 1 {
            if let Some(map) = get_map(&args[1]) {
                style = extract_style_from_map(map);
                if let Some(on_click) = map.get(&style_key("on-click")) {
                    props_pairs.push((Value::keyword("on-click"), on_click.clone()));
                }
            }
        }
    }

    let props = if props_pairs.is_empty() { None } else { Some(Value::map(props_pairs)) };
    Ok(make_node("button", Some(label), style, props, None))
}

trait IntoResult {
    fn into_result(self) -> Result<Value, String>;
}

impl IntoResult for Value {
    fn into_result(self) -> Result<Value, String> {
        Ok(self)
    }
}
