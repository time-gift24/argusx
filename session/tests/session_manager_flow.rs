use session::{manager::SessionManager, store::ThreadStore};
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
