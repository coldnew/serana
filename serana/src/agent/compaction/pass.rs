use crate::core::{CompactionAction, CompactionConfig, Message};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PassLevel {
    Reclaim = 0,
    Shrink = 1,
    Microcompact = 2,
    Collapse = 3,
    Evict = 4,
}

pub struct PassContext<'a> {
    pub config: &'a CompactionConfig,
    pub pressure: super::pressure::Pressure,
}

pub struct PassResult {
    pub messages: Vec<Message>,
    pub actions: Vec<CompactionAction>,
}

pub trait Pass: Send + Sync {
    fn level(&self) -> PassLevel;
    fn should_run(&self, ctx: &PassContext<'_>) -> bool;
    fn run(&self, messages: Vec<Message>, ctx: &PassContext<'_>) -> PassResult;
}
