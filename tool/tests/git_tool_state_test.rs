use git2::{Repository, Signature};
use serde_json::json;
use std::path::Path;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;
use tool::{GitTool, Tool, ToolContext};

fn create_test_repo() -> (TempDir, Repository) {
    let temp = tempfile::tempdir().unwrap();
    let repo = Repository::init(temp.path()).unwrap();

    // Configure identity
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();

    // Create initial commit
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    let mut index = repo.index().unwrap();
    std::fs::write(temp.path().join("README.md"), b"# Test\n").unwrap();
    index.add_path(Path::new("README.md")).unwrap();

    let tree_id = index.write_tree().unwrap();
    drop(index);

    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit\n", &tree, &[])
        .unwrap();
    drop(tree);

    (temp, repo)
}

fn create_dirty_repo() -> (TempDir, Repository) {
    let (temp, repo) = create_test_repo();

    // Create a modified file (unstaged)
    std::fs::write(temp.path().join("README.md"), b"# Modified\n").unwrap();

    // Create a new untracked file
    std::fs::write(temp.path().join("new_file.txt"), b"new content\n").unwrap();

    (temp, repo)
}

fn tool_context() -> ToolContext {
    ToolContext::new("test-session", "test-turn", CancellationToken::new())
}

#[tokio::test]
async fn status_reports_clean_repo() {
    let (temp, _repo) = create_test_repo();
    let tool = GitTool::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = tool
        .execute(
            tool_context(),
            json!({
                "action": "status",
                "repo_path": temp.path().to_str().unwrap()
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let output = &result.output;
    println!(
        "CLEAN DEBUG: {}",
        serde_json::to_string_pretty(&output).unwrap()
    );
    assert_eq!(output["action"], "status");
    assert!(output["data"]["is_clean"].as_bool().unwrap());
    assert!(output["data"]["files"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn status_reports_dirty_files() {
    let (temp, _repo) = create_dirty_repo();
    let tool = GitTool::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = tool
        .execute(
            tool_context(),
            json!({
                "action": "status",
                "repo_path": temp.path().to_str().unwrap(),
                "include_untracked": true
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let output = &result.output;
    assert_eq!(output["action"], "status");
    assert!(!output["data"]["is_clean"].as_bool().unwrap());

    let files = output["data"]["files"].as_array().unwrap();
    assert!(!files.is_empty());
}

#[tokio::test]
async fn branch_list_reports_head_and_branches() {
    let (temp, _repo) = create_test_repo();
    let tool = GitTool::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = tool
        .execute(
            tool_context(),
            json!({
                "action": "branch_list",
                "repo_path": temp.path().to_str().unwrap()
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let output = &result.output;
    assert_eq!(output["action"], "branch_list");

    // Should have at least main/master branch
    let branches = output["data"]["branches"].as_array().unwrap();
    assert!(!branches.is_empty());

    // Should report HEAD
    let head = &output["data"]["head"];
    assert!(head.is_string());
}

#[tokio::test]
async fn remote_list_reports_configured_remotes() {
    let (temp, repo) = create_test_repo();

    // Add a remote
    repo.remote("origin", "https://github.com/example/repo.git")
        .unwrap();

    let tool = GitTool::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = tool
        .execute(
            tool_context(),
            json!({
                "action": "remote_list",
                "repo_path": temp.path().to_str().unwrap()
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let output = &result.output;
    assert_eq!(output["action"], "remote_list");

    let remotes = output["data"]["remotes"].as_array().unwrap();
    assert!(!remotes.is_empty());

    // Check origin is in the list
    let names: Vec<&str> = remotes
        .iter()
        .filter_map(|r| r.get("name")?.as_str())
        .collect();
    assert!(names.contains(&"origin"));
}

#[tokio::test]
async fn worktree_list_reports_main_worktree() {
    let (temp, _repo) = create_test_repo();
    let tool = GitTool::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = tool
        .execute(
            tool_context(),
            json!({
                "action": "worktree_list",
                "repo_path": temp.path().to_str().unwrap()
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let output = &result.output;
    assert_eq!(output["action"], "worktree_list");

    let worktrees = output["data"]["worktrees"].as_array().unwrap();
    assert!(!worktrees.is_empty());
}
