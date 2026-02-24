use std::process::Command;

#[test]
fn help_shows_chat_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_agent-cli"))
        .arg("--help")
        .output()
        .expect("run agent-cli --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--session"));
    assert!(stdout.contains("--store-dir"));
    assert!(stdout.contains("--api-key"));
}
