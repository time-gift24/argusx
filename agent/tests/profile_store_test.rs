#[tokio::test(flavor = "current_thread")]
async fn store_seeds_builtin_main_agent_and_round_trips_custom_profile() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let store = agent::AgentProfileStore::new(pool);

    store.init_schema().await.unwrap();
    store.seed_builtin_profiles().await.unwrap();

    let builtin = store.get_profile("builtin-main").await.unwrap().unwrap();
    assert!(matches!(builtin.kind, agent::AgentProfileKind::BuiltinMain));
    assert!(!builtin.allow_subagent_dispatch);

    let custom = agent::AgentProfileRecord::custom(
        "reviewer",
        "Reviewer",
        "Review code for regressions",
        "You are a strict reviewer.",
        serde_json::json!({"builtins": ["read", "grep"]}),
    );

    store.upsert_profile(&custom).await.unwrap();

    let loaded = store.get_profile("reviewer").await.unwrap().unwrap();
    assert_eq!(loaded.display_name, "Reviewer");
    assert!(!loaded.allow_subagent_dispatch);
}
