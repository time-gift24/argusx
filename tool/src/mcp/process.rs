use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::{Mutex, oneshot},
};

use super::{
    client::{McpError, McpStdioConfig},
    transport::{RpcResponse, notification_message, request_message},
};

type PendingMap = Arc<Mutex<BTreeMap<u64, oneshot::Sender<Result<Value, McpError>>>>>;

#[derive(Debug)]
pub struct McpSession {
    server_label: String,
    child: Mutex<Child>,
    writer: Mutex<ChildStdin>,
    pending: PendingMap,
    next_id: AtomicU64,
}

pub async fn spawn_stdio_session(config: &McpStdioConfig) -> Result<McpSession, McpError> {
    let mut command = Command::new(&config.command);
    command.args(&config.args);
    command.kill_on_drop(true);
    command.stdin(std::process::Stdio::piped());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::inherit());

    if let Some(cwd) = &config.cwd {
        command.current_dir(cwd);
    }

    for (key, value) in &config.env {
        command.env(key, value);
    }

    let mut child = command.spawn()?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| McpError::Protocol("child stdin pipe unavailable".to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| McpError::Protocol("child stdout pipe unavailable".to_string()))?;

    let pending = Arc::new(Mutex::new(BTreeMap::new()));
    tokio::spawn(read_stdout_loop(
        config.server_label.clone(),
        stdout,
        Arc::clone(&pending),
    ));

    let session = McpSession {
        server_label: config.server_label.clone(),
        child: Mutex::new(child),
        writer: Mutex::new(stdin),
        pending,
        next_id: AtomicU64::new(1),
    };

    session
        .request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "argusx-tool",
                    "version": env!("CARGO_PKG_VERSION"),
                },
            }),
        )
        .await?;
    session
        .notify("notifications/initialized", json!({}))
        .await?;

    Ok(session)
}

impl McpSession {
    pub async fn request(&self, method: &str, params: Value) -> Result<Value, McpError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending.lock().await;
            pending.insert(id, tx);
        }

        let message = request_message(id, method, params);
        if let Err(err) = self.write_message(&message).await {
            let mut pending = self.pending.lock().await;
            pending.remove(&id);
            return Err(err);
        }

        rx.await
            .map_err(|_| McpError::ProcessExited(self.server_label.clone()))?
    }

    pub async fn notify(&self, method: &str, params: Value) -> Result<(), McpError> {
        let message = notification_message(method, params);
        self.write_message(&message).await
    }

    async fn write_message(&self, message: &Value) -> Result<(), McpError> {
        let mut writer = self.writer.lock().await;
        let mut encoded = serde_json::to_vec(message)?;
        encoded.push(b'\n');
        writer.write_all(&encoded).await?;
        writer.flush().await?;
        Ok(())
    }
}

impl Drop for McpSession {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.try_lock() {
            let _ = child.start_kill();
        }
    }
}

async fn read_stdout_loop(server_label: String, stdout: ChildStdout, pending: PendingMap) {
    let mut lines = BufReader::new(stdout).lines();

    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                if line.trim().is_empty() {
                    continue;
                }

                let parsed = serde_json::from_str::<RpcResponse>(&line)
                    .map_err(McpError::from)
                    .and_then(validate_response);

                match parsed {
                    Ok(response) => {
                        if let Some(sender) = pending.lock().await.remove(&response.id) {
                            let result = if let Some(error) = response.error {
                                Err(McpError::Server {
                                    code: error.code,
                                    message: error.message,
                                })
                            } else {
                                response.result.ok_or_else(|| {
                                    McpError::Protocol(format!(
                                        "missing result field in response from `{server_label}`"
                                    ))
                                })
                            };
                            let _ = sender.send(result);
                        }
                    }
                    Err(err) => {
                        fail_pending(&pending, err).await;
                        return;
                    }
                }
            }
            Ok(None) => {
                fail_pending(&pending, McpError::ProcessExited(server_label.clone())).await;
                return;
            }
            Err(err) => {
                fail_pending(&pending, McpError::Io(err)).await;
                return;
            }
        }
    }
}

fn validate_response(response: RpcResponse) -> Result<RpcResponse, McpError> {
    if response.jsonrpc != "2.0" {
        return Err(McpError::Protocol(format!(
            "unexpected jsonrpc version `{}`",
            response.jsonrpc
        )));
    }

    Ok(response)
}

async fn fail_pending(pending: &PendingMap, err: McpError) {
    let mut pending = pending.lock().await;
    let senders: Vec<_> = std::mem::take(&mut *pending).into_values().collect();
    drop(pending);

    for sender in senders {
        let _ = sender.send(Err(match &err {
            McpError::Io(inner) => McpError::Io(std::io::Error::new(inner.kind(), inner.to_string())),
            McpError::Json(inner) => {
                McpError::Protocol(format!("invalid JSON message from server: {inner}"))
            }
            McpError::Protocol(message) => McpError::Protocol(message.clone()),
            McpError::Server { code, message } => McpError::Server {
                code: *code,
                message: message.clone(),
            },
            McpError::ProcessExited(label) => McpError::ProcessExited(label.clone()),
        }));
    }
}
