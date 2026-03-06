use argus_core::ResponseEvent;
use provider_openai::Mapper;

const FIXTURE: &str = include_str!("fixtures/2026-03-06-openai-chat-completions-sse.txt");

#[test]
fn replay_fixture_emits_done_with_usage() {
    let lines = FIXTURE;
    let mut mapper = Mapper::new("openai".into());

    let mut all = Vec::new();
    for line in lines.lines().filter(|l| l.starts_with("data: ")) {
        let payload = &line[6..];
        if payload == "[DONE]" {
            all.extend(mapper.on_done().unwrap());
            continue;
        }
        all.extend(mapper.feed(payload).unwrap());
    }

    assert!(all.iter().any(|e| matches!(e, ResponseEvent::ReasoningDelta(_))));
    assert!(all.iter().any(|e| matches!(e, ResponseEvent::ContentDelta(_))));
    assert!(all.iter().any(|e| matches!(e, ResponseEvent::ToolDone(_))));
    assert!(all.iter().any(|e| matches!(e, ResponseEvent::Done(Some(_)))));
}

#[test]
fn replay_fixture_has_correct_event_order() {
    let lines = FIXTURE;
    let mut mapper = Mapper::new("openai".into());

    let mut all = Vec::new();
    for line in lines.lines().filter(|l| l.starts_with("data: ")) {
        let payload = &line[6..];
        if payload == "[DONE]" {
            all.extend(mapper.on_done().unwrap());
            continue;
        }
        all.extend(mapper.feed(payload).unwrap());
    }

    // Verify Created comes first
    let created_idx = all.iter().position(|e| matches!(e, ResponseEvent::Created(_)));
    assert!(created_idx.is_some());

    // Verify Done comes last
    let done_idx = all.iter().rposition(|e| matches!(e, ResponseEvent::Done(_)));
    assert!(done_idx.is_some());

    // Verify Done is after Created
    if let (Some(c), Some(d)) = (created_idx, done_idx) {
        assert!(c < d, "Created should come before Done");
    }
}
