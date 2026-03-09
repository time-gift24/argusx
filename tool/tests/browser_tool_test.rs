//! Browser tool smoke test
//!
//! This test verifies that the BrowserTool module compiles and can be instantiated.
//! Since Chrome is required for actual browser tests, this is a compile-only smoke test
//! that verifies the module structure is correct.

#[test]
fn browser_tool_satisfies_tool_trait() {
    fn assert_tool<T: tool::Tool>() {}

    // Verify BrowserTool implements the Tool trait
    assert_tool::<tool::builtin::BrowserTool>();
}
