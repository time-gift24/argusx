use std::process::Command;

#[test]
fn help_shows_subcommands() {
    let output = Command::new(env!("CARGO_BIN_EXE_agent-session-cli"))
        .arg("--help")
        .output()
        .expect("run cli");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--store-dir"));
    assert!(stdout.contains("create"));
    assert!(stdout.contains("list"));
    assert!(stdout.contains("run"));
}
