use chrono::Utc;
use session::{
    SessionRecord, SubagentDispatchRecord, SubagentDispatchStatus, ThreadAgentSnapshotRecord,
    ThreadLifecycle, ThreadRecord, store::ThreadStore,
};
use sqlx::sqlite::SqlitePoolOptions;
use uuid::Uuid;

#[tokio::test(flavor = "current_thread")]
async fn store_round_trips_thread_agent_snapshot_and_dispatch_record() {
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
    let child_thread_id = Uuid::new_v4();
    for thread_id in [thread_id, child_thread_id] {
        store
            .insert_thread(&ThreadRecord {
                id: thread_id,
                session_id: session.id.clone(),
                agent_profile_id: None,
                is_subagent: false,
                title: Some("Agent".into()),
                lifecycle: ThreadLifecycle::Open,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                last_turn_number: 0,
            })
            .await
            .unwrap();
    }

    let parent_turn_id = Uuid::new_v4();

    store
        .insert_thread_agent_snapshot(&ThreadAgentSnapshotRecord {
            thread_id,
            profile_id: "reviewer".into(),
            display_name_snapshot: "Reviewer".into(),
            system_prompt_snapshot: "You are a reviewer.".into(),
            tool_policy_snapshot_json: serde_json::json!({"builtins": ["read"]}),
            model_config_snapshot_json: serde_json::Value::Null,
            allow_subagent_dispatch_snapshot: false,
            created_at: Utc::now(),
        })
        .await
        .unwrap();

    let dispatch = SubagentDispatchRecord {
        id: Uuid::new_v4(),
        parent_thread_id: thread_id,
        parent_turn_id,
        dispatch_tool_call_id: "call-1".into(),
        child_thread_id,
        child_agent_profile_id: "reviewer".into(),
        status: SubagentDispatchStatus::Running,
        requested_at: Utc::now(),
        finished_at: None,
        result_summary: None,
    };
    store.insert_subagent_dispatch(&dispatch).await.unwrap();

    let snapshot = store.get_thread_agent_snapshot(thread_id).await.unwrap().unwrap();
    assert_eq!(snapshot.profile_id, "reviewer");

    let dispatches = store.list_subagent_dispatches(thread_id).await.unwrap();
    assert_eq!(dispatches, vec![dispatch]);
}
