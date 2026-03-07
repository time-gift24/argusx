use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use argus_core::{Builtin, BuiltinToolCall, McpCall, McpCallType, ToolCall};
use async_trait::async_trait;
use serde_json::json;
use telemetry::{RecordingSink, TelemetryConfig, TelemetryLayer};
use tokio_util::sync::CancellationToken;
use tool::{
    Tool, ToolContext, ToolError, ToolResult, ToolSpec,
    mcp::{McpClient, McpStdioConfig},
    scheduler::{BuiltinRegistration, EffectiveToolPolicy, McpRegistration, ToolScheduler},
};
use tracing_subscriber::{Registry, layer::SubscriberExt};

#[derive(Debug)]
struct EchoBuiltin;

#[async_trait]
impl Tool for EchoBuiltin {
    fn name(&self) -> &str {
        "read"
    }

    fn description(&self) -> &str {
        "Echo builtin"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "read".into(),
            description: "Echo builtin".into(),
            input_schema: json!({ "type": "object" }),
        }
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        _args: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::ok(json!({ "ok": true })))
    }
}

fn test_context() -> ToolContext {
    ToolContext::new("session-1", "turn-1", CancellationToken::new())
}

#[tokio::test(flavor = "current_thread")]
async fn scheduler_emits_builtin_tool_events_with_turn_context() {
    let sink = RecordingSink::default();
    let subscriber = Registry::default().with(TelemetryLayer::new(
        sink.clone(),
        TelemetryConfig::default(),
    ));

    let _guard = tracing::subscriber::set_default(subscriber);
    let scheduler = ToolScheduler::new([BuiltinRegistration::new(
        Builtin::Read,
        Arc::new(EchoBuiltin),
        EffectiveToolPolicy {
            allow_parallel: true,
            max_concurrency: 1,
        },
    )])
    .unwrap();

    let _ = scheduler
        .execute_builtin(
            BuiltinToolCall {
                sequence: 41,
                call_id: "call-1".into(),
                builtin: Builtin::Read,
                arguments_json: "{}".into(),
            },
            test_context(),
        )
        .await
        .unwrap();

    let records = sink.take();
    assert!(records.iter().any(|record| {
        record.event_name == "tool_started"
            && record.tool_name.as_deref() == Some("read")
            && record.session_id == "session-1"
            && record.turn_id == "turn-1"
            && record.sequence_no == 41
    }));
    assert!(records.iter().any(|record| {
        record.event_name == "tool_completed"
            && record.tool_name.as_deref() == Some("read")
            && record.session_id == "session-1"
            && record.turn_id == "turn-1"
            && record.sequence_no == 41
    }));
}

#[tokio::test(flavor = "current_thread")]
async fn scheduler_emits_mcp_tool_events_with_turn_context() {
    let sink = RecordingSink::default();
    let subscriber = Registry::default().with(TelemetryLayer::new(
        sink.clone(),
        TelemetryConfig::default(),
    ));

    let _guard = tracing::subscriber::set_default(subscriber);
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

    let scheduler = ToolScheduler::from_parts(
        std::iter::empty(),
        [McpRegistration::new(
            "mock".into(),
            Arc::new(client),
            EffectiveToolPolicy {
                allow_parallel: true,
                max_concurrency: 1,
            },
        )],
    )
    .unwrap();

    let _ = scheduler
        .execute(
            ToolCall::Mcp(McpCall {
                sequence: 7,
                id: "call-mcp".into(),
                mcp_type: McpCallType::McpCall,
                server_label: Some("mock".into()),
                name: Some("echo".into()),
                arguments_json: Some(r#"{"text":"hi"}"#.into()),
                output_json: None,
                tools_json: None,
                error: None,
            }),
            test_context(),
        )
        .await
        .unwrap();

    let records = sink.take();
    assert!(records.iter().any(|record| {
        record.event_name == "tool_started"
            && record.tool_name.as_deref() == Some("echo")
            && record.session_id == "session-1"
            && record.turn_id == "turn-1"
            && record.sequence_no == 7
    }));
    assert!(records.iter().any(|record| {
        record.event_name == "tool_completed"
            && record.tool_name.as_deref() == Some("echo")
            && record.session_id == "session-1"
            && record.turn_id == "turn-1"
            && record.sequence_no == 7
    }));
}
