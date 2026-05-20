use std::path::PathBuf;

use crate::Result;

pub struct ContextGatherer;

impl ContextGatherer {
    pub async fn gather_relevant_files(
        &self,
        _workspace: &PathBuf,
        _instruction: &str,
    ) -> Result<Vec<PathBuf>> {
        // TODO: Implement file relevance detection
        Ok(Vec::new())
    }
}
