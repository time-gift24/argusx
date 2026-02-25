use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use prompt_lab_core::{
    ChecklistFilter, ChecklistItem, PromptLab, Sop, SopStage, SopStep, UpdateSopStepInput,
};

pub struct AppState {
    pub prompt_lab: PromptLab,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub color: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub status: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SopData {
    pub sop: Sop,
    pub detect_stages: Vec<SopStage>,
    pub handle_stages: Vec<SopStage>,
    pub verification_stages: Vec<SopStage>,
    pub rollback_stages: Vec<SopStage>,
}

impl From<prompt_lab_core::SopAggregate> for SopData {
    fn from(agg: prompt_lab_core::SopAggregate) -> Self {
        Self {
            sop: agg.sop,
            detect_stages: vec![SopStage {
                name: "detect".to_string(),
                steps: agg
                    .detect_steps
                    .iter()
                    .map(|s| prompt_lab_core::SopStepRef {
                        sop_step_id: s.id,
                        name: s.name.clone(),
                    })
                    .collect(),
            }],
            handle_stages: vec![SopStage {
                name: "handle".to_string(),
                steps: agg
                    .handle_steps
                    .iter()
                    .map(|s| prompt_lab_core::SopStepRef {
                        sop_step_id: s.id,
                        name: s.name.clone(),
                    })
                    .collect(),
            }],
            verification_stages: vec![SopStage {
                name: "verification".to_string(),
                steps: agg
                    .verification_steps
                    .iter()
                    .map(|s| prompt_lab_core::SopStepRef {
                        sop_step_id: s.id,
                        name: s.name.clone(),
                    })
                    .collect(),
            }],
            rollback_stages: vec![SopStage {
                name: "rollback".to_string(),
                steps: agg
                    .rollback_steps
                    .iter()
                    .map(|s| prompt_lab_core::SopStepRef {
                        sop_step_id: s.id,
                        name: s.name.clone(),
                    })
                    .collect(),
            }],
        }
    }
}

// SOP Commands

#[tauri::command]
async fn get_sop_with_steps(
    state: State<'_, Arc<AppState>>,
    sop_id: String,
) -> Result<SopData, String> {
    let agg = state
        .prompt_lab
        .sop_service()
        .get_sop_aggregate_by_sop_id(&sop_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(SopData::from(agg))
}

#[tauri::command]
async fn get_checklist_items_by_step(
    state: State<'_, Arc<AppState>>,
    step_id: i64,
) -> Result<Vec<ChecklistItem>, String> {
    let items = state
        .prompt_lab
        .checklist_service()
        .list(ChecklistFilter {
            status: None,
            context_type: None,
            sop_step_id: Some(step_id.to_string()),
        })
        .await
        .map_err(|e| e.to_string())?;
    Ok(items)
}

#[tauri::command]
async fn update_sop_step(
    state: State<'_, Arc<AppState>>,
    id: i64,
    operation: Option<String>,
    verification: Option<String>,
    impact_analysis: Option<String>,
    rollback: Option<String>,
) -> Result<SopStep, String> {
    let step = state
        .prompt_lab
        .sop_service()
        .update_sop_step(UpdateSopStepInput {
            id,
            name: None,
            version: None,
            operation,
            verification,
            impact_analysis,
            rollback,
        })
        .await
        .map_err(|e| e.to_string())?;
    Ok(step)
}

#[tauri::command]
async fn create_chat_session(title: Option<String>) -> Result<ChatSession, String> {
    let now = chrono::Utc::now().timestamp_millis();
    Ok(ChatSession {
        id: format!("s-{}", uuid::Uuid::new_v4()),
        title: title.unwrap_or_else(|| "New Chat".to_string()),
        color: "blue".to_string(),
        created_at: now,
        updated_at: now,
        status: "active".to_string(),
    })
}

#[tauri::command]
async fn list_chat_sessions() -> Result<Vec<ChatSession>, String> {
    Ok(vec![])
}

#[tauri::command]
async fn delete_chat_session(_id: String) -> Result<(), String> {
    Ok(())
}

fn get_default_db_path() -> String {
    let default_dir = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("prompt_lab")
        .join("dev.db");
    default_dir.to_string_lossy().to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize PromptLab
    let settings = argusx_common::config::Settings {
        database: argusx_common::config::DatabaseConfig {
            path: get_default_db_path(),
            busy_timeout_ms: 5_000,
            max_connections: 5,
        },
        logging: argusx_common::config::LoggingConfig::default(),
    };

    let runtime = tokio::runtime::Runtime::new()?;
    let prompt_lab = runtime.block_on(PromptLab::new(settings))?;

    let app_state = Arc::new(AppState { prompt_lab });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            create_chat_session,
            list_chat_sessions,
            delete_chat_session,
            get_sop_with_steps,
            get_checklist_items_by_step,
            update_sop_step,
        ])
        .run(tauri::generate_context!())?;
    Ok(())
}
