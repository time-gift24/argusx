use agent_core::{SessionInfo, SessionStatus, TurnStatus, TurnSummary};
use agent_session::{SessionFilter, SessionStore, SqliteSessionStore};

fn build_session(id: &str, updated_at: i64) -> SessionInfo {
    let mut info = SessionInfo::new(id.to_string(), format!("Session {id}"));
    info.status = SessionStatus::Idle;
    info.created_at = updated_at - 100;
    info.updated_at = updated_at;
    info
}

fn build_done_turn(turn_id: &str, started_at: i64, ended_at: i64) -> TurnSummary {
    TurnSummary {
        turn_id: turn_id.to_string(),
        epoch: 0,
        started_at,
        ended_at: Some(ended_at),
        status: TurnStatus::Done,
        final_message: Some("ok".to_string()),
        tool_calls_count: 0,
        input_tokens: 0,
        output_tokens: 0,
    }
}

#[tokio::test]
async fn sqlite_store_orders_by_last_ended_then_updated_at() {
    let temp = tempfile::tempdir().expect("create tempdir");
    let db_path = temp.path().join("sessions.db");
    let store = SqliteSessionStore::new(db_path).expect("init sqlite store");

    let with_newer_turn = build_session("s_with_newer_turn", 2_000);
    let with_older_turn = build_session("s_with_older_turn", 9_000);
    let without_turn = build_session("s_without_turn", 5_000);

    store
        .create(&with_newer_turn)
        .await
        .expect("create session 1");
    store
        .create(&with_older_turn)
        .await
        .expect("create session 2");
    store.create(&without_turn).await.expect("create session 3");

    store
        .save_turn_summary(
            &with_newer_turn.session_id,
            &build_done_turn("turn-1", 1_000, 30_000),
        )
        .await
        .expect("save turn summary 1");
    store
        .save_turn_summary(
            &with_older_turn.session_id,
            &build_done_turn("turn-2", 1_000, 10_000),
        )
        .await
        .expect("save turn summary 2");

    let sessions = store
        .list(SessionFilter {
            limit: Some(10),
            ..SessionFilter::default()
        })
        .await
        .expect("list sessions");

    let ids: Vec<&str> = sessions.iter().map(|s| s.session_id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["s_with_newer_turn", "s_with_older_turn", "s_without_turn"]
    );
}
