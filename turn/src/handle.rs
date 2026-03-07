use tokio::sync::{Mutex, mpsc};

use crate::{TurnCommand, TurnError, TurnEvent};

#[derive(Clone)]
pub struct TurnController {
    command_tx: mpsc::Sender<TurnCommand>,
}

pub struct TurnHandle {
    controller: TurnController,
    event_rx: Mutex<mpsc::Receiver<TurnEvent>>,
}

impl TurnHandle {
    pub(crate) fn new(
        command_tx: mpsc::Sender<TurnCommand>,
        event_rx: mpsc::Receiver<TurnEvent>,
    ) -> Self {
        Self {
            controller: TurnController { command_tx },
            event_rx: Mutex::new(event_rx),
        }
    }

    pub fn controller(&self) -> TurnController {
        self.controller.clone()
    }

    pub async fn next_event(&self) -> Option<TurnEvent> {
        self.event_rx.lock().await.recv().await
    }

    pub async fn cancel(&self) -> Result<(), TurnError> {
        self.controller.cancel().await
    }

    pub async fn resolve_permission(
        &self,
        request_id: String,
        decision: crate::PermissionDecision,
    ) -> Result<(), TurnError> {
        self.controller
            .resolve_permission(request_id, decision)
            .await
    }
}

impl TurnController {
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
