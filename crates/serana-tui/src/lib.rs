pub mod app;
pub mod dialog;
pub mod diff;
pub mod editor;
pub mod event;
pub mod image;
pub mod markdown;
pub mod slash_commands;
pub mod status_line;
pub mod symbols;
pub mod syntax;
pub mod theme;
pub mod tool_execution;
pub mod tui;
pub mod ui;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serana_agent::SessionStore;
use serana_agent::{AgentFactory, AgentRuntimeConfig, HermesAgent};
use serana_core::Agent;
use serana_core::AgentCallbacks;
use serana_core::CancelToken;
use serana_core::Config;
use serana_core::LlmClient;
use serana_core::Result;
use serana_llm::OpenAiClient;
use tokio::sync::mpsc;

use crate::app::App;
use crate::event::Event;
use crate::tui::Tui;

pub fn run(workspace: PathBuf, model: String, provider: String, config: Config) -> Result<()> {
    let mut tui = Tui::new()?;
    let workspace_for_agent = workspace.clone();
    let events = event::EventHandler::new(Duration::from_millis(16));

    let (response_tx, mut response_rx) = mpsc::unbounded_channel::<AgentResponse>();
    let (stream_tx, mut stream_rx) = mpsc::unbounded_channel::<String>();
    let llm: Box<dyn LlmClient> = Box::new(OpenAiClient::new(config));
    // Skill system — discover skills, register creation tool
    let skill_store = serana_tools::skill::SkillStore::discover(&workspace_for_agent);

    // Session persistence
    let session_store = SessionStore::default_location();
    session_store.init()?;
    let session = session_store.create_session()?;

    // Wire session store into App for /session commands
    let mut app = App::with_model(workspace, model, provider);
    app.session_store = Some(session_store.clone());
    app.current_session_id = Some(session.meta.id.clone());
    app.load_recent_sessions();
    // Extract skill descriptions before moving store into App
    let skill_prompts: Vec<String> = skill_store
        .all()
        .into_iter()
        .map(|s| format!("[{}] {}", s.name, s.description))
        .collect();
    app.skill_store = Some(skill_store);

    let stream_tx_clone = stream_tx.clone();
    let callbacks = AgentCallbacks::new().with_stream_delta(Arc::new(move |delta| {
        let _ = stream_tx_clone.send(delta.to_string());
    }));
    let cancel_token = CancelToken::new();
    let agent_cancel_token = cancel_token.clone();

    let runtime_config = AgentRuntimeConfig::hermes(workspace_for_agent).with_skills(skill_prompts);
    let factory = AgentFactory::hermes(runtime_config);
    let agent = Arc::new(
        factory
            .build(llm)
            .with_callbacks(callbacks)
            .with_session(session_store, session.meta.id)
            .with_cancel_token(agent_cancel_token),
    );

    let result = run_app(
        &mut tui,
        &mut app,
        events,
        agent,
        response_tx,
        &mut response_rx,
        &mut stream_rx,
    );

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
    agent: Arc<HermesAgent>,
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
                content: resp.content.clone(),
                tool_calls: Vec::new(),
                thinking: None,
            });
            app.mode = app::AppMode::Normal;
            app.clear_pending_response();
            streaming_content.clear();
            pending_request = None;

            // Persist assistant message to session store
            if let (Some(ref store), Some(ref sid)) = (&app.session_store, &app.current_session_id)
            {
                let _ = store.save_message(sid, "assistant", &resp.content);
            }
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
                if !app.handle_key_event(key_event)? || app.should_quit {
                    return Ok(());
                }

                if app.mode == app::AppMode::Processing && pending_request.is_none() {
                    if let Some(last_msg) = app.messages.last() {
                        if last_msg.role == app::MessageRole::User {
                            let user_input = last_msg.content.clone();
                            // Persist user message to session store
                            if let (Some(ref store), Some(ref sid)) =
                                (&app.session_store, &app.current_session_id)
                            {
                                let _ = store.save_message(sid, "user", &user_input);
                            }
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
