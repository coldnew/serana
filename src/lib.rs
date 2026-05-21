pub mod agent;
pub mod config;
pub mod context;
pub mod llm;
pub mod lsp;
pub mod tools;
pub mod tree_sitter;
pub mod tui;
pub mod web;

pub type Result<T> = std::result::Result<T, anyhow::Error>;
