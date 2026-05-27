use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serana_core::{
    AgentCallbacks, AgentStatus, Message, MetaCognition, ModificationKind, ModificationRecord,
    ToolCall, ToolCallData, ToolApproval,
};
use crate::tools::ToolRegistry;

use super::execute_tools_concurrent;

pub struct ToolTurnOutput {
    pub messages: Vec<Message>,
    pub tool_calls: Vec<ToolCall>,
}

pub async fn handle_tool_turn(
    tool_calls: &[ToolCallData],
    tools: &ToolRegistry,
    callbacks: &AgentCallbacks,
    meta_cognition: &Arc<MetaCognition>,
    approval: Option<&ToolApproval>,
) -> ToolTurnOutput {
    let mut enriched_tool_calls = Vec::with_capacity(tool_calls.len());
    for tool_call in tool_calls {
        let recent_failures = meta_cognition
            .get_recent_failures(&tool_call.function.name, 3)
            .await;
        if !recent_failures.is_empty() {
            let warning = format!(
                "[Meta] {} has failed {} times recently. Last args: {}",
                tool_call.function.name,
                recent_failures.len(),
                recent_failures[0].description
            );
            callbacks.fire_status(AgentStatus::Thinking);
            callbacks.fire_stream_delta(&warning);
        }
        enriched_tool_calls.push(tool_call.clone());
    }

    callbacks.fire_status(AgentStatus::ExecutingTool);
    let results = execute_tools_concurrent(&enriched_tool_calls, tools, callbacks).await;
    callbacks.fire_status(AgentStatus::Running);

    let mut messages = Vec::with_capacity(results.len());
    let mut completed_tool_calls = Vec::with_capacity(results.len());

    for result in results {
        let tool_call = result.to_tool_call();
        let result_str = result.result_string();
        messages.push(Message::tool_result(result.id, result_str));

        let success = result.result.is_ok();
        let description = format!(
            "Tool call: {} with args: {}",
            tool_call.name, tool_call.arguments
        );
        let timestamp = current_unix_timestamp();
        let record = ModificationRecord {
            timestamp: timestamp.clone(),
            file: format!("tool:{}", tool_call.name),
            kind: ModificationKind::ToolCall,
            description,
            tests_passed: success,
            commit: None,
            lessons: vec![],
        };
        let _ = meta_cognition.record(record).await;

        if !success {
            let error_msg = match &result.result {
                Ok(_) => String::new(),
                Err(err) => err.to_string(),
            };
            let lesson = format!(
                "Failed to call {} with args {}: {}",
                tool_call.name, tool_call.arguments, error_msg
            );
            let _ = meta_cognition.add_lesson(&timestamp, lesson).await;
        }

        completed_tool_calls.push(tool_call);
    }

    ToolTurnOutput {
        messages,
        tool_calls: completed_tool_calls,
    }
}

fn current_unix_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
