use chrono::Utc;
use session::{
    manager::SessionManager,
    store::ThreadStore,
    types::{
        PersistedMessage, SessionRecord, ThreadLifecycle, ThreadRecord, TurnRecord, TurnStatus,
    },
};
use sqlx::sqlite::SqlitePoolOptions;
use uuid::Uuid;

#[tokio::test]
async fn manager_marks_incomplete_turns_interrupted_on_startup() {
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
        default_model: "gpt-5".into(),
        system_prompt: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    store.upsert_session(&session).await.unwrap();

    let thread_id = Uuid::new_v4();
    store
        .insert_thread(&ThreadRecord {
            id: thread_id,
            session_id: session.id.clone(),
            title: Some("A".into()),
            lifecycle: ThreadLifecycle::Open,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_turn_number: 1,
        })
        .await
        .unwrap();

    store
        .insert_turn(&TurnRecord {
            id: Uuid::new_v4(),
            thread_id,
            turn_number: 1,
            user_input: "hello".into(),
            status: TurnStatus::Running,
            finish_reason: None,
            transcript: vec![PersistedMessage::User {
                content: "hello".into(),
            }],
            final_output: None,
            started_at: Utc::now(),
            finished_at: None,
        })
        .await
        .unwrap();

    let manager = SessionManager::new(session.id.clone(), store.clone());
    let interrupted = manager.initialize().await.unwrap();

    assert_eq!(interrupted, 1);
    assert_eq!(manager.active_thread_id(), None);

    let turns = manager.load_thread_history(thread_id).await.unwrap();
    assert!(matches!(turns[0].status, TurnStatus::Interrupted));
}
