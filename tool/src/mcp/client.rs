use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::sync::OnceCell;

use super::process::{spawn_stdio_session, McpSession};

#[derive(Debug, Clone)]
pub struct McpStdioConfig {
    pub server_label: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpToolDescriptor {
    pub name: String,
    pub description: Option<String>,
    #[serde(default, rename = "inputSchema")]
    pub input_schema: Value,
}

#[derive(Debug, Error)]
pub enum McpError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("MCP protocol error: {0}")]
    Protocol(String),
    #[error("MCP server returned error {code}: {message}")]
    Server { code: i64, message: String },
    #[error("MCP process exited for server `{0}`")]
    ProcessExited(String),
}

#[derive(Debug)]
pub struct McpClient {
    inner: Arc<ClientInner>,
}

#[derive(Debug)]
struct ClientInner {
    config: McpStdioConfig,
    session: OnceCell<McpSession>,
}

impl McpClient {
    pub async fn connect_stdio(config: McpStdioConfig) -> Result<Self, McpError> {
        let client = Self {
            inner: Arc::new(ClientInner {
                config,
                session: OnceCell::new(),
            }),
        };

        client.session().await?;
        Ok(client)
    }

    pub async fn list_tools(&self) -> Result<Vec<McpToolDescriptor>, McpError> {
        #[derive(Deserialize)]
        struct ToolsListResult {
            tools: Vec<McpToolDescriptor>,
        }

        let result = self
            .session()
            .await?
            .request("tools/list", serde_json::json!({}))
            .await?;
        let tools = serde_json::from_value::<ToolsListResult>(result)?;
        Ok(tools.tools)
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments_json: &str,
    ) -> Result<serde_json::Value, McpError> {
        let arguments: Value = serde_json::from_str(arguments_json)?;
        self.session()
            .await?
            .request(
                "tools/call",
                serde_json::json!({
                    "name": name,
                    "arguments": arguments,
                }),
            )
            .await
    }

    async fn session(&self) -> Result<&McpSession, McpError> {
        self.inner
            .session
            .get_or_try_init(|| async { spawn_stdio_session(&self.inner.config).await })
            .await
    }
}
