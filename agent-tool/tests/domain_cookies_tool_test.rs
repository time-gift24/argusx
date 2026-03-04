use agent_core::tools::ToolCatalog;
use agent_tool::{AgentToolRuntime, DomainCookiesTool, Tool, ToolContext, ToolRegistry};
use axum::{extract::Json, routing::post, Router};
use serde_json::{json, Value};

fn test_context() -> ToolContext {
    ToolContext {
        session_id: "test-session".to_string(),
        turn_id: "test-turn".to_string(),
    }
}

#[tokio::test]
async fn default_runtime_registers_domain_cookie_tool() {
    let runtime = AgentToolRuntime::default_with_builtins().await;
    let tools = runtime.list_tools().await;
    assert!(tools.iter().any(|tool| tool.name == "get_domain_cookies"));
}

#[tokio::test]
async fn domain_cookie_tool_calls_gateway_and_returns_payload() {
    let app = Router::new().route(
        "/api/cookies/fetch",
        post(|Json(payload): Json<Value>| async move {
            assert_eq!(payload["domain"], "github.com");
            assert_eq!(payload["refresh_after_ms"], 120000);
            Json(json!({
                "domain": "github.com",
                "source": "refresh",
                "age_ms": 0,
                "fetched_at_unix_ms": 123,
                "count": 1,
                "cookies": [{"name": "_gh_sess", "value": "x"}]
            }))
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let registry = ToolRegistry::new();
    registry
        .register(DomainCookiesTool::new(format!("http://{addr}")))
        .await;

    let result = registry
        .call(
            "get_domain_cookies",
            json!({"domain":"github.com","refresh_after_ms":120000}),
            test_context(),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    assert_eq!(result.output["domain"], "github.com");
    assert_eq!(result.output["source"], "refresh");
    assert_eq!(result.output["count"], 1);

    server.abort();
}

#[tokio::test]
async fn domain_cookie_tool_rejects_missing_domain() {
    let tool = DomainCookiesTool::new("http://127.0.0.1:3456");
    let err = tool
        .execute(test_context(), json!({"refresh_after_ms": 1000}))
        .await
        .expect_err("domain should be required");

    assert!(err.to_string().contains("domain"));
}
