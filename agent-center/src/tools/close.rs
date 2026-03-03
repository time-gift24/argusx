use crate::api::center::{CloseRequest, CloseResponse};

/// Tool for closing agent threads
pub struct CloseAgentTool;

impl CloseAgentTool {
    pub fn name() -> &'static str {
        "close_agent"
    }

    pub fn description() -> &'static str {
        "Close an agent thread and mark it as terminal"
    }

    pub async fn execute(
        center: &crate::AgentCenter,
        req: CloseRequest,
    ) -> anyhow::Result<CloseResponse> {
        center.close(req).await
    }
}
