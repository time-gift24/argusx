use std::sync::Arc;

use agent_core::{CheckpointStore, RunStreamEvent, RuntimeEvent, UiThreadEvent};
use tokio::sync::mpsc;

use crate::bus::EventBus;
use crate::command::{normalizer::CommandNormalizer, DomainCommand};
use crate::domain::DomainEvent;
use crate::effect::EffectExecutor;
use crate::handlers::HandlerRegistry;
use crate::journal::TranscriptJournal;
use crate::output::OutputEvent;
use crate::projection::{emit_run_events, emit_ui_events};
use crate::projectors::{output::OutputProjector, state::StateProjector};
use crate::reducer::reduce;
use crate::state::{Lifecycle, TurnEngineConfig, TurnState};
use crate::transition::Transition;

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
    pub bus: EventBus,
    pub normalizer: CommandNormalizer,
    pub handlers: HandlerRegistry,
}

impl<L, T> TurnEngine<L, T>
where
    L: agent_core::LanguageModel + 'static,
    T: agent_core::tools::ToolExecutor + agent_core::tools::ToolCatalog + 'static,
{
    pub async fn run(mut self) -> TurnState {
        while let Some(event) = self.event_rx.recv().await {
            self.handle_runtime_event_with_bus(event).await;

            if matches!(self.state.lifecycle, Lifecycle::Done | Lifecycle::Failed) {
                break;
            }
        }

        self.state
    }

    async fn handle_runtime_event_with_bus(&mut self, event: RuntimeEvent) {
        let Some(cmd) = self
            .normalizer
            .normalize(DomainCommand::from_runtime(event.clone()))
        else {
            return;
        };

        if self.bus.enqueue_command(cmd).is_err() {
            let _ = self.run_tx.send(RunStreamEvent::ProtocolWarning {
                turn_id: self.state.meta.turn_id.clone(),
                message: "event bus command queue is full".to_string(),
            });
            self.apply_legacy_event(event).await;
            return;
        }

        while let Some(next_cmd) = self.bus.dequeue_command() {
            let domain_events = self.handlers.handle(next_cmd, &self.state);
            for domain_event in domain_events {
                match domain_event {
                    DomainEvent::LegacyRuntimeEvent { event } => {
                        self.apply_legacy_event(event).await;
                    }
                    other => {
                        StateProjector::apply(&mut self.state, &other);
                        let outputs = OutputProjector::map(&self.state, &other);
                        for output in outputs {
                            self.dispatch_output(output).await;
                        }
                    }
                }
            }
        }
    }

    async fn dispatch_output(&self, output: OutputEvent) {
        match output {
            OutputEvent::Run(event) => {
                let _ = self.run_tx.send(event);
            }
            OutputEvent::Ui(event) => {
                let _ = self.ui_tx.send(event);
            }
            OutputEvent::Effect(effect) => {
                self.effect_executor.execute(effect).await;
            }
            OutputEvent::Noop => {}
        }
    }

    async fn apply_transition(&mut self, transition: Transition) {
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
    }

    async fn apply_legacy_event(&mut self, event: RuntimeEvent) {
        let transition = reduce(self.state.clone(), event, &self.config);
        self.apply_transition(transition).await;
    }
}
