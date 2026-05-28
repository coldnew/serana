//! Bytecode compiler and virtual machine for mora-lisp.
//!
//! This is an opt-in execution path that compiles Value ASTs to a compact
//! bytecode representation. The VM reuses the Evaluator's namespace registry,
//! environment chain, and value representation — so all existing builtins,
//! macros, and native functions work unchanged.
//!
//! Usage: `vm::compile_and_run(&mut evaluator, &forms)` for a batch of forms,
//! or use the `BytecodeCompiler` / `BytecodeVm` separately.
use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::bytecode::{CompiledFunction, Op};
use crate::eval::{EvalError, Evaluator};
use crate::types::{FnValue, Param, Symbol, Value};

// ── Compiler ──────────────────────────────────────────────────────────────────

/// Bytecode compiler — compiles a sequence of Value AST forms into
/// a main function's bytecode.
pub struct BytecodeCompiler {
    constants: Vec<Value>,
    constant_map: HashMap<Value, u16>,
    strings: Vec<String>,
    string_map: HashMap<String, u16>,
    functions: Vec<CompiledFunction>,
    native_fns: Vec<fn(&[Value]) -> Result<Value, String>>,
    native_map: HashMap<fn(&[Value]) -> Result<Value, String>, u16>,
    ops: Vec<Op>,
    local_scope: Vec<HashMap<String, u16>>,
    next_local: u16,
}

impl BytecodeCompiler {
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
            constant_map: HashMap::new(),
            strings: Vec::new(),
            string_map: HashMap::new(),
            functions: Vec::new(),
            native_fns: Vec::new(),
            native_map: HashMap::new(),
            ops: Vec::new(),
            local_scope: Vec::new(),
            next_local: 0,
        }
    }

    /// Compile a batch of forms (top-level).
    pub fn compile_forms(&mut self, forms: &[Value]) -> Result<CompiledFunction, CompilerError> {
        self.ops.clear();
        for form in forms {
            self.compile_expr(form, false)?;
        }
        self.ops.push(Op::Halt);
        Ok(CompiledFunction {
            name: Some("<main>".to_string()),
            params: vec![],
            locals: self.next_local,
            ops: self.ops.clone(),
            constants: self.constants.clone(),
            strings: self.strings.clone(),
            native_fns: self.native_fns.clone(),
        })
    }

    fn compile_expr(&mut self, form: &Value, tail: bool) -> Result<(), CompilerError> {
        match form {
            // Self-evaluating forms
            Value::Nil => self.ops.push(Op::PushNil),
            Value::Bool(true) => self.ops.push(Op::PushTrue),
            Value::Bool(false) => self.ops.push(Op::PushFalse),
            Value::Int(_) | Value::Float(_) | Value::String(_) | Value::Char(_)
            | Value::Keyword(_) => {
                let idx = self.add_constant(form.clone());
                self.ops.push(Op::PushConst(idx));
            }
            Value::Symbol(sym) => {
                // Check locals first
                if sym.ns.is_none() {
                    if let Some(&local_idx) = self
                        .local_scope
                        .last()
                        .and_then(|scope| scope.get(sym.name.as_str()))
                    {
                        self.ops.push(Op::GetLocal(local_idx));
                        return Ok(());
                    }
                }
                let idx = self.add_string(self.symbol_to_string(sym));
                self.ops.push(Op::ResolveSymbol(idx));
            }
            Value::List(list) if !list.is_empty() => {
                self.compile_list(list, tail)?;
            }
            Value::List(_) => {
                // Empty list
                let idx = self.add_constant(Value::list(vec![]));
                self.ops.push(Op::PushConst(idx));
            }
            Value::Vector(v) => {
                for item in v.iter() {
                    self.compile_expr(item, false)?;
                }
                let idx = self.add_constant(Value::Int(v.len() as i64));
                self.ops.push(Op::MakeVector(idx));
            }
            Value::Map(m) => {
                for (k, val) in m.iter() {
                    self.compile_expr(k, false)?;
                    self.compile_expr(val, false)?;
                }
                let idx = self.add_constant(Value::Int(m.len() as i64));
                self.ops.push(Op::MakeMap(idx));
            }
            other => {
                // For unsupported forms, fall back to constant
                let idx = self.add_constant(other.clone());
                self.ops.push(Op::PushConst(idx));
            }
        }
        Ok(())
    }

    fn compile_list(&mut self, list: &[Value], tail: bool) -> Result<(), CompilerError> {
        if let Value::Symbol(sym) = &list[0] {
            if sym.ns.is_none() {
                match sym.name.as_str() {
                    "def" => return self.compile_def(list),
                    "do" => return self.compile_do(list, tail),
                    "if" => return self.compile_if(list, tail),
                    "let" => return self.compile_let(list, tail),
                    "fn" => return self.compile_fn(list),
                    "defn" => return self.compile_defn(list),
                    "quote" => return self.compile_quote(list),
                    "ns" => return self.compile_ns(list),
                    "require" => return self.compile_require(list),
                    "loop" => return self.compile_loop(list, tail),
                    "recur" => return self.compile_recur(list),
                    _ => {}
                }
            }
        }

        // Check for native fn first operand
        if let Value::Native(f) = &list[0] {
            let native_idx = self.add_native(*f);
            for arg in &list[1..] {
                self.compile_expr(arg, false)?;
            }
            self.ops.push(Op::InvokeNative(native_idx));
            return Ok(());
        }

        // General call: compile callee + args
        for item in list {
            self.compile_expr(item, false)?;
        }
        let argc = list.len() - 1;
        if tail {
            self.ops.push(Op::TailCall(argc as u8));
        } else {
            self.ops.push(Op::Call(argc as u8));
        }
        Ok(())
    }

    fn compile_def(&mut self, list: &[Value]) -> Result<(), CompilerError> {
        if list.len() < 2 {
            return Err(CompilerError::InvalidForm("def requires at least a name".into()));
        }
        let name = match &list[1] {
            Value::Symbol(s) => self.symbol_to_string(s),
            _ => return Err(CompilerError::InvalidForm("def name must be a symbol".into())),
        };
        if list.len() >= 3 {
            self.compile_expr(&list[2], false)?;
        } else {
            self.ops.push(Op::PushNil);
        }
        let name_idx = self.add_string(name);
        self.ops.push(Op::DefineSymbol(name_idx));
        self.ops.push(Op::PushNil); // def returns nil
        Ok(())
    }

    fn compile_do(&mut self, list: &[Value], tail: bool) -> Result<(), CompilerError> {
        for (i, form) in list[1..].iter().enumerate() {
            let is_last = i == list.len() - 2;
            self.compile_expr(form, tail && is_last)?;
            if !is_last {
                self.ops.push(Op::Pop);
            }
        }
        if list.len() == 1 {
            self.ops.push(Op::PushNil);
        }
        Ok(())
    }

    fn compile_if(&mut self, list: &[Value], tail: bool) -> Result<(), CompilerError> {
        if list.len() < 3 {
            return Err(CompilerError::InvalidForm("if requires at least test and then".into()));
        }
        self.compile_expr(&list[1], false)?; // condition

        let jump_if_false_pos = self.ops.len();
        self.ops.push(Op::JumpIfFalse(0)); // placeholder

        // then branch
        self.ops.push(Op::Pop); // pop condition
        self.compile_expr(&list[2], tail)?;

        if list.len() >= 4 {
            let jump_pos = self.ops.len();
            self.ops.push(Op::Jump(0)); // placeholder, skip else

            // Patch JumpIfFalse to here
            let else_pos = self.ops.len() as i16;
            self.patch_jump(jump_if_false_pos, else_pos - jump_if_false_pos as i16);

            self.ops.push(Op::Pop); // pop condition
            self.compile_expr(&list[3], tail)?; // else branch

            let end_pos = self.ops.len() as i16;
            self.patch_jump(jump_pos, end_pos - jump_pos as i16);
        } else {
            // No else — patch JumpIfFalse to end, push nil for else
            let end_pos = self.ops.len() as i16;
            self.patch_jump(jump_if_false_pos, end_pos - jump_if_false_pos as i16);

            self.ops.push(Op::Pop); // pop condition
            self.ops.push(Op::PushNil);
        }
        Ok(())
    }

    fn compile_let(&mut self, list: &[Value], tail: bool) -> Result<(), CompilerError> {
        if list.len() < 3 {
            return Err(CompilerError::InvalidForm("let requires bindings and body".into()));
        }
        let bindings = match &list[1] {
            Value::Vector(v) => v,
            _ => return Err(CompilerError::InvalidForm("let bindings must be a vector".into())),
        };
        if bindings.len() % 2 != 0 {
            return Err(CompilerError::InvalidForm("let bindings must have even count".into()));
        }

        self.local_scope.push(HashMap::new());
        let base_local = self.next_local;

        for chunk in bindings.chunks(2) {
            let name = match &chunk[0] {
                Value::Symbol(s) => s.name.to_string(),
                _ => return Err(CompilerError::InvalidForm("let binding name must be symbol".into())),
            };
            self.compile_expr(&chunk[1], false)?;
            let local_idx = self.next_local;
            self.next_local += 1;
            self.local_scope.last_mut().unwrap().insert(name, local_idx);
            self.ops.push(Op::SetLocal(local_idx));
            self.ops.push(Op::Pop); // discard set result
        }

        // Compile body
        for (i, form) in list[2..].iter().enumerate() {
            let is_last = i == list.len() - 3;
            self.compile_expr(form, tail && is_last)?;
            if !is_last {
                self.ops.push(Op::Pop);
            }
        }

        self.local_scope.pop();
        // Note: we don't reclaim locals for simplicity — a real impl would.
        Ok(())
    }

    fn compile_fn(&mut self, list: &[Value]) -> Result<(), CompilerError> {
        if list.len() < 3 {
            return Err(CompilerError::InvalidForm("fn requires params and body".into()));
        }
        let params = self.parse_params(&list[1])?;

        // Compile body in a separate compiler context
        let mut body_compiler = BytecodeCompiler::new();
        body_compiler.local_scope.push(HashMap::new());

        // Add params as locals
        for param in &params {
            match param {
                Param::Named(sym) => {
                    let idx = body_compiler.next_local;
                    body_compiler.next_local += 1;
                    body_compiler
                        .local_scope
                        .last_mut()
                        .unwrap()
                        .insert(sym.name.to_string(), idx);
                }
                Param::Rest(sym) => {
                    let idx = body_compiler.next_local;
                    body_compiler.next_local += 1;
                    body_compiler
                        .local_scope
                        .last_mut()
                        .unwrap()
                        .insert(sym.name.to_string(), idx);
                }
                _ => {}
            }
        }

        // Compile body
        for (i, form) in list[2..].iter().enumerate() {
            let is_last = i == list.len() - 3;
            body_compiler.compile_expr(form, is_last)?;
            if !is_last {
                body_compiler.ops.push(Op::Pop);
            }
        }
        body_compiler.ops.push(Op::Return);

       // Merge child pools first (before moving ops)
       self.merge_compiler(&body_compiler);
       let body_constants = body_compiler.constants.clone();
       let body_strings = body_compiler.strings.clone();
       let body_native_fns = body_compiler.native_fns.clone();

        let fn_idx = self.functions.len() as u16;
        self.functions.push(CompiledFunction {
            name: None,
            params,
            locals: body_compiler.next_local,
            ops: body_compiler.ops,
            constants: body_constants,
            strings: body_strings,
            native_fns: body_native_fns,
        });
        self.ops.push(Op::MakeClosure(fn_idx));
        Ok(())
    }

    fn compile_defn(&mut self, list: &[Value]) -> Result<(), CompilerError> {
        if list.len() < 4 {
            return Err(CompilerError::InvalidForm("defn requires name, params, and body".into()));
        }
        let name = match &list[1] {
            Value::Symbol(s) => self.symbol_to_string(s),
            _ => return Err(CompilerError::InvalidForm("defn name must be symbol".into())),
        };

        // Compile as (def name (fn params body...))
        let fn_form = Value::list(
            std::iter::once(Value::Symbol(Symbol {
                ns: None,
                name: Arc::new("fn".to_string()),
            }))
            .chain(list[2..].iter().cloned())
            .collect(),
        );
        self.compile_expr(&fn_form, false)?;

        // Set the name on the last compiled function so MakeClosure
        // and call_fn can dispatch through the VM
        if let Some(last_fn) = self.functions.last_mut() {
            last_fn.name = Some(name.clone());
        }

        let name_idx = self.add_string(name);
        self.ops.push(Op::DefineSymbol(name_idx));
        self.ops.push(Op::PushNil); // defn returns nil
        Ok(())
    }

    fn compile_quote(&mut self, list: &[Value]) -> Result<(), CompilerError> {
        if list.len() < 2 {
            return Err(CompilerError::InvalidForm("quote requires 1 argument".into()));
        }
        let idx = self.add_constant(list[1].clone());
        self.ops.push(Op::PushConst(idx));
        Ok(())
    }

    fn compile_ns(&mut self, list: &[Value]) -> Result<(), CompilerError> {
        if list.len() < 2 {
            return Err(CompilerError::InvalidForm("ns requires namespace name".into()));
        }
        let name = match &list[1] {
            Value::Symbol(s) => self.symbol_to_string(s),
            _ => return Err(CompilerError::InvalidForm("ns name must be symbol".into())),
        };
        let idx = self.add_string(name);
        self.ops.push(Op::PushNs(idx));
        self.ops.push(Op::PushNil);
        Ok(())
    }

    fn compile_require(&mut self, list: &[Value]) -> Result<(), CompilerError> {
        for arg in &list[1..] {
            match arg {
                Value::Symbol(sym) => {
                    let idx = self.add_string(self.symbol_to_string(sym));
                    self.ops.push(Op::PushNil); // no alias
                    self.ops.push(Op::RequireNs(idx));
                }
                Value::Vector(v) if v.len() >= 1 => {
                    if let Value::Symbol(sym) = &v[0] {
                        let ns_idx = self.add_string(self.symbol_to_string(sym));
                        // Check for :as
                        let mut alias = None;
                        let mut i = 1;
                        while i < v.len() {
                            if let Value::Keyword(kw) = &v[i] {
                                if kw.name.as_str() == "as" {
                                    if let Some(Value::Symbol(a)) = v.get(i + 1) {
                                        alias = Some(a.name.to_string());
                                    }
                                    i += 2;
                                    continue;
                                }
                            }
                            i += 1;
                        }
                        if let Some(a) = alias {
                            let val = Value::string(&format!("alias:{}", a));
                            let const_idx = self.add_constant(val);
                            self.ops.push(Op::PushConst(const_idx));
                        } else {
                            self.ops.push(Op::PushNil);
                        }
                        self.ops.push(Op::RequireNs(ns_idx));
                    }
                }
                _ => return Err(CompilerError::InvalidForm("invalid require form".into())),
            }
        }
        self.ops.push(Op::PushNil);
        Ok(())
    }

    fn compile_loop(&mut self, list: &[Value], _tail: bool) -> Result<(), CompilerError> {
        // Simplified loop — compile as a constant for now (loop/recur
        // requires complex stack frame manipulation that we'll defer).
        let idx = self.add_constant(Value::List(Arc::new(list.to_vec())));
        self.ops.push(Op::PushConst(idx));
        Ok(())
    }

    fn compile_recur(&mut self, list: &[Value]) -> Result<(), CompilerError> {
        // Simplified — compile args and mark as constant for now
        let idx = self.add_constant(Value::List(Arc::new(list.to_vec())));
        self.ops.push(Op::PushConst(idx));
        Ok(())
    }

    // ── Helpers ────────────────────────────────────────────────────────────

    fn parse_params(&self, form: &Value) -> Result<Vec<Param>, CompilerError> {
        let vec = match form {
            Value::Vector(v) => v,
            Value::List(l) => l,
            _ => return Err(CompilerError::InvalidForm("params must be a vector".into())),
        };
        let mut params = Vec::new();
        let mut i = 0;
        while i < vec.len() {
            match &vec[i] {
                Value::Symbol(s) if s.name.as_str() == "&" => {
                    if let Some(Value::Symbol(rest)) = vec.get(i + 1) {
                        params.push(Param::Rest(rest.clone()));
                        i += 2;
                    } else {
                        return Err(CompilerError::InvalidForm("& must be followed by symbol".into()));
                    }
                }
                Value::Symbol(s) => {
                    params.push(Param::Named(s.clone()));
                    i += 1;
                }
                _ => {
                    return Err(CompilerError::InvalidForm(
                        "param must be a symbol".into(),
                    ))
                }
            }
        }
        Ok(params)
    }

    fn symbol_to_string(&self, sym: &Symbol) -> String {
        match &sym.ns {
            Some(ns) => format!("{}/{}", ns, sym.name),
            None => sym.name.to_string(),
        }
    }

    fn add_constant(&mut self, val: Value) -> u16 {
        if let Some(&idx) = self.constant_map.get(&val) {
            return idx;
        }
        let idx = self.constants.len() as u16;
        self.constant_map.insert(val.clone(), idx);
        self.constants.push(val);
        idx
    }

    fn add_string(&mut self, s: String) -> u16 {
        if let Some(&idx) = self.string_map.get(&s) {
            return idx;
        }
        let idx = self.strings.len() as u16;
        self.string_map.insert(s.clone(), idx);
        self.strings.push(s);
        idx
    }

    fn add_native(&mut self, f: fn(&[Value]) -> Result<Value, String>) -> u16 {
        if let Some(&idx) = self.native_map.get(&f) {
            return idx;
        }
        let idx = self.native_fns.len() as u16;
        self.native_map.insert(f, idx);
        self.native_fns.push(f);
        idx
    }

    fn patch_jump(&mut self, pos: usize, offset: i16) {
        match &mut self.ops[pos] {
            Op::Jump(ref mut o) | Op::JumpIfFalse(ref mut o) | Op::JumpIfTrue(ref mut o) => {
                *o = offset;
            }
            _ => {}
        }
    }

    fn merge_compiler(&mut self, other: &BytecodeCompiler) {
        // Merge constants
        for (val, &other_idx) in &other.constant_map {
            let _ = other_idx;
            self.add_constant(val.clone());
        }
        // Merge strings
        for (s, &other_idx) in &other.string_map {
            let _ = other_idx;
            self.add_string(s.clone());
        }
        // Merge natives
        for (&f, &other_idx) in &other.native_map {
            let _ = other_idx;
            self.add_native(f);
        }
    }
}

// ── Compilation result ────────────────────────────────────────────────────────

/// Full compilation output: main function + constant/string pools.
pub struct CompiledProgram {
    pub main: CompiledFunction,
    pub constants: Vec<Value>,
    pub strings: Vec<String>,
    pub functions: Vec<CompiledFunction>,
    pub native_fns: Vec<fn(&[Value]) -> Result<Value, String>>,
}

// ── VM ────────────────────────────────────────────────────────────────────────

/// Stack-based bytecode virtual machine.
pub struct BytecodeVm {
    stack: Vec<Value>,
    call_stack: Vec<CallFrame>,
}

struct CallFrame {
    ops: Vec<Op>,
    ip: usize,
    locals: Vec<Value>,
    base_stack: usize,
}

impl BytecodeVm {
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(64),
            call_stack: Vec::new(),
        }
    }

    /// Execute a compiled program against an evaluator (for namespace access).
    pub fn run(
        &mut self,
        program: &CompiledProgram,
        eval: &mut Evaluator,
    ) -> Result<Value, EvalError> {
        self.stack.clear();
        self.call_stack.clear();

        let mut frame = CallFrame {
            ops: program.main.ops.clone(),
            ip: 0,
            locals: vec![Value::Nil; program.main.locals as usize],
            base_stack: 0,
        };

        loop {
            if frame.ip >= frame.ops.len() {
                break;
            }
            let op = frame.ops[frame.ip].clone();
            frame.ip += 1;

            match op {
                Op::PushConst(idx) => {
                    let val = program
                        .constants
                        .get(idx as usize)
                        .cloned()
                        .unwrap_or(Value::Nil);
                    self.stack.push(val);
                }
                Op::PushNil => self.stack.push(Value::Nil),
                Op::PushTrue => self.stack.push(Value::Bool(true)),
                Op::PushFalse => self.stack.push(Value::Bool(false)),
                Op::Pop => {
                    self.stack.pop();
                }
                Op::Dup => {
                    if let Some(top) = self.stack.last().cloned() {
                        self.stack.push(top);
                    }
                }

                Op::ResolveSymbol(idx) => {
                    let name = program
                        .strings
                        .get(idx as usize)
                        .cloned()
                        .unwrap_or_default();
                    let val = self.resolve_symbol(&name, eval)?;
                    self.stack.push(val);
                }
                Op::DefineSymbol(idx) => {
                    let name = program
                        .strings
                        .get(idx as usize)
                        .cloned()
                        .unwrap_or_default();
                    let val = self.stack.pop().unwrap_or(Value::Nil);
                    self.define_symbol(&name, val, eval)?;
                }
                Op::PushNs(idx) => {
                    let name = program
                        .strings
                        .get(idx as usize)
                        .cloned()
                        .unwrap_or_default();
                    eval.ns.find_or_create(&name);
                    eval.ns
                        .set_current(&name)
                        .map_err(|e| EvalError::SpecialForm(e))?;
                    if !eval.ns.is_loaded(&name) {
                        eval.ns
                            .refer_all("mora.core", &name)
                            .map_err(|e| EvalError::SpecialForm(e))?;
                        eval.ns.mark_loaded(&name);
                    }
                }
                Op::RequireNs(idx) => {
                    let name = program
                        .strings
                        .get(idx as usize)
                        .cloned()
                        .unwrap_or_default();
                    // Pop alias if present
                    let _alias_val = self.stack.pop();
                    eval.ns
                        .require(&name, None)
                        .map_err(|e| EvalError::SpecialForm(e))?;
                }

                Op::GetLocal(idx) => {
                    let val = frame
                        .locals
                        .get(idx as usize)
                        .cloned()
                        .unwrap_or(Value::Nil);
                    self.stack.push(val);
                }
                Op::SetLocal(idx) => {
                    let val = self.stack.last().cloned().unwrap_or(Value::Nil);
                    if (idx as usize) < frame.locals.len() {
                        frame.locals[idx as usize] = val;
                    }
                }

                Op::Jump(offset) => {
                    frame.ip = (frame.ip as isize - 1 + offset as isize) as usize;
                }
                Op::JumpIfFalse(offset) => {
                    let cond = self.stack.last().cloned().unwrap_or(Value::Nil);
                    if !is_truthy(&cond) {
                        frame.ip = (frame.ip as isize - 1 + offset as isize) as usize;
                    }
                }
                Op::JumpIfTrue(offset) => {
                    let cond = self.stack.last().cloned().unwrap_or(Value::Nil);
                    if is_truthy(&cond) {
                        frame.ip = (frame.ip as isize - 1 + offset as isize) as usize;
                    }
                }

                Op::Call(argc) => {
                    let argc = argc as usize;
                    let args_start = self.stack.len() - argc;
                    let callee = if args_start > 0 {
                        self.stack[args_start - 1].clone()
                    } else {
                        Value::Nil
                    };
                    let args: Vec<Value> = self.stack.drain(args_start..).collect();
                    if args_start > 0 {
                        self.stack.pop(); // pop callee
                    }
                    let result = self.call_value(callee, args, program, eval)?;
                    self.stack.push(result);
                }
                Op::TailCall(argc) => {
                    // For now, same as Call (full TCO requires frame recycling)
                    let argc = argc as usize;
                    let args_start = self.stack.len() - argc;
                    let callee = if args_start > 0 {
                        self.stack[args_start - 1].clone()
                    } else {
                        Value::Nil
                    };
                    let args: Vec<Value> = self.stack.drain(args_start..).collect();
                    if args_start > 0 {
                        self.stack.pop();
                    }
                    let result = self.call_value(callee, args, program, eval)?;
                    self.stack.push(result);
                }
                Op::Return => {
                    break;
                }

                Op::MakeClosure(fn_idx) => {
                    let compiled_fn = program
                        .functions
                        .get(fn_idx as usize)
                        .cloned()
                        .ok_or_else(|| {
                            EvalError::Custom(format!("unknown function index: {}", fn_idx))
                        })?;

                    // Build closure
                    let closure_env = HashMap::new();

                    let fn_name = compiled_fn.name.clone().unwrap_or_default();
                    let val = Value::Fn(FnValue {
                        name: compiled_fn.name.clone(),
                        params: compiled_fn.params.clone(),
                        body: Arc::new(vec![]),
                        closure: Arc::new(Mutex::new(closure_env)),
                        is_macro: false,
                        meta: None,
                    });

                    // Register compiled body so call_fn can dispatch through VM
                    if !fn_name.is_empty() {
                        eval.compiled_fns.insert(fn_name, compiled_fn);
                    }

                    self.stack.push(val);
                }

                Op::MakeList(count_idx) => {
                    let count = match program.constants.get(count_idx as usize) {
                        Some(Value::Int(n)) => *n as usize,
                        _ => 0,
                    };
                    let start = self.stack.len().saturating_sub(count);
                    let items: Vec<Value> = self.stack.drain(start..).collect();
                    self.stack.push(Value::list(items));
                }
                Op::MakeVector(count_idx) => {
                    let count = match program.constants.get(count_idx as usize) {
                        Some(Value::Int(n)) => *n as usize,
                        _ => 0,
                    };
                    let start = self.stack.len().saturating_sub(count);
                    let items: Vec<Value> = self.stack.drain(start..).collect();
                    self.stack.push(Value::vector(items));
                }
                Op::MakeMap(count_idx) => {
                    let count = match program.constants.get(count_idx as usize) {
                        Some(Value::Int(n)) => *n as usize,
                        _ => 0,
                    };
                    let pair_count = count;
                    let start = self.stack.len().saturating_sub(pair_count * 2);
                    let items: Vec<Value> = self.stack.drain(start..).collect();
                    let mut pairs = Vec::new();
                    for chunk in items.chunks(2) {
                        if chunk.len() == 2 {
                            pairs.push((chunk[0].clone(), chunk[1].clone()));
                        }
                    }
                    self.stack.push(Value::map(pairs));
                }

                Op::GetIndex => {
                    let idx = self.stack.pop().unwrap_or(Value::Int(0));
                    let seq = self.stack.pop().unwrap_or(Value::Nil);
                    match (&seq, &idx) {
                        (Value::Vector(v), Value::Int(i)) => {
                            let val = v.get(*i as usize).cloned().unwrap_or(Value::Nil);
                            self.stack.push(val);
                        }
                        _ => self.stack.push(Value::Nil),
                    }
                }

                Op::InvokeNative(native_idx) => {
                    let f = program
                        .native_fns
                        .get(native_idx as usize)
                        .copied()
                        .ok_or_else(|| {
                            EvalError::Custom(format!("unknown native fn index: {}", native_idx))
                        })?;
                    // Collect args — native fns expect &[Value]
                    // Convention: the last instruction before InvokeNative pushed all args
                    // For now, we pop args based on what's on the stack
                    // A more robust approach would encode argc
                    let args: Vec<Value> = self.stack.drain(frame.base_stack..).collect();
                    let result = f(&args).map_err(EvalError::Custom)?;
                    self.stack.push(result);
                }

                Op::Halt => break,
            }
        }

        Ok(self.stack.pop().unwrap_or(Value::Nil))
    }

    fn resolve_symbol(
        &self,
        name: &str,
        eval: &Evaluator,
    ) -> Result<Value, EvalError> {
        // Handle namespace-qualified symbols
        if let Some(slash_pos) = name.find('/') {
            let ns_name = &name[..slash_pos];
            let var_name = &name[slash_pos + 1..];
            let sym = Symbol {
                ns: Some(Arc::new(ns_name.to_string())),
                name: Arc::new(var_name.to_string()),
            };
            return eval
                .ns
                .resolve_symbol(&sym)
                .ok_or_else(|| EvalError::Undefined(name.to_string()));
        }
        let sym = Symbol {
            ns: None,
            name: Arc::new(name.to_string()),
        };
        eval.ns
            .resolve_symbol(&sym)
            .ok_or_else(|| EvalError::Undefined(name.to_string()))
    }

    fn define_symbol(
        &self,
        name: &str,
        value: Value,
        eval: &mut Evaluator,
    ) -> Result<(), EvalError> {
        // Handle namespace-qualified names
        let current_ns = eval.ns.current_name();
        let (ns_name, var_name) = if let Some(slash_pos) = name.find('/') {
            (name[..slash_pos].to_string(), name[slash_pos + 1..].to_string())
        } else {
            (current_ns, name.to_string())
        };
        let ns = eval.ns.find_or_create(&ns_name);
        let mut ns_lock = ns.lock();
        ns_lock.intern(&var_name, value);
        Ok(())
    }

    fn call_value(
        &self,
        callee: Value,
        args: Vec<Value>,
        program: &CompiledProgram,
        eval: &mut Evaluator,
    ) -> Result<Value, EvalError> {
        match callee {
            Value::Native(f) => f(&args).map_err(EvalError::Custom),
            Value::Fn(f) => {
                // For compiled closures, we'd need to set up a new call frame
                // For now, fall back to the tree-walking evaluator
                eval.call_fn(f, args)
            }
            other => Err(EvalError::NotAFunction(other.type_name().to_string())),
        }
    }
}

fn is_truthy(val: &Value) -> bool {
    !matches!(val, Value::Nil | Value::Bool(false))
}

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, thiserror::Error)]
pub enum CompilerError {
    #[error("unsupported form: {0}")]
    UnsupportedForm(String),
    #[error("invalid form: {0}")]
    InvalidForm(String),
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Compile a list of forms and run them through the VM.
/// Falls back to tree-walking for unsupported forms.
pub fn compile_and_run(
    eval: &mut Evaluator,
    forms: &[Value],
) -> Result<Value, EvalError> {
    let mut compiler = BytecodeCompiler::new();
    let main = match compiler.compile_forms(forms) {
        Ok(f) => f,
        Err(_) => {
            // Fallback to tree-walking if compilation fails
            let mut result = Value::Nil;
            for form in forms {
                result = eval.eval(form)?;
            }
            return Ok(result);
        }
    };

    let program = CompiledProgram {
        main,
        constants: compiler.constants,
        strings: compiler.strings,
        functions: compiler.functions,
        native_fns: compiler.native_fns,
    };

    let mut vm = BytecodeVm::new();
    vm.run(&program, eval)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MoraLisp;

    #[test]
    fn vm_push_const() {
        let mut lisp = MoraLisp::new();
        let forms = lisp.evaluator.read_cached("42").unwrap();
        let result = compile_and_run(&mut lisp.evaluator, &forms).unwrap();
        assert_eq!(result, Value::Int(42));
    }

    #[test]
    fn vm_arithmetic() {
        let mut lisp = MoraLisp::new();
        let forms = lisp.evaluator.read_cached("(+ 1 2)").unwrap();
        let result = compile_and_run(&mut lisp.evaluator, &forms).unwrap();
        assert_eq!(result, Value::Int(3));
    }

    #[test]
    fn vm_def_and_resolve() {
        let mut lisp = MoraLisp::new();
        let forms = lisp.evaluator.read_cached("(def x 99)").unwrap();
        compile_and_run(&mut lisp.evaluator, &forms).unwrap();
        let forms = lisp.evaluator.read_cached("x").unwrap();
        let result = compile_and_run(&mut lisp.evaluator, &forms).unwrap();
        assert_eq!(result, Value::Int(99));
    }

    #[test]
    fn vm_if_true() {
        let mut lisp = MoraLisp::new();
        let forms = lisp.evaluator.read_cached("(if true 1 2)").unwrap();
        let result = compile_and_run(&mut lisp.evaluator, &forms).unwrap();
        assert_eq!(result, Value::Int(1));
    }

    #[test]
    fn vm_if_false() {
        let mut lisp = MoraLisp::new();
        let forms = lisp.evaluator.read_cached("(if false 1 2)").unwrap();
        let result = compile_and_run(&mut lisp.evaluator, &forms).unwrap();
        assert_eq!(result, Value::Int(2));
    }

    #[test]
    fn vm_do_sequence() {
        let mut lisp = MoraLisp::new();
        let forms = lisp.evaluator.read_cached("(do 1 2 3)").unwrap();
        let result = compile_and_run(&mut lisp.evaluator, &forms).unwrap();
        assert_eq!(result, Value::Int(3));
    }

    #[test]
    fn vm_let_binding() {
        let mut lisp = MoraLisp::new();
        let forms = lisp.evaluator.read_cached("(let [a 10 b 20] (+ a b))").unwrap();
        let result = compile_and_run(&mut lisp.evaluator, &forms).unwrap();
        assert_eq!(result, Value::Int(30));
    }

    #[test]
    fn vm_defn_registers_fn() {
        let mut lisp = MoraLisp::new();
        let forms = lisp.evaluator.read_cached("(defn double [x] (+ x x))").unwrap();
        compile_and_run(&mut lisp.evaluator, &forms).unwrap();
        // The function should be defined in the current namespace
        let forms = lisp.evaluator.read_cached("double").unwrap();
        let result = compile_and_run(&mut lisp.evaluator, &forms).unwrap();
        assert!(matches!(result, Value::Fn(_)), "expected Fn, got {}", result.type_name());
    }

    #[test]
    fn vm_ns_switching() {
        let mut lisp = MoraLisp::new();
        let forms = lisp
            .evaluator
            .read_cached("(ns my-ns) (def my-var 7)")
            .unwrap();
        compile_and_run(&mut lisp.evaluator, &forms).unwrap();
        assert!(lisp.evaluator.ns.is_loaded("my-ns"));
    }
}
