use crate::core::{Context, Result};
use std::path::PathBuf;

pub struct ContextCompactor;

impl ContextCompactor {
    pub fn compact(&self, _context: &Context, _max_tokens: usize) -> Result<Context> {
        Ok(Context::new(PathBuf::new()))
    }
}
