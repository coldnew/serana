use std::sync::Arc;

use crate::ns::Namespace;
use crate::types::{Symbol, Value};

macro_rules! builtin_fn {
    ($name:expr, $func:expr) => {{
        let sym = Symbol {
            ns: Some(Arc::new("mora.core".to_string())),
            name: Arc::new($name.to_string()),
        };
        Var::new(sym, Value::Fn(crate::types::FnValue {
            name: Some($name.to_string()),
            params: vec![],
            body: Arc::new(vec![]),
            closure: Arc::new(parking_lot::Mutex::new(std::collections::HashMap::new())),
            is_macro: false,
            meta: None,
        }))
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
    register_native(ns, "clojure.string/starts-with?", native_starts_with);
    register_native(ns, "clojure.string/ends-with?", native_ends_with);
    register_native(ns, "clojure.string/includes?", native_includes);
    register_native(ns, "clojure.string/upper-case", native_upper_case);
    register_native(ns, "clojure.string/lower-case", native_lower_case);
    register_native(ns, "clojure.string/trim", native_trim);
    register_native(ns, "clojure.string/triml", native_triml);
    register_native(ns, "clojure.string/trimr", native_trimr);
    register_native(ns, "clojure.string/split", native_split);
    register_native(ns, "clojure.string/join", native_join);
    register_native(ns, "clojure.string/replace", native_replace);
    register_native(ns, "clojure.string/blank?", native_is_blank);
    register_native(ns, "clojure.string/reverse", native_str_reverse);

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
}

fn register_native(ns: &mut Namespace, name: &str, func: fn(&[Value]) -> Result<Value, String>) {
    ns.intern(name, Value::Native(func));
}

// Store native functions in a global registry
use std::collections::HashMap;
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
    registry.get(name).map(|f| f(args)).unwrap_or_else(|| {
        Err(format!("unknown native function: {}", name))
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
            if a < b { -1 } else if a > b { 1 } else { 0 }
        }
        (Value::Int(a), Value::Float(b)) => {
            let a_f = *a as f64;
            if a_f < *b { -1 } else if a_f > *b { 1 } else { 0 }
        }
        (Value::Float(a), Value::Int(b)) => {
            let b_f = *b as f64;
            if a < &b_f { -1 } else if a > &b_f { 1 } else { 0 }
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
    Ok(Value::Bool(matches!(args.get(0), Some(Value::Int(_) | Value::Float(_)))))
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
                        new_m.extend(other.iter().cloned());
                    }
                    Value::Vector(v) if v.len() == 2 => {
                        new_m.push((v[0].clone(), v[1].clone()));
                    }
                    Value::List(v) if v.len() == 2 => {
                        new_m.push((v[0].clone(), v[1].clone()));
                    }
                    _ => return Err("conj to map requires map or [k v] pair".to_string()),
                }
            }
            Ok(Value::map(new_m))
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
        Value::List(v) | Value::Vector(v) => {
            Ok(v.first().cloned().unwrap_or(Value::Nil))
        }
        Value::Nil => Ok(Value::Nil),
        _ => Err(format!("first not supported on {}", args[0].type_name())),
    }
}

fn native_second(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("second requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::List(v) | Value::Vector(v) => {
            Ok(v.get(1).cloned().unwrap_or(Value::Nil))
        }
        Value::Nil => Ok(Value::Nil),
        _ => Err(format!("second not supported on {}", args[0].type_name())),
    }
}

fn native_last(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("last requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::List(v) | Value::Vector(v) => {
            Ok(v.last().cloned().unwrap_or(Value::Nil))
        }
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
        Value::Map(m) => {
            for (k, v) in m.iter() {
                if k == &args[1] {
                    return Ok(v.clone());
                }
            }
            Ok(default)
        }
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
                new_m.retain(|(k, _)| k != &args[i]);
                new_m.push((args[i].clone(), args[i + 1].clone()));
                i += 2;
            }
            Ok(Value::map(new_m))
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
                new_m.retain(|(k, _)| k != key);
            }
            Ok(Value::map(new_m))
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
    
    // Navigate to the parent
    let mut current = args[0].clone();
    for key in path_keys {
        current = match current {
            Value::Map(m) => {
                m.iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, v)| v.clone())
                    .unwrap_or(Value::map(vec![]))
            }
            _ => Value::map(vec![]),
        };
    }
    
    // Set the value
    match current {
        Value::Map(m) => {
            let mut new_m = m.as_ref().clone();
            new_m.retain(|(k, _)| k != &last_key);
            new_m.push((last_key, args[2].clone()));
            // Now rebuild the path
            // This is simplified - full implementation would rebuild the entire path
            Ok(Value::map(new_m))
        }
        _ => Err("assoc-in path traversal failed".to_string()),
    }
}

fn native_update(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("update requires at least 3 arguments".to_string());
    }
    let key = args[1].clone();
    let _func = &args[2];
    let _current_val = match &args[0] {
        Value::Map(m) => {
            m.iter()
                .find(|(k, _)| k == &key)
                .map(|(_, v)| v.clone())
                .unwrap_or(Value::Nil)
        }
        _ => return Err(format!("update not supported on {}", args[0].type_name())),
    };
    // Apply function to current value
    // This is simplified - full implementation would call the function
    Ok(args[0].clone())
}

fn native_update_in(args: &[Value]) -> Result<Value, String> {
    // Simplified implementation
    Ok(args[0].clone())
}

fn native_contains(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("contains? requires exactly 2 arguments".to_string());
    }
    match &args[0] {
        Value::Map(m) => {
            Ok(Value::Bool(m.iter().any(|(k, _)| k == &args[1])))
        }
        Value::Set(s) => {
            Ok(Value::Bool(s.contains(&args[1])))
        }
        Value::Vector(v) => {
            if let Value::Int(idx) = &args[1] {
                Ok(Value::Bool(*idx >= 0 && (*idx as usize) < v.len()))
            } else {
                Ok(Value::Bool(false))
            }
        }
        _ => Err(format!("contains? not supported on {}", args[0].type_name())),
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
    let mut result = Vec::new();
    for arg in args {
        match arg {
            Value::Map(m) => result.extend(m.iter().cloned()),
            Value::Nil => {}
            _ => return Err("merge requires maps".to_string()),
        }
    }
    Ok(Value::map(result))
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
            let result: Vec<(Value, Value)> = m
                .iter()
                .filter(|(k, _)| keys.contains(k))
                .cloned()
                .collect();
            Ok(Value::map(result))
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
                _ => vec![],
            };
            result.extend(v.iter().cloned());
            Ok(Value::map(result))
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
    // Simplified - would need to call the keyfn
    native_sort(&args[0..1])
}

fn native_distinct(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("distinct requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::List(v) | Value::Vector(v) => {
            let mut seen = Vec::new();
            let mut result = Vec::new();
            for item in v.iter() {
                if !seen.contains(item) {
                    seen.push(item.clone());
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
    // Simplified - would need to call pred
    Ok(args.get(1).cloned().unwrap_or(Value::Nil))
}

fn native_drop_while(args: &[Value]) -> Result<Value, String> {
    // Simplified - would need to call pred
    Ok(args.get(1).cloned().unwrap_or(Value::Nil))
}

fn native_filter(args: &[Value]) -> Result<Value, String> {
    // Simplified - would need to call pred
    Ok(args.get(1).cloned().unwrap_or(Value::Nil))
}

fn native_remove(args: &[Value]) -> Result<Value, String> {
    // Simplified - would need to call pred
    Ok(args.get(1).cloned().unwrap_or(Value::Nil))
}

fn native_map_fn(args: &[Value]) -> Result<Value, String> {
    // Simplified - would need to call func
    Ok(args.get(1).cloned().unwrap_or(Value::Nil))
}

fn native_mapcat(args: &[Value]) -> Result<Value, String> {
    // Simplified
    Ok(args.get(1).cloned().unwrap_or(Value::Nil))
}

fn native_reduce(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err("reduce requires 2-3 arguments".to_string());
    }
    // Simplified - would need to call func
    Ok(args.last().cloned().unwrap_or(Value::Nil))
}

fn native_reduce_kv(_args: &[Value]) -> Result<Value, String> {
    // Simplified
    Ok(Value::Nil)
}

fn native_apply(args: &[Value]) -> Result<Value, String> {
    // Simplified - would need to call func
    Ok(args.last().cloned().unwrap_or(Value::Nil))
}

fn native_some(_args: &[Value]) -> Result<Value, String> {
    // Simplified
    Ok(Value::Nil)
}

fn native_every(_args: &[Value]) -> Result<Value, String> {
    // Simplified
    Ok(Value::Bool(true))
}

fn native_not_every(_args: &[Value]) -> Result<Value, String> {
    // Simplified
    Ok(Value::Bool(false))
}

fn native_not_any(_args: &[Value]) -> Result<Value, String> {
    // Simplified
    Ok(Value::Bool(true))
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
    let pairs: Vec<(Value, Value)> = keys
        .iter()
        .zip(vals.iter())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    Ok(Value::map(pairs))
}

fn native_interleave(_args: &[Value]) -> Result<Value, String> {
    // Simplified
    Ok(Value::list(vec![]))
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

fn native_partition(_args: &[Value]) -> Result<Value, String> {
    // Simplified
    Ok(Value::list(vec![]))
}

fn native_partition_all(_args: &[Value]) -> Result<Value, String> {
    // Simplified
    Ok(Value::list(vec![]))
}

fn native_group_by(_args: &[Value]) -> Result<Value, String> {
    // Simplified
    Ok(Value::map(vec![]))
}

fn native_frequencies(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("frequencies requires exactly 1 argument".to_string());
    }
    match &args[0] {
        Value::List(v) | Value::Vector(v) => {
            let mut counts = Vec::new();
            for item in v.iter() {
                let mut found = false;
                for (k, count) in counts.iter_mut() {
                    if k == item {
                        if let Value::Int(n) = count {
                            *count = Value::Int(*n + 1);
                        }
                        found = true;
                        break;
                    }
                }
                if !found {
                    counts.push((item.clone(), Value::Int(1)));
                }
            }
            Ok(Value::map(counts))
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
        (Value::String(s), Value::String(prefix)) => Ok(Value::Bool(s.starts_with(prefix.as_str()))),
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
        Value::String(s) => {
            crate::reader::read_str(s).map_err(|e| format!("read error: {}", e))
        }
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
        Value::Keyword(k) => Ok(k.ns.as_ref().map(|ns| Value::string(ns.as_str())).unwrap_or(Value::Nil)),
        Value::Symbol(s) => Ok(s.ns.as_ref().map(|ns| Value::string(ns.as_str())).unwrap_or(Value::Nil)),
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
    // Simple hash implementation
    Ok(Value::Int(0))
}
