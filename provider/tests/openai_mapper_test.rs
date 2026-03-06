use argus_core::ResponseEvent;
use provider::{Dialect, Mapper};

#[test]
fn openai_chunk_maps_content_delta() {
    let mut m = Mapper::new(Dialect::Openai);
    let events = m
        .feed(r#"{"id":"x","object":"chat.completion.chunk","created":1,"model":"glm-5","choices":[{"index":0,"delta":{"content":"hi"}}]}"#)
        .unwrap();

    assert!(
        events
            .iter()
            .any(|e| matches!(e, ResponseEvent::ContentDelta(_)))
    );
}
