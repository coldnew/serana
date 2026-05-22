pub mod app;
pub mod diff;
pub mod event;
pub mod markdown;
pub mod symbols;
pub mod syntax;
pub mod theme;
pub mod tool_execution;
pub mod tui;
pub mod ui;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serana_agent::CodingAgent;
use serana_agent::SessionStore;
use serana_core::Agent;
use serana_core::AgentCallbacks;
use serana_core::CancelToken;
use serana_core::Config;
use serana_core::LlmClient;
use serana_core::Result;
use serana_llm::OpenAiClient;
use serana_tools::self_evolve;
use serana_tools::ToolRegistry;
use tokio::sync::mpsc;

use crate::app::App;
use crate::event::Event;
use crate::tui::Tui;

pub fn run(workspace: PathBuf, model: String, provider: String, config: Config) -> Result<()> {
    let mut tui = Tui::new()?;
    let workspace_for_agent = workspace.clone();
    let mut app = App::with_model(workspace, model, provider);
    let events = event::EventHandler::new(Duration::from_millis(16));

    let (response_tx, mut response_rx) = mpsc::unbounded_channel::<AgentResponse>();
    let (stream_tx, mut stream_rx) = mpsc::unbounded_channel::<String>();
    let llm: Box<dyn LlmClient> = Box::new(OpenAiClient::new(config));
    let mut tools = ToolRegistry::new();
    self_evolve::register_self_evolve_tools(&mut tools);
    let session_store = SessionStore::default_location();
    session_store.init()?;
    let session = session_store.create_session()?;

    let stream_tx_clone = stream_tx.clone();
    let callbacks = AgentCallbacks::new().with_stream_delta(Arc::new(move |delta| {
        let _ = stream_tx_clone.send(delta.to_string());
    }));
    let cancel_token = CancelToken::new();
    let agent_cancel_token = cancel_token.clone();

    let agent = Arc::new(
        CodingAgent::new(llm, tools)
            .with_callbacks(callbacks)
            .with_workspace(workspace_for_agent)
            .with_session(session_store, session.meta.id)
            .with_cancel_token(agent_cancel_token),
    );

    let result = run_app(&mut tui, &mut app, events, agent, response_tx, &mut response_rx, &mut stream_rx);

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
        tui.terminal().draw(|frame| {
            ui::draw(frame, app);
        })?;

        while let Ok(delta) = stream_rx.try_recv() {
            streaming_content.push_str(&delta);
            app.set_pending_response(streaming_content.clone());
        }

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

            if let Some(reminder) = app.todo_reminder_message() {
                app.messages.push(app::ChatMessage {
                    role: app::MessageRole::System,
                    content: reminder,
                    tool_calls: Vec::new(),
                    thinking: None,
                });
            }
        }

        match events.next()? {
            Event::Key(key_event) => {
                if !app.handle_key_event(key_event)? {
                    return Ok(());
                }

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
            }
            Event::Resize(_width, _height) => {}
            Event::Tick => {
                app.tick();
            }
        }
    }
}
