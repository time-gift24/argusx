#[test]
fn rejects_terminal_state_regression() {
    use agent_center::core::lifecycle::{ThreadStateMachine, ThreadStatus};

    let mut sm = ThreadStateMachine::new(ThreadStatus::Running);
    sm.transition_to(ThreadStatus::Succeeded).unwrap();
    assert!(sm.transition_to(ThreadStatus::Running).is_err());
}
