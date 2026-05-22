use std::path::PathBuf;

use serana_core::Result;

pub struct ContextGatherer;

impl ContextGatherer {
    pub async fn gather_relevant_files(
        &self,
        _workspace: &PathBuf,
        _instruction: &str,
    ) -> Result<Vec<PathBuf>> {
        Ok(Vec::new())
    }
}
