use turn::TurnError;

use super::storage::{ConversationRecord, StoredConversationMessage};

pub trait ConversationCompressionPolicy: Send + Sync {
    fn compress(&self, record: ConversationRecord) -> Result<ConversationRecord, TurnError>;
}

pub struct NoopConversationCompressionPolicy;

impl ConversationCompressionPolicy for NoopConversationCompressionPolicy {
    fn compress(&self, record: ConversationRecord) -> Result<ConversationRecord, TurnError> {
        Ok(record)
    }
}

pub struct ThresholdConversationCompressionPolicy {
    max_messages: usize,
    retain_recent_messages: usize,
}

impl ThresholdConversationCompressionPolicy {
    pub fn new(max_messages: usize, retain_recent_messages: usize) -> Self {
        Self {
            max_messages,
            retain_recent_messages,
        }
    }
}

impl ConversationCompressionPolicy for ThresholdConversationCompressionPolicy {
    fn compress(&self, mut record: ConversationRecord) -> Result<ConversationRecord, TurnError> {
        if record.history.len() <= self.max_messages {
            return Ok(record);
        }

        let retain_recent = self.retain_recent_messages.min(record.history.len());
        let compressed_count = record.history.len().saturating_sub(retain_recent);
        let mut history = Vec::with_capacity(retain_recent.saturating_add(1));
        history.push(StoredConversationMessage::SystemNote {
            content: format!(
                "Compressed {compressed_count} earlier messages into a summary checkpoint."
            ),
        });
        history.extend(record.history.into_iter().skip(compressed_count));
        record.history = history;

        Ok(record)
    }
}
