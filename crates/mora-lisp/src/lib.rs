pub mod bytecode;
pub mod eval;
pub mod gc;
pub mod host;
pub mod ns;
pub mod reader;
pub mod repl;
pub mod thread;
pub mod types;
pub mod vm;

use eval::EvalError;
use reader::ReadError;
use types::Value;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("read error: {0}")]
    Read(#[from] ReadError),
    #[error("eval error: {0}")]
    Eval(#[from] EvalError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Custom(String),
}

impl Error {
    pub fn display_with_stack(&self) -> String {
        match self {
            Error::Eval(e) => e.display_with_stack(),
            other => format!("{}", other),
        }
    }
 }

pub struct MoraLisp {
    evaluator: eval::Evaluator,
}

impl MoraLisp {
    pub fn new() -> Self {
        Self {
            evaluator: eval::Evaluator::new(),
        }
    }

    pub fn eval(&mut self, input: &str) -> Result<Value, Error> {
        let forms = self.evaluator.read_cached(input)?;
        let mut result = Value::Nil;
        for form in forms {
            result = self.evaluator.eval(&form)?;
        }
        Ok(result)
    }

    /// Compile to bytecode and run through the VM.
    /// Falls back to tree-walking if compilation fails for any form.
    pub fn eval_vm(&mut self, input: &str) -> Result<Value, Error> {
        let forms = self.evaluator.read_cached(input)?;
        Ok(vm::compile_and_run(&mut self.evaluator, &forms)?)
    }

    pub fn eval_form(&mut self, form: &Value) -> Result<Value, Error> {
        Ok(self.evaluator.eval(form)?)
    }

    pub fn ns(&self) -> &ns::NamespaceRegistry {
        &self.evaluator.ns
    }

    pub fn ns_mut(&mut self) -> &mut ns::NamespaceRegistry {
        &mut self.evaluator.ns
    }
}

impl Default for MoraLisp {
    fn default() -> Self {
        Self::new()
    }
}
