#[test]
fn create_then_list_returns_session() {
    let temp = tempfile::tempdir().expect("tmp");

    let create = std::process::Command::new(env!("CARGO_BIN_EXE_agent-session-cli"))
        .args([
            "--store-dir",
            temp.path().to_str().unwrap(),
            "create",
            "--title",
            "demo",
            "--json",
        ])
        .output()
        .expect("run create");
    assert!(create.status.success());

    let list = std::process::Command::new(env!("CARGO_BIN_EXE_agent-session-cli"))
        .args([
            "--store-dir",
            temp.path().to_str().unwrap(),
            "list",
            "--json",
        ])
        .output()
        .expect("run list");
    assert!(list.status.success());
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(stdout.contains("\"title\":\"demo\""));
}
