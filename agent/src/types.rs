use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentProfileKind {
    BuiltinMain,
    CustomSubagent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentProfileRecord {
    pub id: String,
    pub kind: AgentProfileKind,
    pub display_name: String,
    pub description: String,
    pub system_prompt: String,
    pub tool_policy_json: serde_json::Value,
    pub model_config_json: serde_json::Value,
    pub allow_subagent_dispatch: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AgentProfileRecord {
    pub fn custom(
        id: impl Into<String>,
        display_name: impl Into<String>,
        description: impl Into<String>,
        system_prompt: impl Into<String>,
        tool_policy_json: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            kind: AgentProfileKind::CustomSubagent,
            display_name: display_name.into(),
            description: description.into(),
            system_prompt: system_prompt.into(),
            tool_policy_json,
            model_config_json: serde_json::Value::Null,
            allow_subagent_dispatch: false,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }
}
