pub mod eval;
pub mod ns;
pub mod reader;
pub mod thread;
pub mod types;
pub mod wasm;
pub mod host;

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
        let forms = reader::read_all(input)?;
        let mut result = Value::Nil;
        for form in forms {
            result = self.evaluator.eval(&form)?;
        }
        Ok(result)
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
