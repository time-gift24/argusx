use agent_tool::builtin::fs::guard::FsGuard;

#[test]
fn fs_guard_type_is_exposed() {
    let _ = std::any::type_name::<FsGuard>();
}
