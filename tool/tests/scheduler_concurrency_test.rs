use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

use argus_core::{Builtin, BuiltinToolCall};
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use tool::{scheduler::{BuiltinRegistration, EffectiveToolPolicy, ToolScheduler}, Tool, ToolContext, ToolError, ToolResult, ToolSpec};

#[derive(Debug)]
struct SlowTool {
    current: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
}

#[async_trait]
impl Tool for SlowTool {
    fn name(&self) -> &str {
        "read"
    }

    fn description(&self) -> &str {
        "Slow test tool"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_schema: serde_json::json!({ "type": "object" }),
        }
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        _args: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let now = self.current.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(now, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(50)).await;
        self.current.fetch_sub(1, Ordering::SeqCst);
        Ok(ToolResult::ok(serde_json::json!({"ok": true})))
    }
}

fn test_context() -> ToolContext {
    ToolContext::new("s1", "t1", CancellationToken::new())
}

fn read_call(call_id: &str) -> BuiltinToolCall {
    BuiltinToolCall {
        sequence: 0,
        call_id: call_id.to_string(),
        builtin: Builtin::Read,
        arguments_json: "{}".to_string(),
    }
}

#[tokio::test]
async fn serial_builtin_never_runs_more_than_one_at_a_time() {
    let current = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));

    let scheduler = ToolScheduler::new([BuiltinRegistration::new(
        Builtin::Read,
        Arc::new(SlowTool {
            current: current.clone(),
            peak: peak.clone(),
        }),
        EffectiveToolPolicy {
            allow_parallel: false,
            max_concurrency: 4,
        },
    )])
    .expect("scheduler should build");

    let first = scheduler.execute_builtin(read_call("call-1"), test_context());
    let second = scheduler.execute_builtin(read_call("call-2"), test_context());

    let (_a, _b) = tokio::join!(first, second);
    assert_eq!(peak.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn parallel_builtin_respects_max_concurrency() {
    let current = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));

    let scheduler = ToolScheduler::new([BuiltinRegistration::new(
        Builtin::Read,
        Arc::new(SlowTool {
            current: current.clone(),
            peak: peak.clone(),
        }),
        EffectiveToolPolicy {
            allow_parallel: true,
            max_concurrency: 2,
        },
    )])
    .expect("scheduler should build");

    let first = scheduler.execute_builtin(read_call("call-1"), test_context());
    let second = scheduler.execute_builtin(read_call("call-2"), test_context());
    let third = scheduler.execute_builtin(read_call("call-3"), test_context());

    let (_a, _b, _c) = tokio::join!(first, second, third);
    assert_eq!(peak.load(Ordering::SeqCst), 2);
}
