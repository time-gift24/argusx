use argus_core::{ResponseEvent, ToolCall};
use provider::{Dialect, Mapper};

const ZAI_FIXTURE: &str = include_str!("fixtures/2026-03-06-zai-chat-completions-sse.txt");

#[test]
fn replay_zai_fixture_emits_ordered_mcp_and_done_usage() {
    let mut mapper = Mapper::new(Dialect::Zai);
    let mut all = Vec::new();

    for line in ZAI_FIXTURE.lines().filter(|l| l.starts_with("data: ")) {
        let payload = &line[6..];
        if payload == "[DONE]" {
            all.extend(mapper.on_done().unwrap());
            continue;
        }
        all.extend(mapper.feed(payload).unwrap());
    }
    all.extend(mapper.on_done().unwrap());

    let tools: Vec<_> = all
        .iter()
        .filter_map(|e| match e {
            ResponseEvent::ToolDone(tc) => Some(tc),
            _ => None,
        })
        .collect();

    assert_eq!(tools.len(), 2, "tool calls should not be duplicated");

    assert!(matches!(
        tools[0],
        ToolCall::Mcp(call)
            if call.sequence == 0
                && call.id == "call_mcp_1"
                && call.server_label.as_deref() == Some("filesystem")
    ));

    assert!(matches!(
        tools[1],
        ToolCall::FunctionCall { sequence, call_id, name, arguments_json }
            if *sequence == 1
                && call_id == "call_fn_2"
                && name == "native__calculator__multiply"
                && arguments_json.contains("\"a\":88")
    ));

    let usage = all.iter().rev().find_map(|e| match e {
        ResponseEvent::Done(Some(usage)) => Some(*usage),
        _ => None,
    });
    assert!(usage.is_some());
    let usage = usage.unwrap();
    assert_eq!(usage.input_tokens, 316);
    assert_eq!(usage.output_tokens, 113);
    assert_eq!(usage.total_tokens, 429);
}
