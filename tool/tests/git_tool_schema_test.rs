use serde_json::json;
use tool::{GitTool, Tool};

#[test]
fn git_args_status_parses_correctly() {
    let args: tool::builtin::git::types::GitArgs = serde_json::from_value(json!({
        "action": "status",
        "repo_path": "/tmp/repo"
    }))
    .unwrap();

    match args {
        tool::builtin::git::types::GitArgs::Status {
            repo_path,
            include_untracked,
        } => {
            assert_eq!(repo_path, "/tmp/repo");
            assert!(!include_untracked); // default false
        }
        _ => panic!("expected Status variant"),
    }
}

#[test]
fn git_args_commit_requires_message() {
    // Missing message should fail
    let result = serde_json::from_value::<tool::builtin::git::types::GitArgs>(json!({
        "action": "commit",
        "repo_path": "/tmp/repo"
    }));
    assert!(result.is_err(), "commit without message should fail");
}

#[test]
fn git_args_commit_with_message_parses() {
    let args: tool::builtin::git::types::GitArgs = serde_json::from_value(json!({
        "action": "commit",
        "repo_path": "/tmp/repo",
        "message": "Initial commit"
    }))
    .unwrap();

    match args {
        tool::builtin::git::types::GitArgs::Commit {
            repo_path,
            message,
            allow_empty,
        } => {
            assert_eq!(repo_path, "/tmp/repo");
            assert_eq!(message, "Initial commit");
            assert!(!allow_empty);
        }
        _ => panic!("expected Commit variant"),
    }
}

#[test]
fn git_tool_schema_has_one_of_with_all_actions() {
    let temp = tempfile::tempdir().unwrap();
    let tool = GitTool::new(vec![temp.path().to_path_buf()]).unwrap();
    let schema = tool.spec().input_schema;

    let one_of = schema.get("oneOf").expect("schema should have oneOf");
    let actions = one_of.as_array().expect("oneOf should be an array");

    // Check that we have all expected actions
    let action_names: Vec<&str> = actions
        .iter()
        .filter_map(|item| {
            item.get("properties")?
                .get("action")?
                .get("const")?
                .as_str()
        })
        .collect();

    assert!(action_names.contains(&"status"), "missing status action");
    assert!(action_names.contains(&"diff"), "missing diff action");
    assert!(action_names.contains(&"log"), "missing log action");
    assert!(action_names.contains(&"show"), "missing show action");
    assert!(
        action_names.contains(&"branch_list"),
        "missing branch_list action"
    );
    assert!(
        action_names.contains(&"remote_list"),
        "missing remote_list action"
    );
    assert!(
        action_names.contains(&"worktree_list"),
        "missing worktree_list action"
    );
    assert!(action_names.contains(&"add"), "missing add action");
    assert!(action_names.contains(&"commit"), "missing commit action");
    assert!(
        action_names.contains(&"branch_create"),
        "missing branch_create action"
    );
    assert!(
        action_names.contains(&"checkout"),
        "missing checkout action"
    );
    assert!(action_names.contains(&"clone"), "missing clone action");
    assert!(action_names.contains(&"fetch"), "missing fetch action");
}

#[test]
fn git_tool_schema_commit_action_has_required_fields() {
    let temp = tempfile::tempdir().unwrap();
    let tool = GitTool::new(vec![temp.path().to_path_buf()]).unwrap();
    let schema = tool.spec().input_schema;

    let one_of = schema.get("oneOf").expect("schema should have oneOf");
    let actions = one_of.as_array().expect("oneOf should be an array");

    let commit_action = actions
        .iter()
        .find(|item| {
            item.get("properties")
                .and_then(|p| p.get("action"))
                .and_then(|a| a.get("const"))
                .and_then(|c| c.as_str())
                == Some("commit")
        })
        .expect("should find commit action");

    let required = commit_action
        .get("required")
        .expect("commit should have required fields");
    let required_arr = required.as_array().expect("required should be an array");

    assert!(
        required_arr.contains(&json!("action")),
        "action should be required"
    );
    assert!(
        required_arr.contains(&json!("repo_path")),
        "repo_path should be required"
    );
    assert!(
        required_arr.contains(&json!("message")),
        "message should be required"
    );
}
