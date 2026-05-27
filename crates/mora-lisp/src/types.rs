use parking_lot::Mutex;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::eval::EvalError;
use crate::ns::Namespace;

pub mod builtin;

/// Unique ID for concurrency primitives
static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum TypeError {
    #[error("expected {expected}, got {got}")]
    Mismatch { expected: String, got: String },
    #[error("invalid arity: expected {expected}, got {got}")]
    Arity { expected: usize, got: usize },
    #[error("{0}")]
    Custom(String),
}

/// The core value type for the Lisp VM
#[derive(Clone)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Arc<String>),
    Char(char),
    Keyword(Keyword),
    Symbol(Symbol),
    List(Arc<Vec<Value>>),
    Vector(Arc<Vec<Value>>),
    Map(Arc<HashMap<Value, Value>>),
    Set(Arc<Vec<Value>>),
    Fn(FnValue),
    Native(fn(&[Value]) -> Result<Value, String>),
    Macro(MacroValue),
    Atom(Arc<Mutex<Value>>),
    Agent(AgentValue),
    Future(FutureValue),
    Ref(RefValue),
    Namespace(Arc<Mutex<Namespace>>),
    Promise(PromiseValue),
    Thread(ThreadValue),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Keyword {
    pub ns: Option<Arc<String>>,
    pub name: Arc<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Symbol {
    pub ns: Option<Arc<String>>,
    pub name: Arc<String>,
}

#[derive(Clone)]
pub struct FnValue {
    pub name: Option<String>,
    pub params: Vec<Param>,
    pub body: Arc<Vec<Value>>,
    pub closure: Arc<Mutex<HashMap<Symbol, Value>>>,
    pub is_macro: bool,
    pub meta: Option<Arc<Vec<(Value, Value)>>>,
}

#[derive(Debug, Clone)]
pub enum Param {
    /// Normal parameter
    Named(Symbol),
    /// & rest parameter
    Rest(Symbol),
    /// Destructuring map parameter
    DestructMap(Vec<(Keyword, Param)>),
    /// Destructuring vector parameter
    DestructVec(Vec<Param>),
}

#[derive(Clone)]
pub struct MacroValue {
    pub name: Option<String>,
    pub params: Vec<Param>,
    pub body: Arc<Vec<Value>>,
    pub closure: Arc<Mutex<HashMap<Symbol, Value>>>,
}

#[derive(Clone)]
pub struct AgentValue {
    pub id: u64,
    pub state: Arc<Mutex<Value>>,
    pub queue: Arc<Mutex<Vec<Value>>>,
    pub error: Arc<Mutex<Option<Value>>>,
    pub error_handler: Arc<Mutex<Option<Value>>>,
    pub awaiters: Arc<Mutex<Vec<Arc<AtomicBool>>>>,
}

#[derive(Clone)]
pub struct FutureValue {
    pub id: u64,
    pub result: Arc<Mutex<Option<Result<Value, EvalError>>>>,
    pub done: Arc<AtomicBool>,
}

#[derive(Clone)]
pub struct RefValue {
    pub id: u64,
    pub state: Arc<Mutex<Value>>,
    pub history: Arc<Mutex<Vec<Value>>>,
    pub validators: Arc<Mutex<Vec<Value>>>,
}

#[derive(Clone)]
pub struct PromiseValue {
    pub id: u64,
    pub result: Arc<Mutex<Option<Value>>>,
    pub done: Arc<AtomicBool>,
    pub waiters: Arc<Mutex<Vec<Arc<AtomicBool>>>>,
}

#[derive(Clone)]
pub struct ThreadValue {
    pub id: u64,
    pub handle: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,
    pub done: Arc<AtomicBool>,
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nil => "nil",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Char(_) => "char",
            Value::Keyword(_) => "keyword",
            Value::Symbol(_) => "symbol",
            Value::List(_) => "list",
            Value::Vector(_) => "vector",
            Value::Map(_) => "map",
            Value::Set(_) => "set",
            Value::Fn(f) if f.is_macro => "macro",
            Value::Fn(_) => "fn",
            Value::Native(_) => "native-fn",
            Value::Macro(_) => "macro",
            Value::Atom(_) => "atom",
            Value::Agent(_) => "agent",
            Value::Future(_) => "future",
            Value::Ref(_) => "ref",
            Value::Namespace(_) => "namespace",
            Value::Promise(_) => "promise",
            Value::Thread(_) => "thread",
        }
    }

    pub fn is_truthy(&self) -> bool {
        !matches!(self, Value::Nil | Value::Bool(false))
    }

    pub fn is_seq(&self) -> bool {
        matches!(self, Value::List(_) | Value::Vector(_))
    }

    pub fn is_collection(&self) -> bool {
        matches!(
            self,
            Value::List(_) | Value::Vector(_) | Value::Map(_) | Value::Set(_)
        )
    }

    pub fn seq_to_vec(&self) -> Option<&Vec<Value>> {
        match self {
            Value::List(v) | Value::Vector(v) => Some(v),
            _ => None,
        }
    }

    pub fn nil() -> Self {
        Value::Nil
    }

    pub fn string(s: impl Into<String>) -> Self {
        Value::String(Arc::new(s.into()))
    }

    pub fn symbol(name: impl Into<String>) -> Self {
        let s = name.into();
        let (ns, name) = if let Some(pos) = s.find('/') {
            if pos == 0 || pos == s.len() - 1 {
                // "/" alone or starts/ends with "/" - treat as regular symbol
                (None, Arc::new(s))
            } else {
                (Some(Arc::new(s[..pos].to_string())), Arc::new(s[pos + 1..].to_string()))
            }
        } else {
            (None, Arc::new(s))
        };
        Value::Symbol(Symbol { ns, name })
    }

    pub fn keyword(name: impl Into<String>) -> Self {
        let s = name.into();
        let s = s.strip_prefix(':').unwrap_or(&s);
        let (ns, name) = if let Some(pos) = s.find('/') {
            if pos == 0 || pos == s.len() - 1 {
                (None, Arc::new(s.to_string()))
            } else {
                (Some(Arc::new(s[..pos].to_string())), Arc::new(s[pos + 1..].to_string()))
            }
        } else {
            (None, Arc::new(s.to_string()))
        };
        Value::Keyword(Keyword { ns, name })
    }

    pub fn list(values: Vec<Value>) -> Self {
        Value::List(Arc::new(values))
    }

    pub fn vector(values: Vec<Value>) -> Self {
        Value::Vector(Arc::new(values))
    }

    pub fn map(pairs: Vec<(Value, Value)>) -> Self {
        Value::Map(Arc::new(pairs.into_iter().collect()))
    }

    pub fn set(values: Vec<Value>) -> Self {
        Value::Set(Arc::new(values))
    }

    pub fn atom(val: Value) -> Self {
        Value::Atom(Arc::new(Mutex::new(val)))
    }

    pub fn agent(val: Value) -> Self {
        Value::Agent(AgentValue {
            id: next_id(),
            state: Arc::new(Mutex::new(val)),
            queue: Arc::new(Mutex::new(Vec::new())),
            error: Arc::new(Mutex::new(None)),
            error_handler: Arc::new(Mutex::new(None)),
            awaiters: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub fn future() -> Self {
        Value::Future(FutureValue {
            id: next_id(),
            result: Arc::new(Mutex::new(None)),
            done: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn ref_value(val: Value) -> Self {
        Value::Ref(RefValue {
            id: next_id(),
            state: Arc::new(Mutex::new(val)),
            history: Arc::new(Mutex::new(Vec::new())),
            validators: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub fn promise() -> Self {
        Value::Promise(PromiseValue {
            id: next_id(),
            result: Arc::new(Mutex::new(None)),
            done: Arc::new(AtomicBool::new(false)),
            waiters: Arc::new(Mutex::new(Vec::new())),
        })
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Char(c) => write!(f, "\\{}", c),
            Value::Keyword(k) => {
                if let Some(ns) = &k.ns {
                    write!(f, ":{}:{}", ns, k.name)
                } else {
                    write!(f, ":{}", k.name)
                }
            }
            Value::Symbol(s) => {
                if let Some(ns) = &s.ns {
                    write!(f, "{}/{}", ns, s.name)
                } else {
                    write!(f, "{}", s.name)
                }
            }
            Value::List(v) => {
                write!(f, "(")?;
                for (i, val) in v.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{:?}", val)?;
                }
                write!(f, ")")
            }
            Value::Vector(v) => {
                write!(f, "[")?;
                for (i, val) in v.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{:?}", val)?;
                }
                write!(f, "]")
            }
            Value::Map(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?} {:?}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Set(s) => {
                write!(f, "#{{")?;
                for (i, val) in s.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{:?}", val)?;
                }
                write!(f, "}}")
            }
            Value::Fn(func) => {
                if let Some(name) = &func.name {
                    write!(f, "#<fn:{}>", name)
                } else {
                    write!(f, "#<fn>")
                }
            }
            Value::Macro(m) => {
                if let Some(name) = &m.name {
                    write!(f, "#<macro:{}>", name)
                } else {
                    write!(f, "#<macro>")
                }
            }
            Value::Native(_) => write!(f, "#<native-fn>"),
            Value::Atom(_) => write!(f, "#<atom>"),
            Value::Agent(a) => write!(f, "#<agent:{}>", a.id),
            Value::Future(_) => write!(f, "#<future>"),
            Value::Ref(r) => write!(f, "#<ref:{}>", r.id),
            Value::Namespace(ns) => {
                let ns = ns.lock();
                write!(f, "#<namespace:{}>", ns.name)
            }
            Value::Promise(_) => write!(f, "#<promise>"),
            Value::Thread(_) => write!(f, "#<thread>"),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            _ => fmt::Debug::fmt(self, f),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Nil, Value::Nil) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Keyword(a), Value::Keyword(b)) => a == b,
            (Value::Symbol(a), Value::Symbol(b)) => a == b,
            (Value::List(a), Value::List(b)) => a == b,
            (Value::Vector(a), Value::Vector(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            (Value::Set(a), Value::Set(b)) => a == b,
            (Value::Atom(a), Value::Atom(b)) => Arc::ptr_eq(a, b),
            (Value::Agent(a), Value::Agent(b)) => a.id == b.id,
            (Value::Future(a), Value::Future(b)) => a.id == b.id,
            (Value::Ref(a), Value::Ref(b)) => a.id == b.id,
            (Value::Fn(a), Value::Fn(b)) => {
                a.name == b.name && Arc::ptr_eq(&a.body, &b.body)
            }
            _ => false,
        }
    }
}

impl Eq for Value {}

impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Value::Nil => 0u8.hash(state),
            Value::Bool(b) => b.hash(state),
            Value::Int(n) => n.hash(state),
            Value::Float(f) => f.to_bits().hash(state),
            Value::String(s) => s.hash(state),
            Value::Char(c) => c.hash(state),
            Value::Keyword(k) => k.hash(state),
            Value::Symbol(s) => s.hash(state),
            Value::List(v) => v.hash(state),
            Value::Vector(v) => v.hash(state),
            Value::Map(m) => {
                // HashMap doesn't preserve order, so XOR pair hashes
                let mut combined: u64 = 0;
                for (k, v) in m.iter() {
                    let mut h = DefaultHasher::new();
                    k.hash(&mut h);
                    v.hash(&mut h);
                    combined ^= h.finish();
                }
                combined.hash(state);
            }
            Value::Atom(a) => Arc::as_ptr(a).hash(state),
            Value::Agent(a) => a.id.hash(state),
            Value::Future(a) => a.id.hash(state),
            Value::Ref(a) => a.id.hash(state),
            _ => std::ptr::hash(self, state),
        }
    }
}
