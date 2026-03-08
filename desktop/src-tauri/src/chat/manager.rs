use std::collections::HashMap;

use uuid::Uuid;

#[derive(Default)]
pub struct TurnManager {
    thread_ids: tokio::sync::Mutex<HashMap<String, Uuid>>,
}

impl TurnManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn insert(&self, turn_id: String, thread_id: Uuid) {
        self.thread_ids.lock().await.insert(turn_id, thread_id);
    }

    pub async fn get(&self, turn_id: &str) -> Option<Uuid> {
        self.thread_ids.lock().await.get(turn_id).copied()
    }

    pub async fn take(&self, turn_id: &str) -> Option<Uuid> {
        self.thread_ids.lock().await.remove(turn_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn turn_manager_inserts_and_takes_thread_id_once() {
        let manager = TurnManager::new();
        let thread_id = Uuid::new_v4();

        manager.insert("turn-1".into(), thread_id).await;

        assert!(manager.take("turn-1").await.is_some());
        assert!(manager.take("turn-1").await.is_none());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn turn_manager_peek_returns_thread_id_without_removing_it() {
        let manager = TurnManager::new();
        let thread_id = Uuid::new_v4();

        manager.insert("turn-1".into(), thread_id).await;

        assert_eq!(manager.get("turn-1").await, Some(thread_id));
        assert_eq!(manager.get("turn-1").await, Some(thread_id));
        assert_eq!(manager.take("turn-1").await, Some(thread_id));
        assert!(manager.get("turn-1").await.is_none());
    }
}
