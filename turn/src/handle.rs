use std::sync::Arc;

use tokio::sync::{Mutex, mpsc};

use crate::{TurnCommand, TurnError, TurnEvent};

#[derive(Clone)]
pub struct TurnHandle {
    command_tx: mpsc::Sender<TurnCommand>,
    event_rx: Arc<Mutex<mpsc::Receiver<TurnEvent>>>,
}

impl TurnHandle {
    pub(crate) fn new(
        command_tx: mpsc::Sender<TurnCommand>,
        event_rx: mpsc::Receiver<TurnEvent>,
    ) -> Self {
        Self {
            command_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
        }
    }

    pub async fn next_event(&self) -> Option<TurnEvent> {
        self.event_rx.lock().await.recv().await
    }

    pub async fn cancel(&self) -> Result<(), TurnError> {
        self.command_tx
            .send(TurnCommand::Cancel)
            .await
            .map_err(|_| TurnError::Runtime("turn command receiver dropped".into()))
    }

    pub async fn resolve_permission(
        &self,
        request_id: String,
        decision: crate::PermissionDecision,
    ) -> Result<(), TurnError> {
        self.command_tx
            .send(TurnCommand::ResolvePermission {
                request_id,
                decision,
            })
            .await
            .map_err(|_| TurnError::Runtime("turn command receiver dropped".into()))
    }
}
