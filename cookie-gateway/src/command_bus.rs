use async_trait::async_trait;
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};

use crate::error::CookieGatewayError;
use crate::tool::CookieCommandClient;
use crate::CookieData;

#[derive(Default)]
pub struct GatewayCommandBus {
    connection: RwLock<Option<mpsc::UnboundedSender<Message>>>,
    pending: Mutex<HashMap<String, oneshot::Sender<Result<Vec<CookieData>, String>>>>,
    request_seq: AtomicU64,
}

impl GatewayCommandBus {
    pub fn new() -> Self {
        Self {
            connection: RwLock::new(None),
            pending: Mutex::new(HashMap::new()),
            request_seq: AtomicU64::new(1),
        }
    }

    pub async fn handle_websocket(self: Arc<Self>, socket: WebSocket) {
        let (mut ws_sender, mut ws_receiver) = socket.split();
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

        self.set_connection(tx.clone()).await;

        let writer = tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if ws_sender.send(message).await.is_err() {
                    break;
                }
            }
        });

        while let Some(message_result) = ws_receiver.next().await {
            let message = match message_result {
                Ok(message) => message,
                Err(err) => {
                    tracing::warn!("websocket receive error: {err}");
                    break;
                }
            };

            if let Message::Text(text) = message {
                self.handle_text_message(&tx, text.as_ref()).await;
            }
        }

        self.clear_connection_if_same(&tx).await;
        self.fail_all_pending("extension client disconnected").await;
        writer.abort();
    }

    async fn handle_text_message(&self, tx: &mpsc::UnboundedSender<Message>, text: &str) {
        let parsed: IncomingEnvelope = match serde_json::from_str(text) {
            Ok(value) => value,
            Err(_) => {
                tracing::debug!("ignored non-json websocket payload");
                return;
            }
        };

        if parsed.msg_type.as_deref() == Some("PING") {
            let pong = json!({
                "type": "PONG",
                "timestamp": chrono_like_timestamp(),
            })
            .to_string();
            let _ = tx.send(Message::Text(pong.into()));
            return;
        }

        if parsed.msg_type.as_deref() == Some("ACTION_RESULT") {
            self.resolve_pending(parsed).await;
        }
    }

    async fn resolve_pending(&self, envelope: IncomingEnvelope) {
        let request_id = match envelope.request_id {
            Some(id) if !id.is_empty() => id,
            _ => return,
        };

        let tx = self.pending.lock().await.remove(&request_id);
        let Some(tx) = tx else {
            return;
        };

        if envelope.ok.unwrap_or(false) {
            let cookies = envelope
                .result
                .and_then(|result| result.cookies)
                .unwrap_or_default();
            let _ = tx.send(Ok(cookies));
        } else {
            let reason = envelope
                .error
                .unwrap_or_else(|| "extension action failed".to_string());
            let _ = tx.send(Err(reason));
        }
    }

    async fn set_connection(&self, sender: mpsc::UnboundedSender<Message>) {
        *self.connection.write().await = Some(sender);
    }

    async fn clear_connection_if_same(&self, sender: &mpsc::UnboundedSender<Message>) {
        let mut guard = self.connection.write().await;
        if let Some(current) = guard.as_ref() {
            if current.same_channel(sender) {
                *guard = None;
            }
        }
    }

    async fn fail_all_pending(&self, reason: &str) {
        let mut pending = self.pending.lock().await;
        for (_, tx) in pending.drain() {
            let _ = tx.send(Err(reason.to_string()));
        }
    }

    fn next_request_id(&self) -> String {
        let seq = self.request_seq.fetch_add(1, Ordering::Relaxed);
        format!("get-cookies-{seq}")
    }
}

#[async_trait]
impl CookieCommandClient for GatewayCommandBus {
    async fn request_cookies(
        &self,
        domain: &str,
        timeout: Duration,
    ) -> Result<Vec<CookieData>, CookieGatewayError> {
        let sender = self
            .connection
            .read()
            .await
            .as_ref()
            .cloned()
            .ok_or(CookieGatewayError::ExtensionClientUnavailable)?;

        let request_id = self.next_request_id();
        let (result_tx, result_rx) = oneshot::channel();

        self.pending
            .lock()
            .await
            .insert(request_id.clone(), result_tx);

        let command = json!({
            "requestId": request_id,
            "action": "GET_COOKIES",
            "domain": domain,
        })
        .to_string();

        if sender.send(Message::Text(command.into())).is_err() {
            self.pending.lock().await.remove(&request_id);
            return Err(CookieGatewayError::ExtensionClientUnavailable);
        }

        match tokio::time::timeout(timeout, result_rx).await {
            Ok(Ok(Ok(cookies))) => Ok(cookies),
            Ok(Ok(Err(message))) => Err(CookieGatewayError::ExtensionCommandFailed { message }),
            Ok(Err(_)) => Err(CookieGatewayError::ExtensionCommandFailed {
                message: "extension response channel closed".to_string(),
            }),
            Err(_) => {
                self.pending.lock().await.remove(&request_id);
                Err(CookieGatewayError::ExtensionCommandTimeout {
                    domain: domain.to_string(),
                    timeout_ms: timeout.as_millis() as u64,
                })
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct IncomingEnvelope {
    #[serde(default, rename = "type")]
    msg_type: Option<String>,
    #[serde(default, rename = "requestId")]
    request_id: Option<String>,
    #[serde(default)]
    ok: Option<bool>,
    #[serde(default)]
    result: Option<ActionResultPayload>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ActionResultPayload {
    #[serde(default)]
    cookies: Option<Vec<CookieData>>,
}

fn chrono_like_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:03}Z", now.as_secs(), now.subsec_millis())
}
