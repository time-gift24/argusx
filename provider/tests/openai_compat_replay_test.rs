use argus_core::{ResponseEvent, ToolCall};
use provider::{Dialect, Mapper};

const OPENAI_FIXTURE: &str = include_str!("fixtures/2026-03-06-openai-chat-completions-sse.txt");
const MINIMAX_FIXTURE: &str = include_str!("fixtures/2026-03-06-minimax-chat-completions-sse.txt");

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

#[test]
fn openai_fixture_has_correct_event_order() {
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

    let created_idx = all
        .iter()
        .position(|e| matches!(e, ResponseEvent::Created(_)));
    assert!(created_idx.is_some());

    let done_idx = all
        .iter()
        .rposition(|e| matches!(e, ResponseEvent::Done(_)));
    assert!(done_idx.is_some());

    if let (Some(c), Some(d)) = (created_idx, done_idx) {
        assert!(c < d, "Created should come before Done");
    }
}

#[test]
fn minimax_fixture_emits_ordered_tool_calls_and_final_usage() {
    let mut mapper = Mapper::new(Dialect::Openai);
    let mut all = Vec::new();
    for line in MINIMAX_FIXTURE.lines().filter(|l| l.starts_with("data: ")) {
        let payload = &line[6..];
        if payload == "[DONE]" {
            all.extend(mapper.on_done().unwrap());
            continue;
        }
        all.extend(mapper.feed(payload).unwrap());
    }
    all.extend(mapper.on_done().unwrap());

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

    assert_eq!(tool_done.len(), 2);
    assert_eq!(tool_done[0].0, 0);
    assert_eq!(tool_done[0].1, "call_function_wdjb8wrs8sjw_1");
    assert_eq!(tool_done[0].2, "mcp__filesystem__read_file");
    assert_eq!(tool_done[0].3, "{\"path\": \"./config.yaml\"}");

    assert_eq!(tool_done[1].0, 1);
    assert_eq!(tool_done[1].1, "call_function_wdjb8wrs8sjw_2");
    assert_eq!(tool_done[1].2, "native__calculator__multiply");
    assert_eq!(tool_done[1].3, "{\"a\": 88, \"b\": 99}");

    assert!(
        all.iter()
            .any(|e| matches!(e, ResponseEvent::ReasoningDelta(_)))
    );
    assert!(
        all.iter()
            .any(|e| matches!(e, ResponseEvent::ReasoningDone(_)))
    );
    assert!(
        all.iter()
            .any(|e| matches!(e, ResponseEvent::Done(Some(_))))
    );

    let done_usage = all.iter().rev().find_map(|e| match e {
        ResponseEvent::Done(Some(usage)) => Some(*usage),
        _ => None,
    });
    assert!(done_usage.is_some());
    let usage = done_usage.unwrap();
    assert_eq!(usage.input_tokens, 316);
    assert_eq!(usage.output_tokens, 113);
    assert_eq!(usage.total_tokens, 429);
}
