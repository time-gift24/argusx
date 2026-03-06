use core::{FinishReason, ResponseEvent, Usage};

#[test]
fn done_event_preserves_finish_reason() {
    let event = ResponseEvent::Done {
        reason: FinishReason::ToolCalls,
        usage: Some(Usage::zero()),
    };

    match event {
        ResponseEvent::Done { reason, .. } => assert_eq!(reason, FinishReason::ToolCalls),
        _ => panic!("expected done event"),
    }
}
