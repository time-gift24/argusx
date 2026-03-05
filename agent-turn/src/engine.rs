use std::sync::Arc;

use agent_core::{CheckpointStore, RunStreamEvent, RuntimeEvent, UiThreadEvent};
use tokio::sync::mpsc;

use crate::effect::EffectExecutor;
use crate::journal::TranscriptJournal;
use crate::projection::{emit_run_events, emit_ui_events};
use crate::reducer::reduce;
use crate::state::{Lifecycle, TurnEngineConfig, TurnState};

pub struct TurnEngine<L, T>
where
    L: agent_core::LanguageModel + 'static,
    T: agent_core::tools::ToolExecutor + agent_core::tools::ToolCatalog + 'static,
{
    pub config: TurnEngineConfig,
    pub state: TurnState,
    pub journal: TranscriptJournal,
    pub effect_executor: EffectExecutor<L, T>,
    pub event_rx: mpsc::UnboundedReceiver<RuntimeEvent>,
    pub run_tx: mpsc::UnboundedSender<RunStreamEvent>,
    pub ui_tx: mpsc::UnboundedSender<UiThreadEvent>,
    pub checkpoint_store: Option<Arc<dyn CheckpointStore>>,
}

impl<L, T> TurnEngine<L, T>
where
    L: agent_core::LanguageModel + 'static,
    T: agent_core::tools::ToolExecutor + agent_core::tools::ToolCatalog + 'static,
{
    pub async fn run(mut self) -> TurnState {
        while let Some(event) = self.event_rx.recv().await {
            let transition = reduce(self.state, event, &self.config);
            self.state = transition.state;

            self.journal.append(&transition.new_items).await;
            if let Some(store) = self.checkpoint_store.as_ref() {
                if !transition.new_items.is_empty() {
                    if let Err(err) = store
                        .append_items(self.state.turn_id(), &transition.new_items)
                        .await
                    {
                        let warning = format!("checkpoint append failed: {err}");
                        let _ = self.run_tx.send(RunStreamEvent::ProtocolWarning {
                            turn_id: self.state.meta.turn_id.clone(),
                            message: warning.clone(),
                        });
                        let _ = self.ui_tx.send(UiThreadEvent::Warning {
                            turn_id: self.state.meta.turn_id.clone(),
                            message: warning,
                        });
                    }
                }
            }
            emit_run_events(&self.run_tx, transition.run_events);
            emit_ui_events(&self.ui_tx, transition.ui_events);

            for effect in transition.effects {
                self.effect_executor.execute(effect).await;
            }

            if matches!(self.state.lifecycle, Lifecycle::Done | Lifecycle::Failed) {
                break;
            }
        }

        self.state
    }
}
