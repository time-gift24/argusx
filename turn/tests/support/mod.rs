use argus_core::{FinishReason, ResponseEvent, Usage};

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
    FakeModelRunner::new(events)
}
