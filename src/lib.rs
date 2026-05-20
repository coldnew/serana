pub mod agent;
pub mod tools;
pub mod llm;
pub mod context;
pub mod config;
pub mod tui;
pub mod web;

pub type Result<T> = std::result::Result<T, anyhow::Error>;
