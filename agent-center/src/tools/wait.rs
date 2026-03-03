use crate::api::center::{WaitRequest, WaitResponse};

/// Tool for waiting on agent threads
pub struct WaitTool;

impl WaitTool {
    pub fn name() -> &'static str {
        "wait"
    }

    pub fn description() -> &'static str {
        "Wait for agent threads to reach terminal state"
    }

    pub async fn execute(
        center: &crate::AgentCenter,
        req: WaitRequest,
    ) -> anyhow::Result<WaitResponse> {
        center.wait(req).await
    }
}
