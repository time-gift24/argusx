use argus_core::ToolCall;

// Eq is valid: every field in every variant implements Eq.
// ToolCall (in AssistantToolCalls) derives Eq in argus_core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnMessage {
    User {
        content: String,
    },
    AssistantText {
        content: String,
    },
    AssistantToolCalls {
        content: Option<String>,
        calls: Vec<ToolCall>,
    },
    ToolResult {
        call_id: String,
        tool_name: String,
        content: String,
        is_error: bool,
    },
    SystemNote {
        content: String,
    },
}

#[derive(Debug, Clone, Default)]
pub struct TurnTranscript {
    messages: Vec<TurnMessage>,
}

impl TurnTranscript {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, message: TurnMessage) {
        self.messages.push(message);
    }

    pub fn messages(&self) -> &[TurnMessage] {
        &self.messages
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
        assert!(matches!(&t.messages()[0], TurnMessage::User { content } if content == "hello"));
    }
}
