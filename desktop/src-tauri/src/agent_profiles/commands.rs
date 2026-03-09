use serde::Serialize;
use tauri::State;

use crate::session_commands::DesktopSessionState;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentProfileSummary {
    pub id: String,
    pub label: String,
    pub description: String,
}

pub async fn list_agent_profiles_from_store(
    store: &agent::AgentProfileStore,
) -> Result<Vec<AgentProfileSummary>, String> {
    store
        .list_profiles()
        .await
        .map(|profiles| {
            profiles
                .into_iter()
                .map(|profile| AgentProfileSummary {
                    id: profile.id,
                    label: profile.display_name,
                    description: profile.description,
                })
                .collect()
        })
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn list_agent_profiles(
    state: State<'_, DesktopSessionState>,
) -> Result<Vec<AgentProfileSummary>, String> {
    list_agent_profiles_from_store(state.agent_profiles().as_ref()).await
}
