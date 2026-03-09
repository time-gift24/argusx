use chrono::Utc;
use session::{SessionRecord, manager::SessionManager, store::ThreadStore};
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn create_thread_switch_thread_and_list_history() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let store = ThreadStore::new(pool);
    store.init_schema().await.unwrap();

    let manager = SessionManager::new("session-1".into(), store);
    let first = manager.create_thread(Some("A".into())).await.unwrap();
    let second = manager.create_thread(Some("B".into())).await.unwrap();

    manager.switch_thread(first).await.unwrap();
    assert_eq!(manager.active_thread_id(), Some(first));

    let threads = manager.list_threads().await.unwrap();
    assert_eq!(threads.len(), 2);
    assert!(threads.iter().any(|thread| thread.id == second));
}

#[tokio::test]
async fn create_thread_preserves_existing_session_defaults() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let store = ThreadStore::new(pool);
    store.init_schema().await.unwrap();

    let session = SessionRecord {
        id: "session-1".into(),
        user_id: None,
        default_model: "custom-model".into(),
        system_prompt: Some("Session base prompt".into()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    store.upsert_session(&session).await.unwrap();

    let manager = SessionManager::new(session.id.clone(), store.clone());
    manager.create_thread(Some("A".into())).await.unwrap();

    let loaded = store.get_session(&session.id).await.unwrap().unwrap();
    assert_eq!(loaded.default_model, "custom-model");
    assert_eq!(loaded.system_prompt.as_deref(), Some("Session base prompt"));
}
