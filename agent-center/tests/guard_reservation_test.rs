#[tokio::test]
async fn releases_slot_on_drop() {
    use agent_center::permission::guard::SpawnGuards;

    let guards = SpawnGuards::new(1, 2);
    let r1 = guards.reserve(0).unwrap();
    assert!(guards.reserve(0).is_err(), "second reservation should fail");
    drop(r1);
    assert!(
        guards.reserve(0).is_ok(),
        "reservation after drop should succeed"
    );
}

#[test]
fn returns_max_depth_exceeded_error() {
    use agent_center::permission::guard::{GuardError, SpawnGuards};

    let guards = SpawnGuards::new(10, 2);
    let err = guards.reserve(2).unwrap_err();
    assert!(matches!(
        err,
        GuardError::MaxDepthExceeded {
            parent_depth: 2,
            max_depth: 2
        }
    ));
}

#[test]
fn returns_max_concurrent_exceeded_error() {
    use agent_center::permission::guard::{GuardError, SpawnGuards};

    let guards = SpawnGuards::new(1, 10);
    let _r1 = guards.reserve(0).unwrap();
    let err = guards.reserve(0).unwrap_err();
    assert!(matches!(err, GuardError::MaxConcurrentExceeded));
}
