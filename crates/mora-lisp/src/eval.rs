use std::cell::Cell;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use parking_lot::Mutex;

use crate::gc::{CollectorHandle, GcHeap};
use crate::ns::NamespaceRegistry;
use crate::thread::ThreadPool;
use crate::types::{FnValue, MacroValue, Param, Symbol, Value};

#[derive(Debug, Clone, thiserror::Error)]
pub enum EvalError {
    #[error("undefined symbol: {0}")]
    Undefined(String),
    #[error("not a function: {0}")]
    NotAFunction(String),
    #[error("not a macro: {0}")]
    NotAMacro(String),
    #[error("invalid arity: expected {expected}, got {got}")]
    Arity { expected: usize, got: usize },
    #[error("type error: {0}")]
    Type(String),
    #[error("special form error: {0}")]
    SpecialForm(String),
    #[error("macro expansion error: {0}")]
    MacroExpansion(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("{0}")]
    Custom(String),
    #[error("throw: {0}")]
    Throw(Value),
}

#[derive(Clone)]
pub struct Env {
    bindings: HashMap<Symbol, Value>,
    parent: Option<Box<Env>>,
}

impl Env {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            parent: None,
        }
    }

    pub fn extend(parent: Env) -> Self {
        Self {
            bindings: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn get(&self, sym: &Symbol) -> Option<&Value> {
        self.bindings
            .get(sym)
            .or_else(|| self.parent.as_ref().and_then(|p| p.get(sym)))
    }

    pub fn set(&mut self, sym: Symbol, val: Value) {
        self.bindings.insert(sym, val);
    }

    pub fn update(&mut self, sym: &Symbol, val: Value) -> bool {
        if self.bindings.contains_key(sym) {
            self.bindings.insert(sym.clone(), val);
            true
        } else if let Some(parent) = &mut self.parent {
            parent.update(sym, val)
        } else {
            false
        }
    }
}

pub struct Evaluator {
    pub ns: NamespaceRegistry,
    pub thread_pool: ThreadPool,
    pub gc_heap: Arc<GcHeap>,
    collector: Option<CollectorHandle>,
    macroexpand_cache: HashMap<Symbol, Value>,
    form_cache: HashMap<u64, Vec<Value>>,
    in_tail_position: bool,
    pending_tail_call: Option<(FnValue, Vec<Value>)>,
}

thread_local! {
    static EVALUATOR_PTR: Cell<*mut Evaluator> = Cell::new(std::ptr::null_mut());
}

#[allow(unsafe_code)]
pub fn with_evaluator<R>(f: impl FnOnce(&mut Evaluator) -> R) -> R {
    EVALUATOR_PTR.with(|cell| {
        let ptr = cell.get();
        assert!(
            !ptr.is_null(),
            "with_evaluator called outside native function context"
        );
        let eval = unsafe { &mut *ptr };
        f(eval)
    })
}

impl Evaluator {
    pub fn new() -> Self {
        let gc_heap = GcHeap::new();
        let collector =
            GcHeap::start_collector(Arc::clone(&gc_heap), std::time::Duration::from_millis(100));
        let mut eval = Self {
            ns: NamespaceRegistry::new(),
            thread_pool: ThreadPool::new(4),
            gc_heap,
            collector: Some(collector),
            macroexpand_cache: HashMap::new(),
            form_cache: HashMap::new(),
            in_tail_position: false,
            pending_tail_call: None,
        };
        eval.register_builtins();
        eval
    }

    pub fn eval(&mut self, form: &Value) -> Result<Value, EvalError> {
        let env = Env::new();
        self.eval_in(env, form)
    }

    /// Parse source with a form cache. Returns cached forms if the input
    /// hash matches a previous call.
    pub fn read_cached(&mut self, input: &str) -> Result<Vec<Value>, EvalError> {
        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        let hash = hasher.finish();
        if let Some(forms) = self.form_cache.get(&hash) {
            return Ok(forms.clone());
        }
        let forms = crate::reader::read_all(input)
            .map_err(|e| EvalError::Custom(format!("read error: {}", e)))?;
        self.form_cache.insert(hash, forms.clone());
        Ok(forms)
    }

    /// Clear the form cache (useful if memory is a concern or after init).
    pub fn clear_form_cache(&mut self) {
        self.form_cache.clear();
    }

    pub fn eval_in(&mut self, env: Env, form: &Value) -> Result<Value, EvalError> {
        match form {
            Value::Nil | Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::Char(_) => {
                Ok(form.clone())
            }
            Value::String(s) => Ok(Value::String(s.clone())),
            Value::Keyword(_) => Ok(form.clone()),
            Value::Symbol(sym) => self.resolve_symbol(env, sym),
            Value::List(list) if !list.is_empty() => self.eval_list(env, list),
            Value::List(_) => Ok(Value::list(vec![])),
            Value::Vector(v) => {
                let mut result = Vec::with_capacity(v.len());
                for item in v.iter() {
                    result.push(self.eval_in(env.clone(), item)?);
                }
                Ok(Value::vector(result))
            }
            Value::Map(m) => {
                let mut result = Vec::with_capacity(m.len());
                for (k, v) in m.iter() {
                    result.push((self.eval_in(env.clone(), k)?, self.eval_in(env.clone(), v)?));
                }
                Ok(Value::map(result))
            }
            Value::Set(s) => {
                let mut result = Vec::with_capacity(s.len());
                for item in s.iter() {
                    result.push(self.eval_in(env.clone(), item)?);
                }
                Ok(Value::set(result))
            }
            _ => Ok(form.clone()),
        }
    }

    fn resolve_symbol(&self, env: Env, sym: &Symbol) -> Result<Value, EvalError> {
        // Check local bindings first
        if let Some(val) = env.get(sym) {
            return Ok(val.clone());
        }
        // Check current namespace
        if let Some(val) = self.ns.resolve_symbol(sym) {
            return Ok(val);
        }
        // Check if it's a namespace-qualified symbol
        if sym.ns.is_some() {
            return Err(EvalError::Undefined(format!(
                "{}/{}",
                sym.ns.as_ref().unwrap(),
                sym.name
            )));
        }
        Err(EvalError::Undefined(sym.name.to_string()))
    }

    fn eval_list(&mut self, env: Env, list: &[Value]) -> Result<Value, EvalError> {
        let first = &list[0];

        // Check for special forms
        if let Value::Symbol(sym) = first {
            match sym.name.as_str() {
                "def" => return self.eval_def(env, &list[1..]),
                "if" => return self.eval_if(env, &list[1..]),
                "do" => return self.eval_do(env, &list[1..]),
                "let" => return self.eval_let(env, &list[1..]),
                "fn" => return self.eval_fn(env, &list[1..]),
                "defn" => return self.eval_defn(env, &list[1..]),
                "defcommand" => return self.eval_defcommand(env, &list[1..]),
                "quote" => return self.eval_quote(&list[1..]),
                "quasiquote" | "syntax-quote" => return self.eval_quasiquote(env, &list[1..]),
                "defmacro" => return self.eval_defmacro(env, &list[1..]),
                "macroexpand" => return self.eval_macroexpand(env, &list[1..]),
                "try" => return self.eval_try(env, &list[1..]),
                "throw" => return self.eval_throw(env, &list[1..]),
                "var" => return self.eval_var(&list[1..]),
                "set!" => return self.eval_set(env, &list[1..]),
                "apply" => return self.eval_apply(env, &list[1..]),
                "." => return self.eval_dot(env, &list[1..]),
                "ns" => return self.eval_ns(&list[1..]),
                "require" => return self.eval_require(&list[1..]),
                "refer" => return self.eval_refer(&list[1..]),
                "alias" => return self.eval_alias(&list[1..]),
                "doall" => return self.eval_doall(env, &list[1..]),
                "doseq" => return self.eval_doseq(env, &list[1..]),
                "loop" => return self.eval_loop(env, &list[1..]),
                "recur" => return self.eval_recur(env, &list[1..]),
                "atom" => return self.eval_atom(env, &list[1..]),
                "deref" => return self.eval_deref(env, &list[1..]),
                "swap!" => return self.eval_swap(env, &list[1..]),
                "reset!" => return self.eval_reset(env, &list[1..]),
                "future" => return self.eval_future(env, &list[1..]),
                "agent" => return self.eval_agent(env, &list[1..]),
                "send" => return self.eval_send(env, &list[1..]),
                "send-off" => return self.eval_send_off(env, &list[1..]),
                "await" => return self.eval_await(env, &list[1..]),
                "ref" => return self.eval_ref(env, &list[1..]),
                "alter" => return self.eval_alter(env, &list[1..]),
                "dosync" => return self.eval_dosync(env, &list[1..]),
                "thread" => return self.eval_thread(env, &list[1..]),
                "promise" => return self.eval_promise(env, &list[1..]),
                "deliver" => return self.eval_deliver(env, &list[1..]),
                "locking" => return self.eval_locking(env, &list[1..]),
                "cond" => return self.eval_cond(env, &list[1..]),
                "case" => return self.eval_case(env, &list[1..]),
                "when" => return self.eval_when(env, &list[1..]),
                "when-not" => return self.eval_when_not(env, &list[1..]),
                "and" => return self.eval_and(env, &list[1..]),
                "or" => return self.eval_or(env, &list[1..]),
                "->" => return self.eval_thread_first(env, &list[1..]),
                "->>" => return self.eval_thread_last(env, &list[1..]),
                "loop*" => return self.eval_loop_star(env, &list[1..]),
                "recur*" => return self.eval_recur_star(env, &list[1..]),
                _ => {}
            }
        }

        // Macro expansion
        if let Value::Symbol(sym) = first {
            if let Some(mac) = self.find_macro(sym) {
                let expanded = self.expand_macro(&mac, &list[1..])?;
                return self.eval_in(env, &expanded);
            }
        }

        // Function call
        let func = self.eval_in(env.clone(), first)?;
        let args = self.eval_args(env.clone(), &list[1..])?;

        match func {
            Value::Fn(f) => {
                if self.in_tail_position {
                    self.in_tail_position = false;
                    self.pending_tail_call = Some((f, args));
                    Ok(Value::Nil)
                } else {
                    self.call_fn(f, args)
                }
            }
            Value::Native(f) => {
                self.in_tail_position = false;
                EVALUATOR_PTR.with(|cell| cell.set(self as *mut Evaluator));
                let result = f(&args).map_err(|e| EvalError::Custom(e));
                EVALUATOR_PTR.with(|cell| cell.set(std::ptr::null_mut()));
                result
            }
            Value::Macro(m) => {
                let expanded = self.expand_macro_value(&m, &list[1..])?;
                self.eval_in(env, &expanded)
            }
            _ => Err(EvalError::NotAFunction(format!("{:?}", first))),
        }
    }

    fn eval_args(&mut self, env: Env, args: &[Value]) -> Result<Vec<Value>, EvalError> {
        let mut result = Vec::with_capacity(args.len());
        for arg in args {
            result.push(self.eval_in(env.clone(), arg)?);
        }
        Ok(result)
    }

    pub fn call_fn(&mut self, func: FnValue, args: Vec<Value>) -> Result<Value, EvalError> {
        let mut current_func = func;
        let mut current_args = args;

        loop {
            self.pending_tail_call = None;
            let mut env = Env::new();

            // Capture closure bindings
            let closure = current_func.closure.lock();
            for (sym, val) in closure.iter() {
                env.set(sym.clone(), val.clone());
            }
            drop(closure);

            // Bind parameters
            self.bind_params(&mut env, &current_func.params, current_args)?;

            // Evaluate body
            let body = current_func.body.clone();
            let last = body.len().saturating_sub(1);
            let mut result = Value::Nil;
            for (i, form) in body.iter().enumerate() {
                if i == last {
                    self.in_tail_position = true;
                    result = self.eval_in(env.clone(), form)?;
                    self.in_tail_position = false;
                } else {
                    self.eval_in(env.clone(), form)?;
                }
            }

            // Handle recur marker in fn context
            if let Value::List(ref list) = result {
                if !list.is_empty() {
                    if let Value::Symbol(sym) = &list[0] {
                        if *sym.name == *"recur" {
                            current_args = list[1..].to_vec();
                            continue;
                        }
                    }
                }
            }

            // Check for pending tail call (from eval_list in tail position)
            if let Some((next_func, next_args)) = self.pending_tail_call.take() {
                current_func = next_func;
                current_args = next_args;
                continue;
            }

            return Ok(result);
        }
    }

    fn bind_params(
        &self,
        env: &mut Env,
        params: &[Param],
        args: Vec<Value>,
    ) -> Result<(), EvalError> {
        let mut arg_idx = 0;
        for param in params {
            match param {
                Param::Named(sym) => {
                    if arg_idx >= args.len() {
                        return Err(EvalError::Arity {
                            expected: params.len(),
                            got: args.len(),
                        });
                    }
                    env.set(sym.clone(), args[arg_idx].clone());
                    arg_idx += 1;
                }
                Param::Rest(sym) => {
                    let rest: Vec<Value> = args[arg_idx..].to_vec();
                    env.set(sym.clone(), Value::list(rest));
                    arg_idx = args.len();
                }
                Param::DestructMap(bindings) => {
                    if arg_idx >= args.len() {
                        return Err(EvalError::Arity {
                            expected: params.len(),
                            got: args.len(),
                        });
                    }
                    let map = &args[arg_idx];
                    for (kw, param) in bindings {
                        let val = match map {
                            Value::Map(m) => m
                                .get(&Value::Keyword(kw.clone()))
                                .cloned()
                                .unwrap_or(Value::Nil),
                            _ => Value::Nil,
                        };
                        if let Param::Named(sym) = param {
                            env.set(sym.clone(), val);
                        }
                    }
                    arg_idx += 1;
                }
                Param::DestructVec(params) => {
                    if arg_idx >= args.len() {
                        return Err(EvalError::Arity {
                            expected: params.len(),
                            got: args.len(),
                        });
                    }
                    let vec = match &args[arg_idx] {
                        Value::List(v) | Value::Vector(v) => v.as_ref().clone(),
                        _ => Vec::new(),
                    };
                    self.bind_params(env, params, vec)?;
                    arg_idx += 1;
                }
            }
        }
        Ok(())
    }

    // --- Special Forms ---

    fn eval_def(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.is_empty() || args.len() > 3 {
            return Err(EvalError::SpecialForm(
                "def requires 1-3 arguments".to_string(),
            ));
        }
        let sym = match &args[0] {
            Value::Symbol(s) => s.clone(),
            _ => {
                return Err(EvalError::SpecialForm(
                    "def first arg must be a symbol".to_string(),
                ))
            }
        };

        let value = if args.len() >= 2 {
            self.eval_in(env, &args[1])?
        } else {
            Value::Nil
        };

        let ns = self.ns.current();
        let mut ns = ns.lock();
        ns.intern(&sym.name, value.clone());
        Ok(value)
    }

    fn eval_if(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() < 2 || args.len() > 3 {
            return Err(EvalError::SpecialForm(
                "if requires 2-3 arguments".to_string(),
            ));
        }
        let was_tail = self.in_tail_position;
        self.in_tail_position = false;
        let cond = self.eval_in(env.clone(), &args[0])?;
        self.in_tail_position = was_tail;
        if cond.is_truthy() {
            self.eval_in(env, &args[1])
        } else if args.len() == 3 {
            self.eval_in(env, &args[2])
        } else {
            Ok(Value::Nil)
        }
    }

    fn eval_do(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        let was_tail = self.in_tail_position;
        let last = args.len().saturating_sub(1);
        let mut result = Value::Nil;
        for (i, form) in args.iter().enumerate() {
            if i == last {
                self.in_tail_position = was_tail;
            } else {
                self.in_tail_position = false;
            }
            result = self.eval_in(env.clone(), form)?;
        }
        self.in_tail_position = false;
        Ok(result)
    }

    fn eval_let(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() < 2 {
            return Err(EvalError::SpecialForm(
                "let requires at least 2 arguments".to_string(),
            ));
        }
        let bindings = match &args[0] {
            Value::Vector(v) => v.as_ref(),
            Value::List(l) => l.as_ref(),
            _ => {
                return Err(EvalError::SpecialForm(
                    "let bindings must be a vector".to_string(),
                ))
            }
        };
        if bindings.len() % 2 != 0 {
            return Err(EvalError::SpecialForm(
                "let bindings must have even number of forms".to_string(),
            ));
        }

        let was_tail = self.in_tail_position;
        self.in_tail_position = false;
        let mut let_env = Env::extend(env);
        let mut i = 0;
        while i < bindings.len() {
            let sym = match &bindings[i] {
                Value::Symbol(s) => s.clone(),
                _ => {
                    return Err(EvalError::SpecialForm(
                        "let binding name must be a symbol".to_string(),
                    ))
                }
            };
            let val = self.eval_in(let_env.clone(), &bindings[i + 1])?;
            let_env.set(sym, val);
            i += 2;
        }

        let last = args[1..].len().saturating_sub(1);
        let mut result = Value::Nil;
        for (i, form) in args[1..].iter().enumerate() {
            if i == last {
                self.in_tail_position = was_tail;
            } else {
                self.in_tail_position = false;
            }
            result = self.eval_in(let_env.clone(), form)?;
        }
        self.in_tail_position = false;
        Ok(result)
    }

    fn eval_fn(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.is_empty() {
            return Err(EvalError::SpecialForm(
                "fn requires at least 1 argument".to_string(),
            ));
        }

        let mut name = None;
        let mut arg_start = 0;

        // Check for optional name
        if let Value::Symbol(sym) = &args[0] {
            name = Some(sym.name.to_string());
            arg_start = 1;
        }

        if arg_start >= args.len() {
            return Err(EvalError::SpecialForm(
                "fn requires parameter vector".to_string(),
            ));
        }

        let params = self.parse_params(&args[arg_start])?;
        let body: Vec<Value> = args[arg_start + 1..].to_vec();

        // Capture closure - save current env bindings
        let closure: HashMap<Symbol, Value> = env.bindings.clone();

        Ok(Value::Fn(FnValue {
            name,
            params,
            body: Arc::new(body),
            closure: Arc::new(Mutex::new(closure)),
            is_macro: false,
            meta: None,
        }))
    }

    fn eval_defn(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() < 3 {
            return Err(EvalError::SpecialForm(
                "defn requires at least 3 arguments".to_string(),
            ));
        }
        let sym = match &args[0] {
            Value::Symbol(s) => s.clone(),
            _ => {
                return Err(EvalError::SpecialForm(
                    "defn first arg must be a symbol".to_string(),
                ))
            }
        };

        let (doc, params_idx) = self.parse_optional_doc(args, 1);
        if args.len() <= params_idx {
            return Err(EvalError::SpecialForm(
                "defn requires parameter vector".to_string(),
            ));
        }
        let (interactive, body_start) = self.parse_interactive_marker(args, params_idx + 1);

        let mut fn_args = Vec::with_capacity(args.len() - params_idx + 1);
        fn_args.push(Value::symbol(sym.name.to_string()));
        fn_args.push(args[params_idx].clone());
        fn_args.extend(args[body_start..].iter().cloned());
        let fn_val = self.eval_fn(env, &fn_args)?;

        self.intern_current_and_register_command(&sym, fn_val.clone(), doc, interactive)?;
        Ok(fn_val)
    }

    fn eval_defcommand(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() < 3 {
            return Err(EvalError::SpecialForm(
                "defcommand requires a name, optional docstring, params, and body".to_string(),
            ));
        }
        let sym = match &args[0] {
            Value::Symbol(s) => s.clone(),
            _ => {
                return Err(EvalError::SpecialForm(
                    "defcommand first arg must be a symbol".to_string(),
                ))
            }
        };

        let (doc, params_idx) = self.parse_optional_doc(args, 1);
        if args.len() <= params_idx + 1 {
            return Err(EvalError::SpecialForm(
                "defcommand requires params and body".to_string(),
            ));
        }

        let mut fn_args = Vec::with_capacity(args.len() - params_idx + 1);
        fn_args.push(Value::symbol(sym.name.to_string()));
        fn_args.extend(args[params_idx..].iter().cloned());
        let fn_val = self.eval_fn(env, &fn_args)?;

        self.intern_current_and_register_command(&sym, fn_val.clone(), doc, true)?;
        Ok(fn_val)
    }

    fn parse_optional_doc(&self, args: &[Value], idx: usize) -> (Option<Value>, usize) {
        match args.get(idx) {
            Some(Value::String(s)) => (Some(Value::String(s.clone())), idx + 1),
            _ => (None, idx),
        }
    }

    fn parse_interactive_marker(&self, args: &[Value], body_start: usize) -> (bool, usize) {
        match args.get(body_start) {
            Some(Value::List(list)) if !list.is_empty() => match &list[0] {
                Value::Symbol(sym) if sym.ns.is_none() && sym.name.as_str() == "interactive" => {
                    (true, body_start + 1)
                }
                _ => (false, body_start),
            },
            _ => (false, body_start),
        }
    }

    fn intern_current_and_register_command(
        &mut self,
        sym: &Symbol,
        fn_val: Value,
        doc: Option<Value>,
        interactive: bool,
    ) -> Result<(), EvalError> {
        let current_name = self.ns.current_name();
        let command_name = format!("{}/{}", current_name, sym.name);
        {
            let ns = self.ns.current();
            let mut ns = ns.lock();
            ns.intern(&sym.name, fn_val.clone());
        }

        if interactive {
            if let Some(register) = self.ns.resolve_symbol(&Symbol {
                ns: Some(Arc::new("mora.command".to_string())),
                name: Arc::new("register!".to_string()),
            }) {
                if let Value::Native(f) = register {
                    let mut register_args = vec![Value::string(command_name), fn_val.clone()];
                    if let Some(doc) = doc {
                        register_args.push(doc);
                    }
                    f(&register_args).map_err(EvalError::Custom)?;
                }
            }
        }

        Ok(())
    }

    fn eval_quote(&self, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() != 1 {
            return Err(EvalError::SpecialForm(
                "quote requires exactly 1 argument".to_string(),
            ));
        }
        Ok(args[0].clone())
    }

    fn eval_quasiquote(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() != 1 {
            return Err(EvalError::SpecialForm(
                "quasiquote requires exactly 1 argument".to_string(),
            ));
        }
        self.expand_quasiquote(env, &args[0])
    }

    fn expand_quasiquote(&mut self, env: Env, form: &Value) -> Result<Value, EvalError> {
        match form {
            Value::List(list) if !list.is_empty() => {
                if let Value::Symbol(sym) = &list[0] {
                    match sym.name.as_str() {
                        "unquote" => {
                            if list.len() != 2 {
                                return Err(EvalError::SpecialForm(
                                    "unquote requires exactly 1 argument".to_string(),
                                ));
                            }
                            return self.eval_in(env, &list[1]);
                        }
                        "unquote-splicing" => {
                            return Err(EvalError::SpecialForm(
                                "unquote-splicing not valid outside list".to_string(),
                            ));
                        }
                        _ => {}
                    }
                }
                let mut result = Vec::new();
                for item in list.iter() {
                    if let Value::List(inner) = item {
                        if !inner.is_empty() {
                            if let Value::Symbol(sym) = &inner[0] {
                                if *sym.name == *"unquote-splicing" {
                                    if inner.len() != 2 {
                                        return Err(EvalError::SpecialForm(
                                            "unquote-splicing requires exactly 1 argument"
                                                .to_string(),
                                        ));
                                    }
                                    let val = self.eval_in(env.clone(), &inner[1])?;
                                    match val {
                                        Value::List(v) | Value::Vector(v) => {
                                            result.extend(v.iter().cloned());
                                        }
                                        _ => {
                                            return Err(EvalError::Type(
                                                "unquote-splicing requires a sequence".to_string(),
                                            ))
                                        }
                                    }
                                    continue;
                                }
                            }
                        }
                    }
                    result.push(self.expand_quasiquote(Env::new(), item)?);
                }
                Ok(Value::list(result))
            }
            _ => Ok(form.clone()),
        }
    }

    fn eval_defmacro(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() < 3 {
            return Err(EvalError::SpecialForm(
                "defmacro requires at least 3 arguments".to_string(),
            ));
        }
        let sym = match &args[0] {
            Value::Symbol(s) => s.clone(),
            _ => {
                return Err(EvalError::SpecialForm(
                    "defmacro first arg must be a symbol".to_string(),
                ))
            }
        };

        let params = self.parse_params(&args[1])?;
        let body: Vec<Value> = args[2..].to_vec();

        let mac = Value::Macro(MacroValue {
            name: Some(sym.name.to_string()),
            params,
            body: Arc::new(body),
            closure: Arc::new(Mutex::new(HashMap::new())),
        });

        let ns = self.ns.current();
        let mut ns = ns.lock();
        ns.intern(&sym.name, mac.clone());
        Ok(mac)
    }

    fn eval_macroexpand(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() != 1 {
            return Err(EvalError::SpecialForm(
                "macroexpand requires exactly 1 argument".to_string(),
            ));
        }
        let form = self.eval_in(env.clone(), &args[0])?;
        match &form {
            Value::List(list) if !list.is_empty() => {
                if let Value::Symbol(sym) = &list[0] {
                    if let Some(mac) = self.find_macro(sym) {
                        return self.expand_macro(&mac, &list[1..]);
                    }
                }
                Ok(form)
            }
            _ => Ok(form),
        }
    }

    fn eval_try(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        let mut result = Value::Nil;
        let mut catch_clause = None;
        let mut finally_clause = None;
        let mut body_forms = Vec::new();

        for arg in args {
            match arg {
                Value::List(list) if !list.is_empty() => {
                    if let Value::Symbol(sym) = &list[0] {
                        match sym.name.as_str() {
                            "catch" => catch_clause = Some(list.as_ref()),
                            "finally" => finally_clause = Some(list.as_ref()),
                            _ => body_forms.push(arg),
                        }
                    } else {
                        body_forms.push(arg);
                    }
                }
                _ => body_forms.push(arg),
            }
        }

        // Try body
        let mut error = None;
        for form in &body_forms {
            match self.eval_in(env.clone(), form) {
                Ok(val) => result = val,
                Err(e) => {
                    error = Some(e);
                    break;
                }
            }
        }

        // Catch
        if let Some(err) = error {
            if let Some(catch) = catch_clause {
                if catch.len() >= 3 {
                    let sym = match &catch[1] {
                        Value::Symbol(s) => s.clone(),
                        _ => {
                            return Err(EvalError::SpecialForm(
                                "catch binding must be a symbol".to_string(),
                            ))
                        }
                    };
                    let err_val = Value::string(format!("{}", err));
                    let mut catch_env = Env::new();
                    catch_env.set(sym, err_val);
                    for form in &catch[2..] {
                        result = self.eval_in(env.clone(), form)?;
                    }
                }
            } else {
                return Err(err);
            }
        }

        // Finally
        if let Some(finally) = finally_clause {
            for form in &finally[1..] {
                self.eval_in(env.clone(), form)?;
            }
        }

        Ok(result)
    }

    fn eval_throw(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() != 1 {
            return Err(EvalError::SpecialForm(
                "throw requires exactly 1 argument".to_string(),
            ));
        }
        let val = self.eval_in(env.clone(), &args[0])?;
        Err(EvalError::Throw(val))
    }

    fn eval_var(&self, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() != 1 {
            return Err(EvalError::SpecialForm(
                "var requires exactly 1 argument".to_string(),
            ));
        }
        // Return the var itself, not its value
        match &args[0] {
            Value::Symbol(_) => Ok(args[0].clone()),
            _ => Err(EvalError::SpecialForm(
                "var arg must be a symbol".to_string(),
            )),
        }
    }

    fn eval_set(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() != 2 {
            return Err(EvalError::SpecialForm(
                "set! requires exactly 2 arguments".to_string(),
            ));
        }
        let sym = match &args[0] {
            Value::Symbol(s) => s.clone(),
            _ => {
                return Err(EvalError::SpecialForm(
                    "set! first arg must be a symbol".to_string(),
                ))
            }
        };
        let val = self.eval_in(env.clone(), &args[1])?;

        // Try to update in current namespace
        let ns = self.ns.current();
        let ns = ns.lock();
        if let Some(var) = ns.vars.get(sym.name.as_str()) {
            var.set(val.clone());
            return Ok(val);
        }

        Err(EvalError::Undefined(sym.name.to_string()))
    }

    fn eval_apply(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() < 2 {
            return Err(EvalError::SpecialForm(
                "apply requires at least 2 arguments".to_string(),
            ));
        }
        let func = self.eval_in(env.clone(), &args[0])?;
        let mut all_args = Vec::new();
        for arg in &args[1..] {
            let val = self.eval_in(env.clone(), arg)?;
            match val {
                Value::List(v) | Value::Vector(v) => all_args.extend(v.iter().cloned()),
                _ => all_args.push(val),
            }
        }
        match func {
            Value::Fn(f) => self.call_fn(f, all_args),
            _ => Err(EvalError::NotAFunction(format!("{:?}", args[0]))),
        }
    }

    fn eval_dot(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        // Interop: (. object method args...)
        if args.len() < 2 {
            return Err(EvalError::SpecialForm(
                ". requires at least 2 arguments".to_string(),
            ));
        }
        // For now, just return Nil - full interop would need reflection
        Ok(Value::Nil)
    }

    fn eval_ns(&mut self, args: &[Value]) -> Result<Value, EvalError> {
        if args.is_empty() {
            return Err(EvalError::SpecialForm(
                "ns requires at least 1 argument".to_string(),
            ));
        }
        let sym = match &args[0] {
            Value::Symbol(s) => s.clone(),
            _ => {
                return Err(EvalError::SpecialForm(
                    "ns first arg must be a symbol".to_string(),
                ))
            }
        };
        self.ns.find_or_create(&sym.name);
        self.ns
            .set_current(&sym.name)
            .map_err(|e| EvalError::SpecialForm(e))?;
        if self.ns.is_loaded(&sym.name) {
            return Ok(Value::Nil);
        }
        self.ns
            .refer_all("mora.core", &sym.name)
            .map_err(|e| EvalError::SpecialForm(e))?;
        self.ns.mark_loaded(&sym.name);
        Ok(Value::Nil)
    }

    fn eval_require(&mut self, args: &[Value]) -> Result<Value, EvalError> {
        for arg in args {
            match arg {
                Value::Symbol(sym) => {
                    self.ns
                        .require(&sym.name, None)
                        .map_err(|e| EvalError::SpecialForm(e))?;
                }
                Value::List(list) | Value::Vector(list) if list.len() >= 2 => {
                    if let Value::Symbol(sym) = &list[0] {
                        let mut alias = None;
                        let mut i = 1;
                        while i < list.len() {
                            if let Value::Keyword(kw) = &list[i] {
                                if kw.name.as_str() == "as" {
                                    if let Some(Value::Symbol(a)) = list.get(i + 1) {
                                        alias = Some(a.name.to_string());
                                    }
                                    i += 2;
                                    continue;
                                }
                            }
                            i += 1;
                        }
                        self.ns
                            .require(&sym.name, alias.as_deref())
                            .map_err(|e| EvalError::SpecialForm(e))?;
                    }
                }
                _ => return Err(EvalError::SpecialForm("invalid require form".to_string())),
            }
        }
        Ok(Value::Nil)
    }

    fn eval_refer(&mut self, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() < 1 {
            return Err(EvalError::SpecialForm(
                "refer requires at least 1 argument".to_string(),
            ));
        }
        // Implementation: (refer 'namespace :only [names] :exclude [names])
        Ok(Value::Nil)
    }

    fn eval_alias(&mut self, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() != 2 {
            return Err(EvalError::SpecialForm(
                "alias requires exactly 2 arguments".to_string(),
            ));
        }
        let alias_sym = match &args[0] {
            Value::Symbol(s) => s.clone(),
            _ => {
                return Err(EvalError::SpecialForm(
                    "alias first arg must be a symbol".to_string(),
                ))
            }
        };
        let ns_sym = match &args[1] {
            Value::Symbol(s) => s.clone(),
            _ => {
                return Err(EvalError::SpecialForm(
                    "alias second arg must be a symbol".to_string(),
                ))
            }
        };
        let current = self.ns.current();
        let mut current = current.lock();
        current.alias(&alias_sym.name, &ns_sym.name);
        Ok(Value::Nil)
    }

    fn eval_doall(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.is_empty() {
            return Err(EvalError::SpecialForm(
                "doall requires at least 1 argument".to_string(),
            ));
        }
        let val = self.eval_in(env.clone(), &args[0])?;
        Ok(val)
    }

    fn eval_doseq(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() < 2 {
            return Err(EvalError::SpecialForm(
                "doseq requires at least 2 arguments".to_string(),
            ));
        }
        let bindings = match &args[0] {
            Value::Vector(v) => v.as_ref(),
            _ => {
                return Err(EvalError::SpecialForm(
                    "doseq bindings must be a vector".to_string(),
                ))
            }
        };
        if bindings.len() != 2 {
            return Err(EvalError::SpecialForm(
                "doseq requires exactly 2 binding forms".to_string(),
            ));
        }
        let sym = match &bindings[0] {
            Value::Symbol(s) => s.clone(),
            _ => {
                return Err(EvalError::SpecialForm(
                    "doseq binding must be a symbol".to_string(),
                ))
            }
        };
        let seq = self.eval_in(env.clone(), &bindings[1])?;
        let items = match seq {
            Value::List(v) | Value::Vector(v) => v.as_ref().clone(),
            _ => return Err(EvalError::Type("doseq requires a sequence".to_string())),
        };
        let mut result = Value::Nil;
        for item in items {
            let mut loop_env = Env::extend(env.clone());
            loop_env.set(sym.clone(), item);
            for form in &args[1..] {
                result = self.eval_in(loop_env.clone(), form)?;
            }
        }
        Ok(result)
    }

    fn eval_loop(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        // (loop [bindings] body)
        if args.len() < 2 {
            return Err(EvalError::SpecialForm(
                "loop requires at least 2 arguments".to_string(),
            ));
        }
        self.eval_loop_star(env, args)
    }

    fn eval_loop_star(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() < 2 {
            return Err(EvalError::SpecialForm(
                "loop* requires at least 2 arguments".to_string(),
            ));
        }
        let bindings = match &args[0] {
            Value::Vector(v) => v.as_ref(),
            _ => {
                return Err(EvalError::SpecialForm(
                    "loop bindings must be a vector".to_string(),
                ))
            }
        };
        if bindings.len() % 2 != 0 {
            return Err(EvalError::SpecialForm(
                "loop bindings must have even number of forms".to_string(),
            ));
        }

        let mut loop_env = Env::extend(env.clone());
        let mut i = 0;
        while i < bindings.len() {
            let sym = match &bindings[i] {
                Value::Symbol(s) => s.clone(),
                _ => {
                    return Err(EvalError::SpecialForm(
                        "loop binding name must be a symbol".to_string(),
                    ))
                }
            };
            let val = self.eval_in(loop_env.clone(), &bindings[i + 1])?;
            loop_env.set(sym, val);
            i += 2;
        }

        // Simple loop - no TCO yet
        loop {
            let mut result = Value::Nil;
            for form in &args[1..] {
                result = self.eval_in(loop_env.clone(), form)?;
            }
            // Check if result is a recur marker
            match result {
                Value::List(list) if !list.is_empty() => {
                    if let Value::Symbol(sym) = &list[0] {
                        if *sym.name == *"recur" {
                            // Rebind loop variables
                            let recur_args = &list[1..];
                            if recur_args.len() != bindings.len() / 2 {
                                return Err(EvalError::Arity {
                                    expected: bindings.len() / 2,
                                    got: recur_args.len(),
                                });
                            }
                            let mut j = 0;
                            while j < bindings.len() {
                                let sym = match &bindings[j] {
                                    Value::Symbol(s) => s.clone(),
                                    _ => unreachable!(),
                                };
                                let val = self.eval_in(env.clone(), &recur_args[j / 2])?;
                                loop_env.set(sym, val);
                                j += 2;
                            }
                            continue;
                        }
                    }
                    return Ok(Value::list(list.as_ref().clone()));
                }
                _ => return Ok(result),
            }
        }
    }

    fn eval_recur(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        // recur is handled by loop*
        let mut vals = vec![Value::symbol("recur")];
        for arg in args {
            vals.push(self.eval_in(env.clone(), arg)?);
        }
        Ok(Value::list(vals))
    }

    fn eval_recur_star(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        self.eval_recur(env, args)
    }

    fn eval_cond(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() % 2 != 0 {
            return Err(EvalError::SpecialForm(
                "cond requires even number of arguments".to_string(),
            ));
        }
        let was_tail = self.in_tail_position;
        let mut i = 0;
        while i < args.len() {
            self.in_tail_position = false;
            let cond = self.eval_in(env.clone(), &args[i])?;
            if cond.is_truthy() {
                self.in_tail_position = was_tail;
                return self.eval_in(env.clone(), &args[i + 1]);
            }
            i += 2;
        }
        self.in_tail_position = false;
        Ok(Value::Nil)
    }

    fn eval_case(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() < 2 || args.len() % 2 != 0 {
            return Err(EvalError::SpecialForm(
                "case requires test-expr and pairs".to_string(),
            ));
        }
        let was_tail = self.in_tail_position;
        self.in_tail_position = false;
        let test = self.eval_in(env.clone(), &args[0])?;
        let mut i = 1;
        while i < args.len() - 1 {
            let val = self.eval_in(env.clone(), &args[i])?;
            if test == val {
                self.in_tail_position = was_tail;
                return self.eval_in(env.clone(), &args[i + 1]);
            }
            i += 2;
        }
        // Default clause
        if i < args.len() {
            self.in_tail_position = was_tail;
            self.eval_in(env.clone(), &args[i])
        } else {
            Ok(Value::Nil)
        }
    }

    fn eval_when(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() < 2 {
            return Err(EvalError::SpecialForm(
                "when requires at least 2 arguments".to_string(),
            ));
        }
        let was_tail = self.in_tail_position;
        self.in_tail_position = false;
        let cond = self.eval_in(env.clone(), &args[0])?;
        if cond.is_truthy() {
            let last = args[1..].len().saturating_sub(1);
            let mut result = Value::Nil;
            for (i, form) in args[1..].iter().enumerate() {
                if i == last {
                    self.in_tail_position = was_tail;
                } else {
                    self.in_tail_position = false;
                }
                result = self.eval_in(env.clone(), form)?;
            }
            self.in_tail_position = false;
            Ok(result)
        } else {
            self.in_tail_position = false;
            Ok(Value::Nil)
        }
    }

    fn eval_when_not(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() < 2 {
            return Err(EvalError::SpecialForm(
                "when-not requires at least 2 arguments".to_string(),
            ));
        }
        let was_tail = self.in_tail_position;
        self.in_tail_position = false;
        let cond = self.eval_in(env.clone(), &args[0])?;
        if !cond.is_truthy() {
            let last = args[1..].len().saturating_sub(1);
            let mut result = Value::Nil;
            for (i, form) in args[1..].iter().enumerate() {
                if i == last {
                    self.in_tail_position = was_tail;
                } else {
                    self.in_tail_position = false;
                }
                result = self.eval_in(env.clone(), form)?;
            }
            self.in_tail_position = false;
            Ok(result)
        } else {
            self.in_tail_position = false;
            Ok(Value::Nil)
        }
    }

    fn eval_and(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        let was_tail = self.in_tail_position;
        let mut result = Value::Bool(true);
        for (i, arg) in args.iter().enumerate() {
            if i == args.len() - 1 {
                self.in_tail_position = was_tail;
            } else {
                self.in_tail_position = false;
            }
            result = self.eval_in(env.clone(), arg)?;
            if !result.is_truthy() {
                self.in_tail_position = false;
                return Ok(result);
            }
        }
        self.in_tail_position = false;
        Ok(result)
    }

    fn eval_or(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        let was_tail = self.in_tail_position;
        let mut result = Value::Nil;
        for (i, arg) in args.iter().enumerate() {
            if i == args.len() - 1 {
                self.in_tail_position = was_tail;
            } else {
                self.in_tail_position = false;
            }
            result = self.eval_in(env.clone(), arg)?;
            if result.is_truthy() {
                self.in_tail_position = false;
                return Ok(result);
            }
        }
        self.in_tail_position = false;
        Ok(result)
    }

    // --- Concurrency Primitives ---

    fn eval_atom(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() != 1 {
            return Err(EvalError::SpecialForm(
                "atom requires exactly 1 argument".to_string(),
            ));
        }
        let val = self.eval_in(env.clone(), &args[0])?;
        Ok(Value::atom(val))
    }

    fn eval_deref(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() != 1 {
            return Err(EvalError::SpecialForm(
                "deref requires exactly 1 argument".to_string(),
            ));
        }
        let val = self.eval_in(env.clone(), &args[0])?;
        match val {
            Value::Atom(a) => Ok(a.lock().clone()),
            Value::Future(f) => {
                if f.done.load(std::sync::atomic::Ordering::SeqCst) {
                    let result = f.result.lock();
                    match result.as_ref() {
                        Some(Ok(v)) => Ok(v.clone()),
                        Some(Err(e)) => Err(EvalError::Custom(format!("future error: {}", e))),
                        None => Ok(Value::Nil),
                    }
                } else {
                    // Block until done
                    while !f.done.load(std::sync::atomic::Ordering::SeqCst) {
                        std::thread::yield_now();
                    }
                    let result = f.result.lock();
                    match result.as_ref() {
                        Some(Ok(v)) => Ok(v.clone()),
                        Some(Err(e)) => Err(EvalError::Custom(format!("future error: {}", e))),
                        None => Ok(Value::Nil),
                    }
                }
            }
            Value::Ref(r) => Ok(r.state.lock().clone()),
            Value::Promise(p) => {
                while !p.done.load(std::sync::atomic::Ordering::SeqCst) {
                    std::thread::yield_now();
                }
                let result = p.result.lock();
                Ok(result.clone().unwrap_or(Value::Nil))
            }
            _ => Err(EvalError::Type("cannot deref this type".to_string())),
        }
    }

    fn eval_swap(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() < 2 {
            return Err(EvalError::SpecialForm(
                "swap! requires at least 2 arguments".to_string(),
            ));
        }
        let atom = self.eval_in(env.clone(), &args[0])?;
        let func = self.eval_in(env.clone(), &args[1])?;
        let extra_args = self.eval_args(Env::new(), &args[2..])?;

        match atom {
            Value::Atom(a) => {
                let current = a.lock().clone();
                let mut all_args = vec![current];
                all_args.extend(extra_args);
                let new_val = match func {
                    Value::Fn(f) => self.call_fn(f, all_args)?,
                    _ => {
                        return Err(EvalError::NotAFunction(
                            "swap! second arg must be a function".to_string(),
                        ))
                    }
                };
                *a.lock() = new_val.clone();
                Ok(new_val)
            }
            _ => Err(EvalError::Type(
                "swap! first arg must be an atom".to_string(),
            )),
        }
    }

    fn eval_reset(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() != 2 {
            return Err(EvalError::SpecialForm(
                "reset! requires exactly 2 arguments".to_string(),
            ));
        }
        let atom = self.eval_in(env.clone(), &args[0])?;
        let val = self.eval_in(env.clone(), &args[1])?;
        match atom {
            Value::Atom(a) => {
                *a.lock() = val.clone();
                Ok(val)
            }
            _ => Err(EvalError::Type(
                "reset! first arg must be an atom".to_string(),
            )),
        }
    }

    fn eval_future(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.is_empty() {
            return Err(EvalError::SpecialForm(
                "future requires at least 1 argument".to_string(),
            ));
        }
        let body: Vec<Value> = args.to_vec();
        let future = Value::future();
        if let Value::Future(ref f) = future {
            let f_clone = f.clone();
            let body_clone = body.clone();
            // Spawn on thread pool
            self.thread_pool.spawn(move || {
                let mut eval = Evaluator::new();
                let mut result = Value::Nil;
                for form in &body_clone {
                    match eval.eval(form) {
                        Ok(val) => result = val,
                        Err(e) => {
                            *f_clone.result.lock() = Some(Err(e));
                            f_clone
                                .done
                                .store(true, std::sync::atomic::Ordering::SeqCst);
                            return;
                        }
                    }
                }
                *f_clone.result.lock() = Some(Ok(result));
                f_clone
                    .done
                    .store(true, std::sync::atomic::Ordering::SeqCst);
            });
        }
        Ok(future)
    }

    fn eval_agent(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() != 1 {
            return Err(EvalError::SpecialForm(
                "agent requires exactly 1 argument".to_string(),
            ));
        }
        let val = self.eval_in(env.clone(), &args[0])?;
        Ok(Value::agent(val))
    }

    fn eval_send(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() < 2 {
            return Err(EvalError::SpecialForm(
                "send requires at least 2 arguments".to_string(),
            ));
        }
        let agent = self.eval_in(env.clone(), &args[0])?;
        let func = self.eval_in(env.clone(), &args[1])?;
        let extra_args = self.eval_args(Env::new(), &args[2..])?;

        match agent {
            Value::Agent(a) => {
                let state = a.state.lock().clone();
                let mut all_args = vec![state];
                all_args.extend(extra_args);
                let new_val = match func {
                    Value::Fn(f) => self.call_fn(f, all_args)?,
                    _ => {
                        return Err(EvalError::NotAFunction(
                            "send second arg must be a function".to_string(),
                        ))
                    }
                };
                *a.state.lock() = new_val;
                Ok(Value::Agent(a))
            }
            _ => Err(EvalError::Type(
                "send first arg must be an agent".to_string(),
            )),
        }
    }

    fn eval_send_off(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        // send-off is like send but for blocking operations
        self.eval_send(env, args)
    }

    fn eval_await(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() != 1 {
            return Err(EvalError::SpecialForm(
                "await requires exactly 1 argument".to_string(),
            ));
        }
        let agent = self.eval_in(env.clone(), &args[0])?;
        match agent {
            Value::Agent(a) => Ok(a.state.lock().clone()),
            _ => Err(EvalError::Type("await arg must be an agent".to_string())),
        }
    }

    fn eval_ref(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() != 1 {
            return Err(EvalError::SpecialForm(
                "ref requires exactly 1 argument".to_string(),
            ));
        }
        let val = self.eval_in(env.clone(), &args[0])?;
        Ok(Value::ref_value(val))
    }

    fn eval_alter(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() < 2 {
            return Err(EvalError::SpecialForm(
                "alter requires at least 2 arguments".to_string(),
            ));
        }
        let ref_val = self.eval_in(env.clone(), &args[0])?;
        let func = self.eval_in(env.clone(), &args[1])?;
        let extra_args = self.eval_args(Env::new(), &args[2..])?;

        match ref_val {
            Value::Ref(r) => {
                let current = r.state.lock().clone();
                let mut all_args = vec![current];
                all_args.extend(extra_args);
                let new_val = match func {
                    Value::Fn(f) => self.call_fn(f, all_args)?,
                    _ => {
                        return Err(EvalError::NotAFunction(
                            "alter second arg must be a function".to_string(),
                        ))
                    }
                };
                *r.state.lock() = new_val.clone();
                Ok(new_val)
            }
            _ => Err(EvalError::Type("alter first arg must be a ref".to_string())),
        }
    }

    fn eval_dosync(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        // Simplified STM: just execute body sequentially
        let mut result = Value::Nil;
        for form in args {
            result = self.eval_in(env.clone(), form)?;
        }
        Ok(result)
    }

    fn eval_thread(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.is_empty() {
            return Err(EvalError::SpecialForm(
                "thread requires at least 1 argument".to_string(),
            ));
        }
        let body: Vec<Value> = args.to_vec();
        let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let done_clone = done.clone();

        let handle = std::thread::spawn(move || {
            let mut eval = Evaluator::new();
            for form in &body {
                let _ = eval.eval(form);
            }
            done_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        Ok(Value::Thread(crate::types::ThreadValue {
            id: crate::types::next_id(),
            handle: Arc::new(Mutex::new(Some(handle))),
            done,
        }))
    }

    fn eval_promise(&mut self, env: Env, _args: &[Value]) -> Result<Value, EvalError> {
        Ok(Value::promise())
    }

    fn eval_deliver(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() != 2 {
            return Err(EvalError::SpecialForm(
                "deliver requires exactly 2 arguments".to_string(),
            ));
        }
        let promise = self.eval_in(env.clone(), &args[0])?;
        let val = self.eval_in(env.clone(), &args[1])?;
        match promise {
            Value::Promise(p) => {
                *p.result.lock() = Some(val.clone());
                p.done.store(true, std::sync::atomic::Ordering::SeqCst);
                // Wake waiters
                for waiter in p.waiters.lock().iter() {
                    waiter.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                Ok(val)
            }
            _ => Err(EvalError::Type(
                "deliver first arg must be a promise".to_string(),
            )),
        }
    }

    fn eval_locking(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.len() < 2 {
            return Err(EvalError::SpecialForm(
                "locking requires at least 2 arguments".to_string(),
            ));
        }
        let lock_obj = self.eval_in(env.clone(), &args[0])?;
        match lock_obj {
            Value::Atom(a) => {
                let _guard = a.lock();
                let mut result = Value::Nil;
                for form in &args[1..] {
                    result = self.eval_in(env.clone(), form)?;
                }
                Ok(result)
            }
            _ => Err(EvalError::Type(
                "locking first arg must be lockable".to_string(),
            )),
        }
    }

    // --- Helpers ---

    fn parse_params(&self, form: &Value) -> Result<Vec<Param>, EvalError> {
        let params = match form {
            Value::Vector(v) => v.as_ref(),
            Value::List(l) => l.as_ref(),
            _ => {
                return Err(EvalError::SpecialForm(
                    "parameter list must be a vector".to_string(),
                ))
            }
        };

        let mut result = Vec::new();
        let mut i = 0;
        while i < params.len() {
            match &params[i] {
                Value::Symbol(sym) => {
                    if *sym.name == *"&" {
                        // Rest parameter
                        i += 1;
                        if i >= params.len() {
                            return Err(EvalError::SpecialForm(
                                "& must be followed by a symbol".to_string(),
                            ));
                        }
                        let rest_sym = match &params[i] {
                            Value::Symbol(s) => s.clone(),
                            _ => {
                                return Err(EvalError::SpecialForm(
                                    "& must be followed by a symbol".to_string(),
                                ))
                            }
                        };
                        result.push(Param::Rest(rest_sym));
                    } else {
                        result.push(Param::Named(sym.clone()));
                    }
                }
                Value::Vector(v) => {
                    // Destructuring vector
                    let inner_params = self.parse_params(&Value::Vector(v.clone()))?;
                    result.push(Param::DestructVec(inner_params));
                }
                Value::Map(m) => {
                    // Destructuring map
                    let mut bindings = Vec::new();
                    for (k, v) in m.iter() {
                        if let (Value::Keyword(kw), Value::Symbol(sym)) = (k, v) {
                            bindings.push((kw.clone(), Param::Named(sym.clone())));
                        }
                    }
                    result.push(Param::DestructMap(bindings));
                }
                _ => {
                    return Err(EvalError::SpecialForm(
                        "parameter must be a symbol or destructuring form".to_string(),
                    ))
                }
            }
            i += 1;
        }
        Ok(result)
    }

    fn eval_thread_first(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.is_empty() {
            return Err(EvalError::SpecialForm(
                "-> requires at least 1 argument".to_string(),
            ));
        }

        let mut result = self.eval_in(env.clone(), &args[0])?;

        for form in &args[1..] {
            match form {
                Value::List(list) if !list.is_empty() => {
                    let mut new_list = Vec::with_capacity(list.len() + 1);
                    new_list.push(list[0].clone());
                    new_list.push(result);
                    new_list.extend_from_slice(&list[1..]);
                    result = self.eval_in(env.clone(), &Value::list(new_list))?;
                }
                Value::Symbol(_) => {
                    result = self.eval_in(env.clone(), &Value::list(vec![form.clone(), result]))?;
                }
                _ => {
                    return Err(EvalError::SpecialForm(
                        "-> expects a list or symbol".to_string(),
                    ))
                }
            }
        }

        Ok(result)
    }

    fn eval_thread_last(&mut self, env: Env, args: &[Value]) -> Result<Value, EvalError> {
        if args.is_empty() {
            return Err(EvalError::SpecialForm(
                "->> requires at least 1 argument".to_string(),
            ));
        }

        let mut result = self.eval_in(env.clone(), &args[0])?;

        for form in &args[1..] {
            match form {
                Value::List(list) if !list.is_empty() => {
                    let mut new_list = Vec::with_capacity(list.len() + 1);
                    new_list.push(list[0].clone());
                    new_list.extend_from_slice(&list[1..]);
                    new_list.push(result);
                    result = self.eval_in(env.clone(), &Value::list(new_list))?;
                }
                Value::Symbol(_) => {
                    result = self.eval_in(env.clone(), &Value::list(vec![form.clone(), result]))?;
                }
                _ => {
                    return Err(EvalError::SpecialForm(
                        "->> expects a list or symbol".to_string(),
                    ))
                }
            }
        }

        Ok(result)
    }

    fn find_macro(&self, sym: &Symbol) -> Option<MacroValue> {
        let ns = self.ns.current();
        let ns = ns.lock();
        if let Some(var) = ns.find_var(&sym.name) {
            if let Value::Macro(m) = var.deref() {
                return Some(m);
            }
        }
        None
    }

    fn expand_macro(&mut self, mac: &MacroValue, args: &[Value]) -> Result<Value, EvalError> {
        let mut env = Env::new();
        self.bind_params(&mut env, &mac.params, args.to_vec())?;

        let body = mac.body.clone();
        let _last = body.len().saturating_sub(1);
        let mut result = Value::Nil;
        for (_i, form) in body.iter().enumerate() {
            result = self.eval_in(env.clone(), form)?;
        }
        Ok(result)
    }

    fn expand_macro_value(&mut self, mac: &MacroValue, args: &[Value]) -> Result<Value, EvalError> {
        self.expand_macro(mac, args)
    }

    fn register_builtins(&mut self) {
        let core = self.ns.find_or_create("mora.core");
        let mut ns = core.lock();
        crate::types::builtin::register_builtins(&mut ns);
        drop(ns);
        self.ns
            .refer_all("mora.core", "user")
            .expect("mora.core and user namespaces exist");
    }
}
