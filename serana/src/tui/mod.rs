pub mod app;
pub mod dialog;
pub mod diff;
pub mod editor;
pub mod event;
pub mod image;
pub mod markdown;
pub mod render;
pub mod slash_commands;
pub mod status_line;
pub mod symbols;
pub mod syntax;
pub mod theme;
pub mod tool_execution;
pub mod tui;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use crate::agent::SessionStore;
use crate::agent::{AgentFactory, AgentRuntimeConfig, HermesAgent};
use crate::core::Agent;
use crate::core::AgentCallbacks;
use crate::core::CancelToken;
use crate::core::Config;
use crate::core::LlmClient;
use crate::core::Message;
use crate::core::Result;
use crate::core::ToolDefinition;
use crate::llm::{CodexClient, OpenAiClient};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tokio::sync::mpsc;

use app::App;
use event::Event;
use tui::Tui;

struct ActiveLlmClient {
    config: Arc<RwLock<Config>>,
}

impl ActiveLlmClient {
    fn new(config: Arc<RwLock<Config>>) -> Self {
        Self { config }
    }

    fn client(&self) -> Box<dyn LlmClient> {
        let config = self
            .config
            .read()
            .expect("active LLM config poisoned")
            .clone();
        match config.provider.name.as_str() {
            "codex" => Box::new(
                CodexClient::new(config.model().to_string()).with_workspace(config.workspace),
            ),
            _ => Box::new(OpenAiClient::new(config)),
        }
    }
}

#[async_trait]
impl LlmClient for ActiveLlmClient {
    async fn chat(&self, messages: &[Message]) -> Result<String> {
        self.client().chat(messages).await
    }

    async fn chat_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<Message> {
        self.client().chat_with_tools(messages, tools).await
    }

    fn chat_stream<'a>(
        &'a self,
        messages: &'a [Message],
    ) -> Pin<Box<dyn Stream<Item = Result<String>> + Send + 'a>> {
        let client = self.client();
        let messages = messages.to_vec();
        Box::pin(async_stream::try_stream! {
            let mut stream = client.chat_stream(&messages);
            while let Some(chunk) = stream.next().await {
                yield chunk?;
            }
        })
    }

    fn chat_with_tools_stream<'a>(
        &'a self,
        messages: &'a [Message],
        tools: &'a [ToolDefinition],
    ) -> Pin<Box<dyn Stream<Item = Result<Message>> + Send + 'a>> {
        let client = self.client();
        let messages = messages.to_vec();
        let tools = tools.to_vec();
        Box::pin(async_stream::try_stream! {
            let mut stream = client.chat_with_tools_stream(&messages, &tools);
            while let Some(chunk) = stream.next().await {
                yield chunk?;
            }
        })
    }
}

pub fn run(workspace: PathBuf, model: String, provider: String, config: Config) -> Result<()> {
    let mut tui = Tui::new()?;
    let workspace_for_agent = workspace.clone();
    let events = event::EventHandler::new(Duration::from_millis(16));

    let (response_tx, mut response_rx) = mpsc::unbounded_channel::<AgentResponse>();
    let (stream_tx, mut stream_rx) = mpsc::unbounded_channel::<String>();
    let active_config = Arc::new(RwLock::new(config));
    let llm: Box<dyn LlmClient> = Box::new(ActiveLlmClient::new(active_config.clone()));
    // Skill system — discover skills, register creation tool
    let skill_store = crate::tools::skill::SkillStore::discover(&workspace_for_agent);

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
        cancel_token,
        active_config,
    );

    tui.restore()?;
    result
}

#[derive(Debug)]
struct AgentResponse {
    content: String,
}

fn sync_active_config(active_config: &Arc<RwLock<Config>>, app: &App) {
    let mut config = active_config.write().expect("active LLM config poisoned");
    config.provider.name = app.provider.clone();
    config.llm.model = app.model.clone();
}

fn run_app(
    tui: &mut Tui,
    app: &mut App,
    mut events: event::EventHandler,
    agent: Arc<HermesAgent>,
    response_tx: mpsc::UnboundedSender<AgentResponse>,
    response_rx: &mut mpsc::UnboundedReceiver<AgentResponse>,
    stream_rx: &mut mpsc::UnboundedReceiver<String>,
    cancel_token: crate::core::CancelToken,
    active_config: Arc<RwLock<Config>>,
) -> Result<()> {
    let mut pending_request: Option<tokio::task::JoinHandle<()>> = None;
    let mut streaming_content = String::new();

    loop {
        let (w, h) = tui.inner().size();
        let ui = render::build_ui(app, w, h);
        tui.render_ui(&ui)?;

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

            // Process next queued task if available
            if let Some(next_task) = app.task_queue.pop_front() {
                app.messages.push(app::ChatMessage {
                    role: app::MessageRole::User,
                    content: next_task.clone(),
                    tool_calls: Vec::new(),
                    thinking: None,
                });
                app.mode = app::AppMode::Processing;
            } else {
                app.mode = app::AppMode::Normal;
            }
        }

        match events.next()? {
            Event::Input(display_protocol::InputEvent::Key(key_event)) => {
                // Convert display-protocol KeyEvent to crossterm KeyEvent for app
                let crossterm_key = display_tui::conversions::key_event_to_crossterm(key_event);
                let was_processing = app.mode == app::AppMode::Processing;
                if !app.handle_key_event(crossterm_key)? || app.should_quit {
                    return Ok(());
                }
                sync_active_config(&active_config, app);

                // Cancel agent if Esc was pressed during Processing
                if was_processing && app.mode != app::AppMode::Processing {
                    cancel_token.cancel();
                    if let Some(handle) = pending_request.take() {
                        handle.abort();
                    }
                    app.clear_pending_response();
                    streaming_content.clear();
                    app.messages.push(app::ChatMessage {
                        role: app::MessageRole::System,
                        content: "Interrupted by user".to_string(),
                        tool_calls: Vec::new(),
                        thinking: None,
                    });
                }

                if app.mode == app::AppMode::Processing && pending_request.is_none() {
                    if let Some(last_msg) = app.messages.last() {
                        if last_msg.role == app::MessageRole::User {
                            let user_input = last_msg.content.clone();
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
            Event::Input(_) => {}
            Event::Tick => {
                app.tick();
            }
        }
    }
}
