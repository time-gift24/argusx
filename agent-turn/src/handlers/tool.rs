use crate::command::DomainCommand;
use crate::domain::DomainEvent;
use crate::handlers::CommandHandler;
use crate::state::TurnState;

pub struct ToolHandler;

impl CommandHandler for ToolHandler {
    fn handle(&self, cmd: &DomainCommand, state: &TurnState) -> Vec<DomainEvent> {
        match cmd {
            DomainCommand::ToolResultOk { epoch, result, .. } => {
                if state.inflight_tools.contains_key(&result.call_id) {
                    vec![DomainEvent::ToolFinished {
                        epoch: *epoch,
                        call_id: result.call_id.clone(),
                        is_error: false,
                    }]
                } else {
                    Vec::new()
                }
            }
            DomainCommand::ToolResultErr { epoch, result, .. } => {
                if state.inflight_tools.contains_key(&result.call_id) {
                    vec![DomainEvent::ToolFinished {
                        epoch: *epoch,
                        call_id: result.call_id.clone(),
                        is_error: true,
                    }]
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use agent_core::{SessionMeta, ToolCall, ToolResult};

    use crate::command::DomainCommand;
    use crate::domain::DomainEvent;
    use crate::handlers::HandlerRegistry;
    use crate::state::TurnState;

    fn state_with_inflight(call_id: &str) -> TurnState {
        let mut state = TurnState::new(SessionMeta::new("s1", "t1"), "p", "m");
        state.inflight_tools.insert(
            call_id.to_string(),
            ToolCall {
                call_id: call_id.to_string(),
                tool_name: "echo".to_string(),
                arguments: serde_json::json!({}),
            },
        );
        state
    }

    #[test]
    fn tool_result_ok_command_emits_tool_finished_event() {
        let reg = HandlerRegistry::default();
        let cmd = DomainCommand::ToolResultOk {
            id: "c1".into(),
            epoch: 0,
            result: ToolResult::ok("call-1", serde_json::json!({"ok": true})),
        };
        let out = reg.handle(cmd, &state_with_inflight("call-1"));
        assert!(out
            .iter()
            .any(|e| matches!(e, DomainEvent::ToolFinished { is_error: false, .. })));
    }
}

