use std::path::PathBuf;

pub struct Context {
    pub workspace_root: PathBuf,
    pub relevant_files: Vec<PathBuf>,
    pub conversation_history: Vec<String>,
}

impl Context {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            relevant_files: Vec::new(),
            conversation_history: Vec::new(),
        }
    }

    pub fn add_message(&mut self, message: String) {
        self.conversation_history.push(message);
    }
}
