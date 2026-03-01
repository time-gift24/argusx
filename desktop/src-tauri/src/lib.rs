use serde::{Deserialize, Serialize};

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            create_chat_session,
            list_chat_sessions,
            delete_chat_session,
        ])
        .run(tauri::generate_context!())?;
    Ok(())
}
