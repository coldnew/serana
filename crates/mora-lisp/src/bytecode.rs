//! Bytecode types shared between the compiler/VM and the evaluator.

use crate::types::{Param, Value};

/// Bytecode instruction for the stack-based VM.
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

/// A compiled bytecode function.
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
