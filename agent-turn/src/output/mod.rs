use agent_core::{RunStreamEvent, UiThreadEvent};

use crate::effect::Effect;

#[derive(Debug, Clone)]
pub enum OutputEvent {
    Run(RunStreamEvent),
    Ui(UiThreadEvent),
    Effect(Effect),
    Noop,
}
