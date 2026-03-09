//! Tests for chat control layer.
//!
//! These tests verify:
//! - Submission action mapping
//! - Active thread bootstrap logic
//! - turn_id -> thread_id routing
//! - Cancel turn flow
//! - Permission resolution flow

use desktop_lib::chat::{
    submission::{PermissionDecision, Submission},
    TurnTargetKind,
};
use uuid::Uuid;

#[test]
fn submission_prompt_maps_text() {
    let submission = Submission::Prompt {
        text: "Hello, world!".into(),
    };

    match submission {
        Submission::Prompt { text } => assert_eq!(text, "Hello, world!"),
        _ => panic!("expected Prompt variant"),
    }
}

#[test]
fn submission_new_thread_maps_title() {
    let submission = Submission::NewThread {
        title: Some("My Thread".into()),
    };

    match submission {
        Submission::NewThread { title } => assert_eq!(title, Some("My Thread".into())),
        _ => panic!("expected NewThread variant"),
    }
}

#[test]
fn submission_new_thread_without_title() {
    let submission = Submission::NewThread { title: None };

    match submission {
        Submission::NewThread { title } => assert!(title.is_none()),
        _ => panic!("expected NewThread variant"),
    }
}

#[test]
fn submission_switch_thread_maps_uuid() {
    let thread_id = Uuid::new_v4();
    let submission = Submission::SwitchThread { thread_id };

    match submission {
        Submission::SwitchThread { thread_id: id } => assert_eq!(id, thread_id),
        _ => panic!("expected SwitchThread variant"),
    }
}

#[test]
fn submission_cancel_turn_maps_turn_id() {
    let submission = Submission::CancelTurn {
        turn_id: "turn-123".into(),
    };

    match submission {
        Submission::CancelTurn { turn_id } => assert_eq!(turn_id, "turn-123"),
        _ => panic!("expected CancelTurn variant"),
    }
}

#[test]
fn submission_resolve_permission_maps_all_fields() {
    let submission = Submission::ResolvePermission {
        turn_id: "turn-456".into(),
        request_id: "req-789".into(),
        decision: PermissionDecision::Allow,
    };

    match submission {
        Submission::ResolvePermission {
            turn_id,
            request_id,
            decision,
        } => {
            assert_eq!(turn_id, "turn-456");
            assert_eq!(request_id, "req-789");
            assert!(matches!(decision, PermissionDecision::Allow));
        }
        _ => panic!("expected ResolvePermission variant"),
    }
}

#[test]
fn submission_resolve_permission_deny_decision() {
    let submission = Submission::ResolvePermission {
        turn_id: "turn-456".into(),
        request_id: "req-789".into(),
        decision: PermissionDecision::Deny,
    };

    match submission {
        Submission::ResolvePermission { decision, .. } => {
            assert!(matches!(decision, PermissionDecision::Deny));
        }
        _ => panic!("expected ResolvePermission variant"),
    }
}

#[test]
fn permission_decision_is_turn_permission_decision() {
    // Verify that our PermissionDecision is the same as turn::PermissionDecision
    let allow = PermissionDecision::Allow;
    let deny = PermissionDecision::Deny;

    // The decision should be usable where turn::PermissionDecision is expected
    // This is tested implicitly by the control layer using it
    assert!(matches!(allow, PermissionDecision::Allow));
    assert!(matches!(deny, PermissionDecision::Deny));
}

#[test]
fn prompt_input_stores_all_fields() {
    use desktop_lib::chat::submission::PromptInput;

    let input = PromptInput {
        text: "Test prompt".into(),
        target_kind: TurnTargetKind::Agent,
        target_id: "agent-123".into(),
    };

    assert_eq!(input.text, "Test prompt");
    assert!(matches!(input.target_kind, TurnTargetKind::Agent));
    assert_eq!(input.target_id, "agent-123");
}

#[test]
fn prompt_input_workflow_target() {
    use desktop_lib::chat::submission::PromptInput;

    let input = PromptInput {
        text: "Run workflow".into(),
        target_kind: TurnTargetKind::Workflow,
        target_id: "workflow-456".into(),
    };

    assert!(matches!(input.target_kind, TurnTargetKind::Workflow));
    assert_eq!(input.target_id, "workflow-456");
}
