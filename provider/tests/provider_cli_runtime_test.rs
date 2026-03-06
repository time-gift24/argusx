use std::io::Write;
use std::process::{Command, Stdio};

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn provider_cli_streams_without_nested_runtime_panic() {
    let server = MockServer::start().await;
    let body = concat!(
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",",
        "\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hello\"}}]}\n\n",
        "data: [DONE]\n\n"
    );

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
        .mount(&server)
        .await;

    let mut child = Command::new(env!("CARGO_BIN_EXE_provider_cli"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run provider_cli");

    {
        let stdin = child.stdin.as_mut().expect("child stdin");
        write!(stdin, "1\ntest-key\n{}\ngpt-test\nhello\n", server.uri()).expect("write input");
    }

    let output = child.wait_with_output().expect("wait for provider_cli");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "stdout:\n{stdout}\n\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("hello"),
        "stdout:\n{stdout}\n\nstderr:\n{stderr}"
    );
}
