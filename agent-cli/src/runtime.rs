use agent::AgentStreamEvent;
use agent_core::{RunStreamEvent, UiThreadEvent};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    AssistantDelta { delta: String },
    ReasoningDelta { delta: String },
    ToolRequested { call_id: String, tool_name: String },
    ToolProgress { call_id: String, status: String },
    ToolCompleted { call_id: String },
    Warning { message: String },
    Error { message: String },
    TurnFinished { turn_id: String, failed: bool },
}

pub fn map_stream_event(event: AgentStreamEvent) -> Option<AppEvent> {
    match event {
        AgentStreamEvent::Ui(UiThreadEvent::MessageDelta { delta, .. }) => {
            Some(AppEvent::AssistantDelta { delta })
        }
        AgentStreamEvent::Ui(UiThreadEvent::ReasoningDelta { delta, .. }) => {
            Some(AppEvent::ReasoningDelta { delta })
        }
        AgentStreamEvent::Ui(UiThreadEvent::ToolCallRequested { call_id, tool_name, .. }) => {
            Some(AppEvent::ToolRequested { call_id, tool_name })
        }
        AgentStreamEvent::Ui(UiThreadEvent::ToolCallProgress { call_id, status, .. }) => {
            Some(AppEvent::ToolProgress {
                call_id,
                status: format!("{status:?}"),
            })
        }
        AgentStreamEvent::Ui(UiThreadEvent::ToolCallCompleted { result, .. }) => {
            Some(AppEvent::ToolCompleted {
                call_id: result.call_id,
            })
        }
        AgentStreamEvent::Ui(UiThreadEvent::Warning { message, .. }) => {
            Some(AppEvent::Warning { message })
        }
        AgentStreamEvent::Ui(UiThreadEvent::Error { message, .. }) => {
            Some(AppEvent::Error { message })
        }
        AgentStreamEvent::Run(RunStreamEvent::TurnDone { turn_id, .. }) => {
            Some(AppEvent::TurnFinished { turn_id, failed: false })
        }
        AgentStreamEvent::Run(RunStreamEvent::TurnFailed { turn_id, .. }) => {
            Some(AppEvent::TurnFinished { turn_id, failed: true })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_ui_and_run_events_to_app_events() {
        let ev = map_stream_event(AgentStreamEvent::Ui(UiThreadEvent::ReasoningDelta {
            turn_id: "t1".into(),
            delta: "thinking".into(),
        }));
        assert!(matches!(ev, Some(AppEvent::ReasoningDelta { .. })));

        let done = map_stream_event(AgentStreamEvent::Run(RunStreamEvent::TurnDone {
            turn_id: "t1".into(),
            epoch: 0,
            final_message: None,
            usage: agent_core::Usage::default(),
            stats: agent_core::TurnStats::default(),
        }));
        assert!(matches!(done, Some(AppEvent::TurnFinished { .. })));
    }
}
