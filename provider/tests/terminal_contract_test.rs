use provider::{Dialect, Mapper};

#[test]
fn no_events_after_done() {
    let mut m = Mapper::new(Dialect::Openai);
    let done = m.on_done().unwrap();
    assert!(
        done.iter()
            .any(|e| matches!(e, argus_core::ResponseEvent::Done(_)))
    );

    assert!(m.on_done().is_err());
    assert!(m
        .feed(
            r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[{"index":0,"delta":{"content":"late"}}]}"#,
        )
        .is_err());
}
