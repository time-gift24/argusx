use agent_core::{RunStreamEvent, RuntimeEvent, UiThreadEvent};
use tokio::sync::mpsc;

use crate::effect::EffectExecutor;
use crate::journal::TranscriptJournal;
use crate::projection::{emit_run_events, emit_ui_events};
use crate::reducer::reduce;
use crate::state::{Lifecycle, TurnEngineConfig, TurnState};

pub struct TurnEngine<L, T>
where
    L: agent_core::LanguageModel + 'static,
    T: agent_core::tools::ToolExecutor + 'static,
{
    pub config: TurnEngineConfig,
    pub state: TurnState,
    pub journal: TranscriptJournal,
    pub effect_executor: EffectExecutor<L, T>,
    pub event_rx: mpsc::UnboundedReceiver<RuntimeEvent>,
    pub run_tx: mpsc::UnboundedSender<RunStreamEvent>,
    pub ui_tx: mpsc::UnboundedSender<UiThreadEvent>,
}

impl<L, T> TurnEngine<L, T>
where
    L: agent_core::LanguageModel + 'static,
    T: agent_core::tools::ToolExecutor + 'static,
{
    pub async fn run(mut self) -> TurnState {
        while let Some(event) = self.event_rx.recv().await {
            let transition = reduce(self.state, event, &self.config);
            self.state = transition.state;

            self.journal.append(&transition.new_items).await;
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
