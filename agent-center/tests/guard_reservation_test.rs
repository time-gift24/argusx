#[tokio::test]
async fn releases_slot_on_drop() {
    use agent_center::permission::guard::SpawnGuards;

    let guards = SpawnGuards::new(1, 2);
    let r1 = guards.reserve(0).unwrap();
    assert!(guards.reserve(0).is_err(), "second reservation should fail");
    drop(r1);
    assert!(guards.reserve(0).is_ok(), "reservation after drop should succeed");
}
