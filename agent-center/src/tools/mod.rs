pub mod close;
pub mod spawn;
pub mod wait;

use std::sync::Arc;

pub use close::CloseAgentTool;
pub use spawn::SpawnAgentTool;
pub use wait::WaitTool;

use crate::AgentCenter;

/// Register all agent-center tools with a tool runtime
pub async fn register_tools(center: Arc<AgentCenter>, runtime: &agent_tool::AgentToolRuntime) {
    runtime
        .register_tool(SpawnAgentTool::new(center.clone()))
        .await;
    runtime.register_tool(WaitTool::new(center.clone())).await;
    runtime.register_tool(CloseAgentTool::new(center)).await;
}
