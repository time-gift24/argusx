use argus_core::{ResponseEvent, ToolCall};
use provider::{Dialect, Mapper};

const OPENAI_FIXTURE: &str = include_str!("fixtures/2026-03-06-openai-chat-completions-sse.txt");

#[test]
fn openai_fixture_still_emits_expected_sequence() {
    let mut mapper = Mapper::new(Dialect::Openai);
    let mut all = Vec::new();

    for line in OPENAI_FIXTURE.lines().filter(|l| l.starts_with("data: ")) {
        let payload = &line[6..];
        if payload == "[DONE]" {
            all.extend(mapper.on_done().unwrap());
            continue;
        }
        all.extend(mapper.feed(payload).unwrap());
    }

    assert!(
        all.iter()
            .any(|e| matches!(e, ResponseEvent::ReasoningDelta(_)))
    );
    assert!(
        all.iter()
            .any(|e| matches!(e, ResponseEvent::ContentDelta(_)))
    );
    assert!(
        all.iter()
            .any(|e| matches!(e, ResponseEvent::Done(Some(_))))
    );

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
    assert_eq!(tool_done[0].0, 0);
    assert_eq!(tool_done[0].1, "call_d1a79f24436349078d8df6a6");
    assert_eq!(tool_done[0].2, "get_weather");
    assert_eq!(tool_done[0].3, "{\"city\":\"北京\"}");
}
