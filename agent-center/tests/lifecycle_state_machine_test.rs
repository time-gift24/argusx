#[test]
fn rejects_terminal_state_regression() {
    use agent_center::core::lifecycle::{ThreadStateMachine, ThreadStatus, LifecycleError};

    let mut sm = ThreadStateMachine::new(ThreadStatus::Running);
    sm.transition_to(ThreadStatus::Succeeded).unwrap();
    let err = sm.transition_to(ThreadStatus::Running).unwrap_err();
    assert!(matches!(err, LifecycleError::IllegalTransition { from: ThreadStatus::Succeeded, to: ThreadStatus::Running }));
}

#[test]
fn allows_pending_to_failed_for_early_failures() {
    use agent_center::core::lifecycle::{ThreadStateMachine, ThreadStatus};

    let mut sm = ThreadStateMachine::new(ThreadStatus::Pending);
    assert!(sm.transition_to(ThreadStatus::Failed).is_ok());
    assert_eq!(sm.status(), ThreadStatus::Failed);
}

#[test]
fn idempotent_terminal_transitions() {
    use agent_center::core::lifecycle::{ThreadStateMachine, ThreadStatus};

    // Succeeded -> Succeeded
    let mut sm = ThreadStateMachine::new(ThreadStatus::Succeeded);
    assert!(sm.transition_to(ThreadStatus::Succeeded).is_ok());

    // Failed -> Failed
    let mut sm = ThreadStateMachine::new(ThreadStatus::Failed);
    assert!(sm.transition_to(ThreadStatus::Failed).is_ok());

    // Closed -> Closed
    let mut sm = ThreadStateMachine::new(ThreadStatus::Closed);
    assert!(sm.transition_to(ThreadStatus::Closed).is_ok());

    // Cancelled -> Cancelled
    let mut sm = ThreadStateMachine::new(ThreadStatus::Cancelled);
    assert!(sm.transition_to(ThreadStatus::Cancelled).is_ok());
}

#[test]
fn idempotent_running_transition() {
    use agent_center::core::lifecycle::{ThreadStateMachine, ThreadStatus};

    let mut sm = ThreadStateMachine::new(ThreadStatus::Running);
    assert!(sm.transition_to(ThreadStatus::Running).is_ok());
}
