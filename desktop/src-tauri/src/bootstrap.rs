use std::path::{Path, PathBuf};

use turn::TurnError;

use crate::{provider_settings::ProviderSettingsService, session_commands::DesktopSessionState};

pub struct DesktopBootstrap {
    pub runtime: runtime::ArgusxRuntime,
    pub session_state: DesktopSessionState,
    pub provider_settings_db_path: PathBuf,
    pub workspace_root: PathBuf,
}

pub fn build_desktop_bootstrap(
    runtime: runtime::ArgusxRuntime,
) -> Result<DesktopBootstrap, TurnError> {
    let workspace_root =
        std::env::current_dir().map_err(|err| TurnError::Runtime(err.to_string()))?;
    build_desktop_bootstrap_with_workspace_root(runtime, workspace_root)
}

pub fn build_desktop_bootstrap_with_workspace_root(
    runtime: runtime::ArgusxRuntime,
    workspace_root: PathBuf,
) -> Result<DesktopBootstrap, TurnError> {
    let provider_settings_db_path = provider_settings_db_path(runtime.config.as_ref());
    let provider_settings =
        ProviderSettingsService::from_db_path(provider_settings_db_path.clone())
            .map_err(|err| TurnError::Runtime(err.to_string()))?;
    let session_state = DesktopSessionState::from_bootstrap(
        runtime.session_manager.clone(),
        provider_settings,
        vec![workspace_root.clone()],
    )?;

    Ok(DesktopBootstrap {
        runtime,
        session_state,
        provider_settings_db_path,
        workspace_root,
    })
}

fn provider_settings_db_path(config: &runtime::AppConfig) -> PathBuf {
    config_dir_for(&config.paths.sqlite).join("desktop.sqlite3")
}

fn config_dir_for(path: &Path) -> PathBuf {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}
