mod support;

use turn::{ModelRunner, ToolAuthorizer, ToolRunner};

fn assert_model<T: ModelRunner>() {}
fn assert_tool<T: ToolRunner>() {}
fn assert_authorizer<T: ToolAuthorizer>() {}

#[test]
fn turn_runtime_traits_are_object_safe() {
    assert_model::<support::FakeModelRunner>();
    assert_tool::<support::FakeToolRunner>();
    assert_authorizer::<support::FakeAuthorizer>();
}
