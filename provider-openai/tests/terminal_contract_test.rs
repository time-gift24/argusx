use provider_openai::Mapper;

#[test]
fn no_events_after_done() {
    let mut m = Mapper::new("openai".into());
    let _ = m.on_done().unwrap();
    let err = m.feed(r#"{"id":"x","created":1,"object":"chat.completion.chunk","model":"glm-5","choices":[]}"#).unwrap_err();
    assert!(format!("{err}").contains("terminal"));
}
