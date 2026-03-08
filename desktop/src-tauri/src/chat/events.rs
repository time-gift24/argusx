use serde::{Deserialize, Serialize};

use crate::chat::plan::DesktopPlanSnapshot;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TurnTargetKind {
    #[serde(rename = "agent")]
    Agent,
    #[serde(rename = "workflow")]
    Workflow,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartTurnInput {
    pub prompt: String,
    pub target_kind: TurnTargetKind,
    pub target_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartTurnResult {
    pub turn_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopTurnEvent {
    pub turn_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HydratedChatTurnStatus {
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HydratedToolCallStatus {
    Running,
    Success,
    Failed,
    TimedOut,
    Denied,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HydratedToolCall {
    pub call_id: String,
    pub name: String,
    pub arguments_json: String,
    pub output_summary: Option<String>,
    pub error_summary: Option<String>,
    pub status: HydratedToolCallStatus,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HydratedChatTurn {
    pub turn_id: String,
    pub prompt: String,
    pub assistant_text: String,
    pub reasoning_text: String,
    pub status: HydratedChatTurnStatus,
    pub error: Option<String>,
    pub latest_plan: Option<DesktopPlanSnapshot>,
    pub tool_calls: Vec<HydratedToolCall>,
}
