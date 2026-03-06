use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use argus_core::{Builtin, BuiltinToolCall, McpCall, McpCallType, ToolCall};
use async_trait::async_trait;
use serde_json::json;
use tokio_util::sync::CancellationToken;
use tool::{
    Tool, ToolContext, ToolError, ToolResult, ToolSpec,
    config::AgentToolConfig,
    mcp::{McpClient, McpStdioConfig},
    scheduler::{BuiltinRegistration, McpRegistration, ToolScheduler},
};

#[derive(Debug)]
struct DummyBuiltin;

#[async_trait]
impl Tool for DummyBuiltin {
    fn name(&self) -> &str {
        "read"
    }

    fn description(&self) -> &str {
        "Dummy builtin"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "read".into(),
            description: "Dummy builtin".into(),
            input_schema: json!({ "type": "object" }),
        }
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        _args: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::ok(json!({ "source": "builtin" })))
    }
}

fn test_context() -> ToolContext {
    ToolContext::new("session-1", "turn-1", CancellationToken::new())
}

#[tokio::test]
async fn scheduler_routes_builtin_and_mcp_calls_to_different_executors() {
    let raw = r#"
        [tools]
        builtin_tools = ["read"]

        [tools.defaults]
        allow_parallel = true
        max_concurrency = 4

        [mcp.defaults]
        allow_parallel = true
        max_concurrency = 4

        [mcp.server.mock]
        enabled = true
        transport = "stdio"
        command = "cargo"
        args = ["run", "-q", "-p", "tool", "--bin", "mock_mcp_server"]
        cwd = "."
        max_concurrency = 2
    "#;

    let cfg = AgentToolConfig::parse_and_validate(raw).unwrap();
    let server = cfg.mcp.server.get("mock").unwrap();
    let client = McpClient::connect_stdio(McpStdioConfig {
        server_label: "mock".into(),
        command: server.command.clone().unwrap(),
        args: server.args.clone(),
        cwd: Some(PathBuf::from(server.cwd.clone().unwrap())),
        env: BTreeMap::new(),
    })
    .await
    .unwrap();

    let scheduler = ToolScheduler::from_parts(
        [BuiltinRegistration::new(
            Builtin::Read,
            Arc::new(DummyBuiltin),
            cfg.effective_builtin_policy(Builtin::Read).unwrap(),
        )],
        [McpRegistration::new(
            "mock".into(),
            Arc::new(client),
            cfg.effective_mcp_policy("mock").unwrap(),
        )],
    )
    .unwrap();

    let builtin = ToolCall::Builtin(BuiltinToolCall {
        sequence: 0,
        call_id: "call_builtin".into(),
        builtin: Builtin::Read,
        arguments_json: "{}".into(),
    });
    let mcp = ToolCall::Mcp(McpCall {
        sequence: 1,
        id: "call_mcp".into(),
        mcp_type: McpCallType::McpCall,
        server_label: Some("mock".into()),
        name: Some("echo".into()),
        arguments_json: Some(r#"{"text":"hi"}"#.into()),
        output_json: None,
        tools_json: None,
        error: None,
    });

    let builtin_out = scheduler.execute(builtin, test_context()).await.unwrap();
    assert_eq!(builtin_out.output["source"], json!("builtin"));

    let mcp_out = scheduler.execute(mcp, test_context()).await.unwrap();
    assert_eq!(mcp_out.output["structuredContent"], json!({ "text": "hi" }));
}
