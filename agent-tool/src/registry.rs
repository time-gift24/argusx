use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::context::{ToolContext, ToolResult};
use crate::error::ToolError;
use crate::spec::ToolSpec;
use crate::trait_def::Tool;

pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Arc<dyn Tool>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, tool: impl Tool + 'static) {
        let tool = Arc::new(tool);
        self.tools.write().await.insert(tool.name().to_string(), tool);
    }

    pub async fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.read().await.get(name).cloned()
    }

    pub async fn list(&self) -> Vec<ToolSpec> {
        let tools = self.tools.read().await;
        tools.values().map(|t| t.spec()).collect()
    }

    pub async fn call(
        &self,
        name: &str,
        args: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let tool = self.get(name).await.ok_or_else(|| ToolError::NotFound(name.to_string()))?;
        tool.execute(ctx, args).await
    }
}
