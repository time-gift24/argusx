use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThreadState {
    Idle,
    Processing,
    BackgroundProcessing,
    WaitingForPermission,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TurnRecordState {
    Completed,
    Failed,
    Interrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCallRecord {
    pub call_id: String,
    pub tool_name: String,
    pub arguments: String,
    pub result: Option<String>,
    pub is_error: bool,
}

/// Response from assistant in a turn
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssistantResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCallRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TurnRecord {
    pub turn_number: usize,
    pub user_input: String,
    pub assistant_response: Option<AssistantResponse>,
    pub tool_calls: Vec<ToolCallRecord>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub state: TurnRecordState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_record_serialization() {
        let record = TurnRecord {
            turn_number: 1,
            user_input: "Hello".to_string(),
            assistant_response: None,
            tool_calls: vec![],
            started_at: Utc::now(),
            completed_at: None,
            state: TurnRecordState::Completed,
        };

        let json = serde_json::to_string(&record).unwrap();
        let deserialized: TurnRecord = serde_json::from_str(&json).unwrap();

        assert_eq!(record.turn_number, deserialized.turn_number);
        assert_eq!(record.user_input, deserialized.user_input);
    }

    #[test]
    fn turn_record_with_tool_calls() {
        let record = TurnRecord {
            turn_number: 1,
            user_input: "Search".to_string(),
            assistant_response: Some(AssistantResponse {
                text: "Found 3 results".to_string(),
                tool_calls: vec![ToolCallRecord {
                    call_id: "call-1".to_string(),
                    tool_name: "web_search".to_string(),
                    arguments: r#"{"query": "rust"}"#.to_string(),
                    result: Some("[...]".to_string()),
                    is_error: false,
                }],
            }),
            tool_calls: vec![ToolCallRecord {
                call_id: "call-1".to_string(),
                tool_name: "web_search".to_string(),
                arguments: r#"{"query": "rust"}"#.to_string(),
                result: Some("[...]".to_string()),
                is_error: false,
            }],
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            state: TurnRecordState::Completed,
        };

        let json = serde_json::to_string(&record).unwrap();
        let deserialized: TurnRecord = serde_json::from_str(&json).unwrap();

        assert_eq!(record.tool_calls.len(), deserialized.tool_calls.len());
        assert_eq!(record.tool_calls[0].tool_name, deserialized.tool_calls[0].tool_name);
    }
}
