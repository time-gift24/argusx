use std::sync::Arc;

use argus_core::ToolCall;

pub type SharedToolCall = Arc<ToolCall>;
pub type SharedToolCalls = Arc<[SharedToolCall]>;
pub type SharedTurnMessage = Arc<TurnMessage>;
pub type TurnMessageSnapshot = Arc<[SharedTurnMessage]>;

// Eq is valid: every field in every variant implements Eq.
// ToolCall (in AssistantToolCalls) derives Eq in argus_core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnMessage {
    User {
        content: Arc<str>,
    },
    AssistantText {
        content: Arc<str>,
    },
    AssistantToolCalls {
        content: Option<Arc<str>>,
        calls: SharedToolCalls,
    },
    ToolResult {
        call_id: Arc<str>,
        tool_name: Arc<str>,
        content: Arc<str>,
        is_error: bool,
    },
    SystemNote {
        content: Arc<str>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct TurnTranscript {
    messages: Vec<SharedTurnMessage>,
}

impl TurnTranscript {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, message: TurnMessage) {
        self.messages.push(Arc::new(message));
    }

    pub fn messages(&self) -> &[SharedTurnMessage] {
        &self.messages
    }

    pub fn snapshot(&self) -> TurnMessageSnapshot {
        Arc::from(self.messages.clone())
    }

    pub fn to_vec(&self) -> Vec<TurnMessage> {
        self.messages
            .iter()
            .map(|message| message.as_ref().clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_transcript_is_empty() {
        let t = TurnTranscript::new();
        assert!(t.messages().is_empty());
    }

    #[test]
    fn push_and_retrieve_messages() {
        let mut t = TurnTranscript::new();
        t.push(TurnMessage::User {
            content: "hello".into(),
        });
        t.push(TurnMessage::AssistantText {
            content: "hi".into(),
        });
        assert_eq!(t.messages().len(), 2);
        assert!(matches!(
            t.messages()[0].as_ref(),
            TurnMessage::User { content } if content.as_ref() == "hello"
        ));
    }
}
