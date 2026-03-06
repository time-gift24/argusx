use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::json;
use tool::mcp::{McpClient, McpStdioConfig};

#[tokio::test]
async fn stdio_mcp_client_lists_and_calls_tools() {
    let client = McpClient::connect_stdio(McpStdioConfig {
        server_label: "mock".into(),
        command: std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()),
        args: vec![
            "run".into(),
            "-q".into(),
            "-p".into(),
            "tool".into(),
            "--bin".into(),
            "mock_mcp_server".into(),
        ],
        cwd: Some(PathBuf::from(env!("CARGO_MANIFEST_DIR"))),
        env: BTreeMap::new(),
    })
    .await
    .unwrap();

    let tools = client.list_tools().await.unwrap();
    assert!(tools.iter().any(|tool| tool.name == "echo"));

    let output = client.call_tool("echo", r#"{"text":"hi"}"#).await.unwrap();
    assert_eq!(output["structuredContent"], json!({ "text": "hi" }));
}
