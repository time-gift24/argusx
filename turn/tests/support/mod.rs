use argus_core::{FinishReason, ResponseEvent, Usage};
use tool::ToolResult;
use turn::{AuthorizationDecision, PermissionRequest};

mod fake_authorizer;
mod fake_model;
mod fake_observer;
mod fake_tool_runner;

pub use fake_authorizer::FakeAuthorizer;
pub use fake_model::FakeModelRunner;
pub use fake_observer::FakeObserver;
pub use fake_tool_runner::FakeToolRunner;

#[allow(dead_code)]
pub fn text_only_model(chunks: impl IntoIterator<Item = &'static str>) -> FakeModelRunner {
    let mut events: Vec<ResponseEvent> = chunks
        .into_iter()
        .map(|chunk| ResponseEvent::ContentDelta(chunk.into()))
        .collect();
    events.push(ResponseEvent::Done {
        reason: FinishReason::Stop,
        usage: Some(Usage::zero()),
    });
    FakeModelRunner::new(vec![events])
}

#[allow(dead_code)]
pub fn multi_step_model(steps: Vec<Vec<ResponseEvent>>) -> FakeModelRunner {
    FakeModelRunner::new(steps)
}

#[allow(dead_code)]
pub fn delayed_tool_runner(
    plans: impl IntoIterator<Item = (&'static str, u64, ToolResult)>,
) -> FakeToolRunner {
    FakeToolRunner::new(
        plans
            .into_iter()
            .map(|(call_id, delay_ms, result)| (call_id.to_string(), delay_ms, result)),
    )
}

#[allow(dead_code)]
pub fn permission_authorizer(
    decisions: impl IntoIterator<Item = (&'static str, AuthorizationDecision)>,
) -> FakeAuthorizer {
    FakeAuthorizer::new(
        decisions
            .into_iter()
            .map(|(call_id, decision)| (call_id.to_string(), decision)),
    )
}

#[allow(dead_code)]
pub fn ask_permission(call_id: &'static str, request_id: &'static str) -> AuthorizationDecision {
    AuthorizationDecision::Ask(PermissionRequest {
        request_id: request_id.to_string(),
        tool_call_id: call_id.to_string(),
    })
}

#[allow(dead_code)]
pub fn instant_tool_runner() -> FakeToolRunner {
    FakeToolRunner::default()
}
