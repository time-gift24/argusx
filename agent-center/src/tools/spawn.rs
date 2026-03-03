use crate::api::center::{SpawnRequest, SpawnResponse};

/// Tool for spawning child agents
pub struct SpawnAgentTool;

impl SpawnAgentTool {
    pub fn name() -> &'static str {
        "spawn_agent"
    }

    pub fn description() -> &'static str {
        "Spawn a child agent with a given agent type and initial input"
    }

    pub async fn execute(
        center: &crate::AgentCenter,
        req: SpawnRequest,
    ) -> anyhow::Result<SpawnResponse> {
        center.spawn(req).await
    }
}
