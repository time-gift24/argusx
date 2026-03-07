use tokio_util::sync::CancellationToken;
use tool::{Tool, ToolContext, ToolError};

#[test]
fn tool_crate_exports_runtime_primitives() {
    let _ctx = ToolContext::new("s1", "t1", CancellationToken::new());
    let _ = std::mem::size_of::<ToolError>();

    fn assert_tool<T: Tool>() {}

    assert_tool::<tool::builtin::read::ReadTool>();
}
