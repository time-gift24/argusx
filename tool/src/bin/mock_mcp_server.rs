use std::io::{self, BufRead, Write};

use serde_json::{Value, json};

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut initialized = false;
    let mut got_initialized_notification = false;

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let message: Value =
            serde_json::from_str(&line).expect("mock server should only receive valid json");
        let method = message.get("method").and_then(Value::as_str);
        let id = message.get("id").and_then(Value::as_u64);

        match (method, id) {
            (Some("initialize"), Some(id)) => {
                initialized = true;
                write_json_line(
                    &mut stdout,
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "protocolVersion": "2025-06-18",
                            "capabilities": {
                                "tools": {}
                            },
                            "serverInfo": {
                                "name": "mock-mcp-server",
                                "version": "0.1.0"
                            }
                        }
                    }),
                )?;
            }
            (Some("notifications/initialized"), None) => {
                assert!(
                    initialized,
                    "initialized notification must follow initialize"
                );
                got_initialized_notification = true;
            }
            (Some("tools/list"), Some(id)) => {
                assert!(
                    got_initialized_notification,
                    "tools/list must not be called before notifications/initialized"
                );
                write_json_line(
                    &mut stdout,
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "tools": [
                                {
                                    "name": "echo",
                                    "description": "Echo structured content",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {
                                            "text": { "type": "string" }
                                        },
                                        "required": ["text"]
                                    }
                                }
                            ]
                        }
                    }),
                )?;
            }
            (Some("tools/call"), Some(id)) => {
                assert!(
                    got_initialized_notification,
                    "tools/call must not be called before notifications/initialized"
                );
                let params = message
                    .get("params")
                    .expect("tools/call params should exist");
                let tool_name = params
                    .get("name")
                    .and_then(Value::as_str)
                    .expect("tool name should exist");
                assert_eq!(tool_name, "echo");

                let text = params
                    .get("arguments")
                    .and_then(|args| args.get("text"))
                    .and_then(Value::as_str)
                    .expect("echo arguments.text should exist");

                write_json_line(
                    &mut stdout,
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [
                                { "type": "text", "text": text }
                            ],
                            "structuredContent": {
                                "text": text
                            }
                        }
                    }),
                )?;
            }
            (Some(other), Some(id)) => {
                write_json_line(
                    &mut stdout,
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32601,
                            "message": format!("unknown method: {other}")
                        }
                    }),
                )?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn write_json_line(stdout: &mut io::Stdout, value: Value) -> io::Result<()> {
    writeln!(stdout, "{value}")?;
    stdout.flush()
}
