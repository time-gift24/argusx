use std::sync::Arc;

use chrono::Utc;
use sqlx::sqlite::SqlitePoolOptions;
use uuid::Uuid;

use session::database::DynSessionDatabase;
use session::store::ThreadStore;
use session::types::{SessionRecord, ThreadEvent, ThreadLifecycle, ThreadRecord};
use session::{Session, SessionError};

#[tokio::test]
async fn session_emits_thread_activated_event_on_switch() {
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

    let db: DynSessionDatabase = store.as_database();
    let mut session = Session::new("session-1".to_string(), db);

    // Create thread first
    let thread_id = session.create_thread(Some("A".into())).await.unwrap();

    // Subscribe after creation to capture FUTURE events
    let mut rx = session.subscribe(thread_id).await.unwrap();

    // Switch to the same thread - this should emit ThreadActivated
    session.switch_thread(thread_id).await.unwrap();

    // Should receive ThreadActivated event
    let event = rx.recv().await.unwrap();
    assert_eq!(event.event, ThreadEvent::ThreadActivated);
    assert_eq!(event.thread_id, thread_id);
}

#[tokio::test]
async fn session_thread_scoped_subscription() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let store = Arc::new(ThreadStore::new(pool));
    store.init_schema().await.unwrap();

    let session_record = SessionRecord {
        id: "session-1".to_string(),
        user_id: None,
        default_model: "gpt-4".to_string(),
        system_prompt: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    store.upsert_session(&session_record).await.unwrap();

    let db: DynSessionDatabase = store.as_database();
    let mut session = Session::new("session-1".to_string(), db);

    let thread_a = session.create_thread(Some("A".into())).await.unwrap();
    let thread_b = session.create_thread(Some("B".into())).await.unwrap();

    // Subscribe to thread A
    let mut rx_a = session.subscribe(thread_a).await.unwrap();

    // Subscribe to thread B to verify it receives its own events
    let mut rx_b = session.subscribe(thread_b).await.unwrap();

    // Switch to thread B - should emit ThreadActivated on thread B only
    session.switch_thread(thread_b).await.unwrap();

    // Thread B should receive ThreadActivated
    let event_b = rx_b.recv().await.unwrap();
    assert_eq!(event_b.event, ThreadEvent::ThreadActivated);
    assert_eq!(event_b.thread_id, thread_b);

    // Thread A should NOT receive thread B's activation (thread-scoped)
    // The recv should timeout/err since no event was sent to thread A's channel
    // Use try_recv to avoid hanging forever
    assert!(rx_a.try_recv().is_err());
}

#[tokio::test]
async fn session_subscribe_lazy_loads_from_database() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let store = Arc::new(ThreadStore::new(pool));
    store.init_schema().await.unwrap();

    let session_record = SessionRecord {
        id: "session-1".to_string(),
        user_id: None,
        default_model: "gpt-4".to_string(),
        system_prompt: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    store.upsert_session(&session_record).await.unwrap();

    // Create a thread and persist it
    let thread_record = session::types::ThreadRecord {
        id: Uuid::new_v4(),
        session_id: "session-1".to_string(),
        title: Some("Persisted".into()),
        lifecycle: session::types::ThreadLifecycle::Open,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_turn_number: 0,
    };
    store.insert_thread(&thread_record).await.unwrap();

    let db: DynSessionDatabase = store.as_database();
    let mut session = Session::new("session-1".to_string(), db);

    // Subscribe without first loading the thread - should lazy-load from DB
    let rx = session.subscribe(thread_record.id).await.unwrap();

    // Should be able to receive events (the channel is alive)
    // Even though no events have been emitted yet, the subscription should work
    drop(rx);
    drop(session);
}

#[tokio::test]
async fn session_get_thread_returns_thread_aggregate() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let store = Arc::new(ThreadStore::new(pool));
    store.init_schema().await.unwrap();

    let session_record = SessionRecord {
        id: "session-1".to_string(),
        user_id: None,
        default_model: "gpt-4".to_string(),
        system_prompt: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    store.upsert_session(&session_record).await.unwrap();

    let db: DynSessionDatabase = store.as_database();
    let mut session = Session::new("session-1".to_string(), db);

    // Create a thread
    let thread_id = session
        .create_thread(Some("Test Thread".into()))
        .await
        .unwrap();

    // get_thread should return a Thread aggregate (sync, returns Arc<Thread>)
    let thread = session.get_thread(thread_id).unwrap().unwrap();

    assert_eq!(thread.id(), thread_id);
    assert_eq!(thread.record().title.as_deref(), Some("Test Thread"));

    // Thread aggregate should allow subscribing to events
    let mut rx = thread.subscribe();
    drop(thread);

    // Switch to emit an event
    session.switch_thread(thread_id).await.unwrap();

    // Should receive the event through the aggregate's subscription
    let event = rx.recv().await.unwrap();
    assert_eq!(event.event, ThreadEvent::ThreadActivated);
}

#[tokio::test]
async fn thread_emit_uses_its_own_thread_id_in_envelope() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let store = Arc::new(ThreadStore::new(pool));
    store.init_schema().await.unwrap();

    let session_record = SessionRecord {
        id: "session-1".to_string(),
        user_id: None,
        default_model: "gpt-4".to_string(),
        system_prompt: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    store.upsert_session(&session_record).await.unwrap();

    let db: DynSessionDatabase = store.as_database();
    let mut session = Session::new("session-1".to_string(), db);
    let thread_id = session.create_thread(Some("Owned".into())).await.unwrap();
    let thread = session.get_thread(thread_id).unwrap().unwrap();
    let mut rx = thread.subscribe();
    let turn_id = Uuid::new_v4();

    thread.emit(ThreadEvent::ThreadActivated, Some(turn_id));

    let event = rx.recv().await.unwrap();
    assert_eq!(event.thread_id, thread_id);
    assert_eq!(event.turn_id, Some(turn_id));
    assert_eq!(event.event, ThreadEvent::ThreadActivated);
}

#[tokio::test]
async fn session_load_thread_rejects_threads_from_other_sessions() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let store = Arc::new(ThreadStore::new(pool));
    store.init_schema().await.unwrap();

    let session_one = SessionRecord {
        id: "session-1".to_string(),
        user_id: None,
        default_model: "gpt-4".to_string(),
        system_prompt: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let session_two = SessionRecord {
        id: "session-2".to_string(),
        user_id: None,
        default_model: "gpt-4".to_string(),
        system_prompt: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    store.upsert_session(&session_one).await.unwrap();
    store.upsert_session(&session_two).await.unwrap();

    let foreign_thread = ThreadRecord {
        id: Uuid::new_v4(),
        session_id: "session-2".to_string(),
        title: Some("Foreign".into()),
        lifecycle: ThreadLifecycle::Open,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_turn_number: 0,
    };
    store.insert_thread(&foreign_thread).await.unwrap();

    let db: DynSessionDatabase = store.as_database();
    let mut session = Session::new("session-1".to_string(), db);

    let loaded = session.load_thread(foreign_thread.id).await.unwrap();
    assert!(loaded.is_none());
}

#[tokio::test]
async fn session_switch_thread_rejects_threads_from_other_sessions() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let store = Arc::new(ThreadStore::new(pool));
    store.init_schema().await.unwrap();

    let session_one = SessionRecord {
        id: "session-1".to_string(),
        user_id: None,
        default_model: "gpt-4".to_string(),
        system_prompt: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let session_two = SessionRecord {
        id: "session-2".to_string(),
        user_id: None,
        default_model: "gpt-4".to_string(),
        system_prompt: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    store.upsert_session(&session_one).await.unwrap();
    store.upsert_session(&session_two).await.unwrap();

    let foreign_thread = ThreadRecord {
        id: Uuid::new_v4(),
        session_id: "session-2".to_string(),
        title: Some("Foreign".into()),
        lifecycle: ThreadLifecycle::Open,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_turn_number: 0,
    };
    store.insert_thread(&foreign_thread).await.unwrap();

    let db: DynSessionDatabase = store.as_database();
    let mut session = Session::new("session-1".to_string(), db);

    let error = session.switch_thread(foreign_thread.id).await.unwrap_err();
    assert!(matches!(error, SessionError::ThreadNotFound(id) if id == foreign_thread.id));
}

#[tokio::test]
async fn session_subscribe_rejects_threads_from_other_sessions() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let store = Arc::new(ThreadStore::new(pool));
    store.init_schema().await.unwrap();

    let session_one = SessionRecord {
        id: "session-1".to_string(),
        user_id: None,
        default_model: "gpt-4".to_string(),
        system_prompt: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let session_two = SessionRecord {
        id: "session-2".to_string(),
        user_id: None,
        default_model: "gpt-4".to_string(),
        system_prompt: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    store.upsert_session(&session_one).await.unwrap();
    store.upsert_session(&session_two).await.unwrap();

    let foreign_thread = ThreadRecord {
        id: Uuid::new_v4(),
        session_id: "session-2".to_string(),
        title: Some("Foreign".into()),
        lifecycle: ThreadLifecycle::Open,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_turn_number: 0,
    };
    store.insert_thread(&foreign_thread).await.unwrap();

    let db: DynSessionDatabase = store.as_database();
    let mut session = Session::new("session-1".to_string(), db);

    let error = session.subscribe(foreign_thread.id).await.unwrap_err();
    assert!(matches!(error, SessionError::ThreadNotFound(id) if id == foreign_thread.id));
}
