use tauri::State;

use crate::{chat::AppState, provider_settings::{ProviderConnectionResult, ProviderProfileSummary, SaveProviderProfileInput, TestProviderProfileInput}};

#[tauri::command]
pub fn list_provider_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<ProviderProfileSummary>, String> {
    state.provider_settings().list_profiles().map_err(stringify)
}

#[tauri::command]
pub fn save_provider_profile(
    state: State<'_, AppState>,
    input: SaveProviderProfileInput,
) -> Result<ProviderProfileSummary, String> {
    state.provider_settings().save_profile(input).map_err(stringify)
}

#[tauri::command]
pub fn delete_provider_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<(), String> {
    state
        .provider_settings()
        .delete_profile(&profile_id)
        .map_err(stringify)
}

#[tauri::command]
pub fn set_default_provider_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<ProviderProfileSummary, String> {
    state
        .provider_settings()
        .set_default_profile(&profile_id)
        .map_err(stringify)
}

#[tauri::command]
pub async fn test_provider_profile(
    state: State<'_, AppState>,
    input: TestProviderProfileInput,
) -> Result<ProviderConnectionResult, String> {
    state
        .provider_settings()
        .test_profile(input)
        .await
        .map_err(stringify)
}

fn stringify(err: impl std::fmt::Display) -> String {
    err.to_string()
}
