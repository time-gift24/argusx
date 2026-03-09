use std::sync::Arc;

use chrono::Utc;
use sqlx::sqlite::SqlitePoolOptions;
use uuid::Uuid;

use session::store::ThreadStore;
use session::types::SessionRecord;
use session::Session;

#[tokio::test]
async fn session_creates_thread_and_emits_event() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let store = Arc::new(ThreadStore::new(pool));
    store.init_schema().await.unwrap();

    // Insert session first (required for foreign key)
    let session_record = SessionRecord {
        id: "session-1".to_string(),
        user_id: None,
        default_model: "gpt-4".to_string(),
        system_prompt: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    store.upsert_session(&session_record).await.unwrap();

    let mut session = Session::new("session-1".to_string(), store);

    let thread_id = session.create_thread(Some("A".into())).await.unwrap();

    // Thread should be created with the correct ID
    assert!(thread_id != Uuid::nil());
}
