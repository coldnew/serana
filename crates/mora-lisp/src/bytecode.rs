//! Bytecode types shared between the compiler/VM and the evaluator.
//!
//! This module exists to avoid circular dependencies: `eval.rs` needs to
//! look up compiled functions during `call_fn`, and `vm.rs` needs to
//! import `EvalError`. Both can safely depend on these types.

use crate::types::{Param, Symbol, Value};

/// Bytecode instruction for the stack-based VM.
///
/// Operates on a value stack. Constants and strings are stored in
/// side tables (pools) indexed by u16.
#[derive(Debug, Clone)]
pub enum Op {
    PushConst(u16),
    PushNil,
    PushTrue,
    PushFalse,
    Pop,
    Dup,

    ResolveSymbol(u16),
    DefineSymbol(u16),
    PushNs(u16),
    RequireNs(u16),

    GetLocal(u16),
    SetLocal(u16),

    Jump(i16),
    JumpIfFalse(i16),
    JumpIfTrue(i16),

    Call(u8),
    TailCall(u8),
    Return,

    MakeClosure(u16),

    MakeList(u16),
    MakeVector(u16),
    MakeMap(u16),

    GetIndex,

    InvokeNative(u16),

    Halt,
}

/// A compiled bytecode function ready for VM execution.
///
/// Each function carries its own constant/string pools so it can
/// be dispatched from the tree-walking evaluator without needing
/// a top-level `CompiledProgram`.
#[derive(Clone)]
pub struct CompiledFunction {
    pub name: Option<String>,
    pub params: Vec<Param>,
    pub locals: u16,
    pub ops: Vec<Op>,
    pub constants: Vec<Value>,
    pub strings: Vec<String>,
    pub native_fns: Vec<fn(&[Value]) -> Result<Value, String>>,
}
