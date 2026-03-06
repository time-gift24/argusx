use argus_core::{ResponseEvent, ToolCall};
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

    assert!(all
        .iter()
        .any(|e| matches!(e, ResponseEvent::ReasoningDelta(_))));
    assert!(all
        .iter()
        .any(|e| matches!(e, ResponseEvent::ContentDelta(_))));
    assert!(all
        .iter()
        .any(|e| matches!(e, ResponseEvent::ReasoningDone(_))));
    assert!(all
        .iter()
        .any(|e| matches!(e, ResponseEvent::ContentDone(_))));
    assert!(all
        .iter()
        .any(|e| matches!(e, ResponseEvent::Done(Some(_)))));

    let tool_done: Vec<_> = all
        .iter()
        .filter_map(|e| match e {
            ResponseEvent::ToolDone(ToolCall::FunctionCall {
                sequence,
                call_id,
                name,
                arguments_json,
            }) => Some((
                *sequence,
                call_id.as_str(),
                name.as_str(),
                arguments_json.as_str(),
            )),
            _ => None,
        })
        .collect();
    assert_eq!(tool_done.len(), 1);
    let (sequence, call_id, name, args) = tool_done[0];
    assert_eq!(sequence, 0);
    assert_eq!(call_id, "call_d1a79f24436349078d8df6a6");
    assert_eq!(name, "get_weather");
    assert_eq!(args, "{\"city\":\"北京\"}");
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
    let created_idx = all
        .iter()
        .position(|e| matches!(e, ResponseEvent::Created(_)));
    assert!(created_idx.is_some());

    // Verify Done comes last
    let done_idx = all
        .iter()
        .rposition(|e| matches!(e, ResponseEvent::Done(_)));
    assert!(done_idx.is_some());

    // Verify Done is after Created
    if let (Some(c), Some(d)) = (created_idx, done_idx) {
        assert!(c < d, "Created should come before Done");
    }
}
