pub mod error;
pub mod events;
pub mod model;
pub mod runtime_event;
pub mod session;
pub mod tools;
pub mod traits;
pub mod transcript_item;

pub use error::{AgentError, RuntimeError, TransientError};
pub use events::{RunStreamEvent, ToolCallStatus, TurnStats, UiThreadEvent};
pub use model::{
    new_id, Id, InputEnvelope, InputPart, InputSource, ModelOutputEvent, ModelRequest, SessionMeta,
    ToolCall, ToolResult, TurnRequest, Usage,
};
pub use runtime_event::RuntimeEvent;
pub use session::{
    SessionId, SessionInfo, SessionStatus, TurnContext, TurnId, TurnStatus, TurnSummary,
};
pub use traits::{
    CheckpointStore, LanguageModel, ModelEventStream, RunEventStream, Runtime, RuntimeStreams,
    UiEventStream,
};
pub use transcript_item::{NoteLevel, TranscriptItem};

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use futures::stream;

    #[test]
    fn runtime_event_roundtrip_json() {
        let ev = RuntimeEvent::InputInjected {
            event_id: new_id(),
            input: InputEnvelope::user_text("hello"),
        };
        let raw = serde_json::to_string(&ev).expect("serialize runtime event");
        let got: RuntimeEvent = serde_json::from_str(&raw).expect("deserialize runtime event");
        assert_eq!(ev, got);
    }

    #[test]
    fn runtime_event_roundtrip_post_validator_json() {
        let ev = RuntimeEvent::PostValidatorSuccess {
            event_id: new_id(),
            summary: Some("clean summary".to_string()),
        };
        let raw = serde_json::to_string(&ev).expect("serialize runtime event");
        let got: RuntimeEvent = serde_json::from_str(&raw).expect("deserialize runtime event");
        assert_eq!(ev, got);
    }

    #[test]
    fn transcript_item_roundtrip_json() {
        let item = TranscriptItem::assistant_message("done");
        let raw = serde_json::to_string(&item).expect("serialize transcript item");
        let got: TranscriptItem = serde_json::from_str(&raw).expect("deserialize transcript item");
        assert_eq!(item, got);
    }

    #[test]
    fn run_stream_event_post_validation_started_roundtrip() {
        use crate::events::RunStreamEvent;
        let ev = RunStreamEvent::PostValidationStarted {
            turn_id: new_id(),
        };
        let raw = serde_json::to_string(&ev).expect("serialize run stream event");
        let got: RunStreamEvent = serde_json::from_str(&raw).expect("deserialize run stream event");
        assert_eq!(ev, got);
    }

    #[test]
    fn ui_thread_event_post_validation_started_roundtrip() {
        let ev = UiThreadEvent::PostValidationStarted {
            turn_id: new_id(),
        };
        let raw = serde_json::to_string(&ev).expect("serialize ui thread event");
        let got: UiThreadEvent = serde_json::from_str(&raw).expect("deserialize ui thread event");
        assert_eq!(ev, got);
    }

    struct DummyRuntime;
    struct DummyModel;

    #[async_trait]
    impl Runtime for DummyRuntime {
        async fn run_turn(&self, _request: TurnRequest) -> Result<RuntimeStreams, AgentError> {
            Ok(RuntimeStreams {
                run: Box::pin(stream::empty()),
                ui: Box::pin(stream::empty()),
            })
        }

        async fn inject_input(
            &self,
            _turn_id: &str,
            _input: InputEnvelope,
        ) -> Result<(), AgentError> {
            Ok(())
        }

        async fn cancel_turn(
            &self,
            _turn_id: &str,
            _reason: Option<String>,
        ) -> Result<(), AgentError> {
            Ok(())
        }
    }

    #[async_trait]
    impl LanguageModel for DummyModel {
        fn model_name(&self) -> &str {
            "dummy"
        }

        async fn stream(&self, _request: ModelRequest) -> Result<ModelEventStream, AgentError> {
            Ok(Box::pin(stream::empty()))
        }
    }

    #[test]
    fn trait_compatibility_compile_check() {
        fn assert_runtime<T: Runtime>() {}
        fn assert_model<T: LanguageModel>() {}
        assert_runtime::<DummyRuntime>();
        assert_model::<DummyModel>();
    }
}
