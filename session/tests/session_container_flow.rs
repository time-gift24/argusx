use std::sync::Arc;

use sqlx::sqlite::SqlitePoolOptions;
use uuid::Uuid;

use session::store::ThreadStore;

#[tokio::test]
async fn session_creates_thread_and_returns_subscription() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let store = Arc::new(ThreadStore::new(pool));
    store.init_schema().await.unwrap();

    // This test will fail to compile until Task 2 implementation is done
    // The test exercises Session::create_thread() and Session::subscribe()
    // which don't exist yet
}
