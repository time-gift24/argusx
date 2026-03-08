use std::sync::Arc;

use argus_core::{Builtin, BuiltinToolCall, McpCall, McpCallType, ToolCall};
use turn::TurnMessage;
use uuid::Uuid;

use crate::types::{PersistedMessage, PersistedToolCall, PersistedToolKind, TurnRecord, TurnStatus};

#[derive(Debug)]
pub struct ThreadRuntime {
    pub thread_id: Uuid,
    pub active_turn: Option<ActiveTurnRuntime>,
}

impl ThreadRuntime {
    pub fn new(thread_id: Uuid) -> Self {
        Self {
            thread_id,
            active_turn: None,
        }
    }

    pub fn build_prior_messages(&self, turns: &[TurnRecord]) -> Vec<TurnMessage> {
        turns
            .iter()
            .filter(|turn| is_replayable_status(&turn.status))
            .flat_map(|turn| turn.transcript.iter().cloned().map(persisted_message_to_turn_message))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveTurnRuntime {
    pub turn_id: Uuid,
    pub turn_number: u32,
}

fn is_replayable_status(status: &TurnStatus) -> bool {
    matches!(
        status,
        TurnStatus::Completed | TurnStatus::Cancelled | TurnStatus::Failed
    )
}

fn persisted_message_to_turn_message(message: PersistedMessage) -> TurnMessage {
    match message {
        PersistedMessage::User { content } => TurnMessage::User {
            content: content.into(),
        },
        PersistedMessage::AssistantText { content } => TurnMessage::AssistantText {
            content: content.into(),
        },
        PersistedMessage::AssistantToolCalls { content, calls } => {
            let calls: Vec<_> = calls
                .into_iter()
                .map(persisted_tool_call_to_turn_call)
                .map(Arc::new)
                .collect();
            TurnMessage::AssistantToolCalls {
                content: content.map(Into::into),
                calls: Arc::from(calls),
            }
        }
        PersistedMessage::ToolResult {
            call_id,
            tool_name,
            content,
            is_error,
        } => TurnMessage::ToolResult {
            call_id: call_id.into(),
            tool_name: tool_name.into(),
            content: content.into(),
            is_error,
        },
        PersistedMessage::SystemNote { content } => TurnMessage::SystemNote {
            content: content.into(),
        },
    }
}

fn persisted_tool_call_to_turn_call(call: PersistedToolCall) -> ToolCall {
    match call.kind {
        PersistedToolKind::Function => ToolCall::FunctionCall {
            sequence: call.sequence,
            call_id: call.call_id,
            name: call.tool_name,
            arguments_json: call.arguments,
        },
        PersistedToolKind::Builtin => ToolCall::Builtin(BuiltinToolCall {
            sequence: call.sequence,
            call_id: call.call_id,
            builtin: Builtin::from_name(&call.tool_name)
                .unwrap_or_else(|| Builtin::Unknown(call.tool_name.clone())),
            arguments_json: call.arguments,
        }),
        PersistedToolKind::McpCall => ToolCall::Mcp(McpCall {
            sequence: call.sequence,
            id: call.call_id,
            mcp_type: McpCallType::McpCall,
            server_label: call.server_label,
            name: Some(call.tool_name),
            arguments_json: Some(call.arguments),
            output_json: None,
            tools_json: None,
            error: None,
        }),
        PersistedToolKind::McpListTools => ToolCall::Mcp(McpCall {
            sequence: call.sequence,
            id: call.call_id,
            mcp_type: McpCallType::McpListTools,
            server_label: call.server_label,
            name: Some(call.tool_name),
            arguments_json: (!call.arguments.is_empty()).then_some(call.arguments),
            output_json: None,
            tools_json: None,
            error: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::types::{PersistedMessage, TurnRecord};

    #[test]
    fn thread_runtime_flattens_completed_turn_history_in_order() {
        let thread_id = Uuid::new_v4();
        let runtime = ThreadRuntime::new(thread_id);

        let turns = vec![
            TurnRecord {
                id: Uuid::new_v4(),
                thread_id,
                turn_number: 1,
                user_input: "hello".into(),
                status: TurnStatus::Completed,
                finish_reason: Some("Completed".into()),
                transcript: vec![
                    PersistedMessage::User {
                        content: "hello".into(),
                    },
                    PersistedMessage::AssistantText {
                        content: "hi".into(),
                    },
                ],
                final_output: Some("hi".into()),
                started_at: Utc::now(),
                finished_at: Some(Utc::now()),
            },
            TurnRecord {
                id: Uuid::new_v4(),
                thread_id,
                turn_number: 2,
                user_input: "next".into(),
                status: TurnStatus::Interrupted,
                finish_reason: None,
                transcript: vec![PersistedMessage::User {
                    content: "partial".into(),
                }],
                final_output: None,
                started_at: Utc::now(),
                finished_at: None,
            },
        ];

        let prior = runtime.build_prior_messages(&turns);
        assert_eq!(prior.len(), 2);
        assert!(matches!(prior[0], TurnMessage::User { ref content } if content.as_ref() == "hello"));
        assert!(matches!(prior[1], TurnMessage::AssistantText { ref content } if content.as_ref() == "hi"));
    }

    #[test]
    fn thread_runtime_replays_persisted_builtin_tool_calls() {
        let thread_id = Uuid::new_v4();
        let runtime = ThreadRuntime::new(thread_id);
        let turns = vec![TurnRecord {
            id: Uuid::new_v4(),
            thread_id,
            turn_number: 1,
            user_input: "read".into(),
            status: TurnStatus::Completed,
            finish_reason: Some("Completed".into()),
            transcript: vec![PersistedMessage::AssistantToolCalls {
                content: None,
                calls: vec![PersistedToolCall {
                    sequence: 0,
                    call_id: "call-1".into(),
                    tool_name: "read".into(),
                    arguments: "{}".into(),
                    kind: PersistedToolKind::Builtin,
                    server_label: None,
                }],
            }],
            final_output: None,
            started_at: Utc::now(),
            finished_at: Some(Utc::now()),
        }];

        let prior = runtime.build_prior_messages(&turns);
        assert_eq!(prior.len(), 1);
        assert!(matches!(
            &prior[0],
            TurnMessage::AssistantToolCalls { calls, .. }
                if calls.len() == 1
                    && matches!(
                        calls[0].as_ref(),
                        ToolCall::Builtin(call)
                            if call.call_id == "call-1"
                                && call.sequence == 0
                                && matches!(call.builtin, Builtin::Read)
                    )
        ));
    }
}
