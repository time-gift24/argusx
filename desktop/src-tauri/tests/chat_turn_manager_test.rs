//! Tests for turn_id -> thread_id routing via TurnManager.

use desktop_lib::chat::TurnManager;
use uuid::Uuid;

#[tokio::test]
async fn turn_manager_insert_and_get_thread_id() {
    let manager = TurnManager::new();
    let thread_id = Uuid::new_v4();

    manager.insert("turn-1".into(), thread_id).await;

    let found = manager.get("turn-1").await;
    assert_eq!(found, Some(thread_id));
}

#[tokio::test]
async fn turn_manager_returns_none_for_unknown_turn() {
    let manager = TurnManager::new();

    let found = manager.get("unknown-turn").await;
    assert!(found.is_none());
}

#[tokio::test]
async fn turn_manager_take_removes_entry() {
    let manager = TurnManager::new();
    let thread_id = Uuid::new_v4();

    manager.insert("turn-1".into(), thread_id).await;

    let taken = manager.take("turn-1").await;
    assert_eq!(taken, Some(thread_id));

    // After taking, get should return None
    let found = manager.get("turn-1").await;
    assert!(found.is_none());
}

#[tokio::test]
async fn turn_manager_multiple_turns() {
    let manager = TurnManager::new();
    let thread_id_1 = Uuid::new_v4();
    let thread_id_2 = Uuid::new_v4();

    manager.insert("turn-1".into(), thread_id_1).await;
    manager.insert("turn-2".into(), thread_id_2).await;

    assert_eq!(manager.get("turn-1").await, Some(thread_id_1));
    assert_eq!(manager.get("turn-2").await, Some(thread_id_2));
}

#[tokio::test]
async fn turn_manager_overwrites_existing_turn_id() {
    let manager = TurnManager::new();
    let thread_id_1 = Uuid::new_v4();
    let thread_id_2 = Uuid::new_v4();

    manager.insert("turn-1".into(), thread_id_1).await;
    manager.insert("turn-1".into(), thread_id_2).await;

    // Should return the latest thread_id
    assert_eq!(manager.get("turn-1").await, Some(thread_id_2));
}
