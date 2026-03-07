use std::sync::Arc;

use turn::TurnError;

pub mod authorizer;
pub mod commands;
pub mod events;
pub mod manager;
pub mod model;
pub mod observer;
pub mod tools;

pub use authorizer::AllowListedToolAuthorizer;
pub use commands::{cancel_turn, start_turn};
pub use events::{DesktopTurnEvent, StartTurnInput, StartTurnResult, TurnTargetKind};
pub use manager::TurnManager;
pub use model::ProviderModelRunner;
pub use observer::TauriTurnObserver;
pub use tools::ScheduledToolRunner;

pub struct AppState {
    turn_manager: Arc<TurnManager>,
    provider_settings: Arc<crate::provider_settings::ProviderSettingsService>,
    tool_runner: Arc<ScheduledToolRunner>,
    tool_authorizer: Arc<AllowListedToolAuthorizer>,
}

impl AppState {
    pub fn new() -> Result<Self, TurnError> {
        let provider_settings = crate::provider_settings::ProviderSettingsService::from_default_location()
            .map_err(|err| TurnError::Runtime(err.to_string()))?;
        Ok(Self {
            turn_manager: Arc::new(TurnManager::new()),
            provider_settings: Arc::new(provider_settings),
            tool_runner: Arc::new(ScheduledToolRunner::from_current_dir()?),
            tool_authorizer: Arc::new(AllowListedToolAuthorizer),
        })
    }

    pub fn turn_manager(&self) -> Arc<TurnManager> {
        Arc::clone(&self.turn_manager)
    }

    pub fn model_runner(&self) -> Result<Arc<ProviderModelRunner>, TurnError> {
        Ok(Arc::new(ProviderModelRunner::from_provider_settings(
            Some(self.provider_settings.as_ref()),
        )?))
    }

    pub fn provider_settings(&self) -> Arc<crate::provider_settings::ProviderSettingsService> {
        Arc::clone(&self.provider_settings)
    }

    pub fn tool_runner(&self) -> Arc<ScheduledToolRunner> {
        Arc::clone(&self.tool_runner)
    }

    pub fn tool_authorizer(&self) -> Arc<AllowListedToolAuthorizer> {
        Arc::clone(&self.tool_authorizer)
    }
}
