use git2::{Repository, Signature};
use serde_json::json;
use std::path::Path;
use tempfile::TempDir;
use tool::{GitTool, Tool, ToolContext};
use tokio_util::sync::CancellationToken;

fn create_repo_for_writes() -> (TempDir, Repository) {
    let temp = tempfile::tempdir().unwrap();
    let repo = Repository::init(&temp.path()).unwrap();

    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    drop(config);

    // Create initial commit
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    std::fs::write(temp.path().join("initial.txt"), b"initial\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("initial.txt")).unwrap();
    let tree_id = index.write_tree().unwrap();
    drop(index);

    let tree = repo.find_tree(tree_id).unwrap();
    let _commit = repo.commit(Some("HEAD"), &sig, &sig, "Initial commit\n", &tree, &[]).unwrap();
    drop(tree);

    (temp, repo)
}

fn tool_context() -> ToolContext {
    ToolContext::new("test-session", "test-turn", CancellationToken::new())
}

#[tokio::test]
async fn add_stages_new_file() {
    let (temp, _repo) = create_repo_for_writes();

    // Create a new file
    std::fs::write(temp.path().join("new_file.txt"), b"new content\n").unwrap();

    let tool = GitTool::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = tool
        .execute(
            tool_context(),
            json!({
                "action": "add",
                "repo_path": temp.path().to_str().unwrap(),
                "paths": ["new_file.txt"]
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let staged = result.output["data"]["staged_paths"].as_array().unwrap();
    assert!(staged.contains(&json!("new_file.txt")));
}

#[tokio::test]
async fn commit_creates_commit_with_message() {
    let (temp, repo) = create_repo_for_writes();

    // Stage a file
    std::fs::write(temp.path().join("new_file.txt"), b"new content\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("new_file.txt")).unwrap();
    index.write().unwrap();

    let tool = GitTool::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = tool
        .execute(
            tool_context(),
            json!({
                "action": "commit",
                "repo_path": temp.path().to_str().unwrap(),
                "message": "Add new file"
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let commit_id = result.output["data"]["commit_id"].as_str().unwrap();
    assert!(!commit_id.is_empty());
    assert_eq!(result.output["data"]["summary"], "Add new file");
}
