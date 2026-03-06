use tool::{Tool, ToolContext, ToolError};

#[test]
fn tool_crate_exports_runtime_primitives() {
    let _ctx = ToolContext {
        session_id: "s1".into(),
        turn_id: "t1".into(),
    };
    let _ = std::mem::size_of::<ToolError>();

    fn assert_tool<T: Tool>() {}

    assert_tool::<tool::builtin::read::ReadTool>();
}
