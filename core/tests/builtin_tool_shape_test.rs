use core::{Builtin, BuiltinToolCall, ToolCall};

#[test]
fn builtin_tool_call_shape_matches_contract() {
    let call = ToolCall::Builtin(BuiltinToolCall {
        sequence: 0,
        call_id: "call_builtin_1".into(),
        builtin: Builtin::Read,
        arguments_json: r#"{"path":"Cargo.toml"}"#.into(),
    });

    match call {
        ToolCall::Builtin(inner) => assert!(matches!(inner.builtin, Builtin::Read)),
        _ => panic!("expected builtin tool call"),
    }
}

#[test]
fn git_builtin_name_and_variant() {
    assert_eq!(Builtin::Git.canonical_name(), "git");
    assert!(matches!(Builtin::from_name("git"), Some(Builtin::Git)));
}
