//! TUI frontend for serana
//!
//! Custom inline TUI inspired by oh-my-pi, with vertical flow rendering and no
//! alternate-screen takeover.

pub mod app;
pub mod component;
pub mod components;
pub mod event;
pub mod style;
pub mod terminal;
pub mod tool_execution;
pub mod tui;
pub mod ui;
use std::sync::Arc;
use tokio::sync::mpsc;
use crate::agent::{Agent, AgentCallbacks, coding::CodingAgent, SessionStore};
use crate::llm::{LlmClient, openai::OpenAiClient};
use crate::tools::ToolRegistry;
use crate::config::Config;

use std::path::PathBuf;
use std::time::Duration;

use self::app::App;
use self::event::Event;
use self::tui::Tui;
use crate::Result;

/// Run the TUI application.
pub fn run(workspace: PathBuf, model: String, provider: String, config: Config) -> Result<()> {
    let mut tui = Tui::new()?;
    let workspace_for_agent = workspace.clone();
    let mut app = App::with_model(workspace, model, provider);
    let events = event::EventHandler::new(Duration::from_millis(16));

    let (response_tx, mut response_rx) = mpsc::unbounded_channel::<AgentResponse>();
    let (stream_tx, mut stream_rx) = mpsc::unbounded_channel::<String>();
    let llm: Box<dyn LlmClient> = Box::new(OpenAiClient::new(config));
    let tools = ToolRegistry::new();
    let session_store = SessionStore::default_location();
    session_store.init()?;
    let session = session_store.create_session()?;
    
    // Set up streaming callback
    let stream_tx_clone = stream_tx.clone();
    let callbacks = AgentCallbacks::new()
        .with_stream_delta(Arc::new(move |delta| {
            let _ = stream_tx_clone.send(delta.to_string());
        }));
    
    let agent = Arc::new(
        CodingAgent::new(llm, tools)
            .with_callbacks(callbacks)
            .with_workspace(workspace_for_agent)
            .with_session(session_store, session.meta.id),
    );

    tui.clear_screen()?;
    tui.hide_cursor()?;

    let result = run_app(&mut tui, &mut app, events, agent, response_tx, &mut response_rx, &mut stream_rx);

    tui.show_cursor()?;
    tui.restore()?;
    result
}

#[derive(Debug)]
struct AgentResponse {
    content: String,
}


fn run_app(
    tui: &mut Tui,
    app: &mut App,
    mut events: event::EventHandler,
    agent: Arc<CodingAgent>,
    response_tx: mpsc::UnboundedSender<AgentResponse>,
    response_rx: &mut mpsc::UnboundedReceiver<AgentResponse>,
    stream_rx: &mut mpsc::UnboundedReceiver<String>,
) -> Result<()> {
    let mut pending_request: Option<tokio::task::JoinHandle<()>> = None;
    let mut streaming_content = String::new();

    loop {
        ui::draw(tui, app)?;
        tui.render()?;

        // Check for streaming deltas
        while let Ok(delta) = stream_rx.try_recv() {
            streaming_content.push_str(&delta);
            app.set_pending_response(streaming_content.clone());
            tui.request_render();
        }

        // Check for agent responses
        if let Ok(resp) = response_rx.try_recv() {
            app.messages.push(app::ChatMessage {
                role: app::MessageRole::Agent,
                content: resp.content,
                tool_calls: Vec::new(),
                thinking: None,
            });
            app.mode = app::AppMode::Normal;
            app.clear_pending_response();
            streaming_content.clear();
            pending_request = None;
            tui.request_render();
        }

        match events.next()? {
            Event::Key(key_event) => {
                if !app.handle_key_event(key_event)? {
                    return Ok(());
                }

                // Check if user submitted a message
                if app.mode == app::AppMode::Processing && pending_request.is_none() {
                    if let Some(last_msg) = app.messages.last() {
                        if last_msg.role == app::MessageRole::User {
                            let user_input = last_msg.content.clone();
                            let agent_clone = agent.clone();
                            let tx = response_tx.clone();

                            pending_request = Some(tokio::spawn(async move {
                                let result = agent_clone.execute(&user_input).await;
                                let content = match result {
                                    Ok(output) => output.response,
                                    Err(e) => format!("Error: {}", e),
                                };
                                let _ = tx.send(AgentResponse { content });
                            }));
                        }
                    }
                }

                tui.request_render();
            }
            Event::Resize(width, height) => {
                app.handle_resize(width, height);
                tui.request_render();
            }
            Event::Tick => {
                app.tick();
                tui.request_render();
            }
        }
    }
}
