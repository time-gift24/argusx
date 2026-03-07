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

#[tokio::test]
async fn branch_create_creates_new_branch() {
    let (temp, _repo) = create_repo_for_writes();
    let tool = GitTool::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = tool
        .execute(
            tool_context(),
            json!({
                "action": "branch_create",
                "repo_path": temp.path().to_str().unwrap(),
                "branch": "feature-branch"
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    assert_eq!(result.output["action"], "branch_create");
    assert_eq!(result.output["data"]["branch"], "feature-branch");
    assert!(!result.output["data"]["checked_out"].as_bool().unwrap());
}

#[tokio::test]
async fn branch_create_with_checkout() {
    let (temp, _repo) = create_repo_for_writes();
    let tool = GitTool::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = tool
        .execute(
            tool_context(),
            json!({
                "action": "branch_create",
                "repo_path": temp.path().to_str().unwrap(),
                "branch": "feature-branch",
                "checkout": true
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    assert!(result.output["data"]["checked_out"].as_bool().unwrap());
}

#[tokio::test]
async fn checkout_switches_branch() {
    let (temp, repo) = create_repo_for_writes();

    // Create another branch
    let head = repo.head().unwrap().target().unwrap();
    let commit = repo.find_commit(head).unwrap();
    repo.branch("other-branch", &commit, false).unwrap();

    let tool = GitTool::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = tool
        .execute(
            tool_context(),
            json!({
                "action": "checkout",
                "repo_path": temp.path().to_str().unwrap(),
                "branch": "other-branch"
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    assert_eq!(result.output["action"], "checkout");
    assert_eq!(result.output["data"]["branch"], "other-branch");
}

#[tokio::test]
async fn checkout_rejects_dirty_worktree() {
    let (temp, repo) = create_repo_for_writes();

    // Create another branch
    let head = repo.head().unwrap().target().unwrap();
    let commit = repo.find_commit(head).unwrap();
    repo.branch("other-branch", &commit, false).unwrap();

    // Make uncommitted changes
    std::fs::write(temp.path().join("initial.txt"), b"modified\n").unwrap();

    let tool = GitTool::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = tool
        .execute(
            tool_context(),
            json!({
                "action": "checkout",
                "repo_path": temp.path().to_str().unwrap(),
                "branch": "other-branch"
            }),
        )
        .await;

    // Should return an error (Err or Ok with is_error=true)
    match result {
        Ok(r) => assert!(r.is_error, "expected error for dirty worktree"),
        Err(_) => {} // Error is also acceptable
    }
}

#[tokio::test]
async fn clone_creates_local_copy() {
    // Create source repo
    let source_temp = tempfile::tempdir().unwrap();
    let source_repo = Repository::init(&source_temp.path()).unwrap();
    let mut config = source_repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    drop(config);

    // Create initial commit
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    std::fs::write(source_temp.path().join("README.md"), b"test\n").unwrap();
    let mut index = source_repo.index().unwrap();
    index.add_path(Path::new("README.md")).unwrap();
    let tree_id = index.write_tree().unwrap();
    drop(index);
    let tree = source_repo.find_tree(tree_id).unwrap();
    source_repo.commit(Some("HEAD"), &sig, &sig, "Initial\n", &tree, &[]).unwrap();
    drop(tree);

    // Clone target
    let target_temp = tempfile::tempdir().unwrap();
    let target_path = target_temp.path().to_str().unwrap();
    let source_url = source_temp.path().to_str().unwrap();

    let tool = GitTool::new(vec![target_temp.path().to_path_buf()]).unwrap();

    let result = tool
        .execute(
            tool_context(),
            json!({
                "action": "clone",
                "url": source_url,
                "target_path": target_path
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    assert!(result.output["data"]["repo_path"].is_string());
    assert!(std::fs::exists(target_path).unwrap());
}

#[tokio::test]
async fn fetch_updates_from_remote() {
    let (source_temp, source_repo) = create_repo_for_writes();

    // Clone to target (this creates origin automatically)
    let target_temp = tempfile::tempdir().unwrap();
    let target_path = target_temp.path().to_str().unwrap();
    let source_url = source_temp.path().to_str().unwrap();

    git2::Repository::clone(source_url, target_path).unwrap();

    // Add new commit to source
    std::fs::write(source_temp.path().join("new.txt"), b"new\n").unwrap();
    let mut index = source_repo.index().unwrap();
    index.add_path(Path::new("new.txt")).unwrap();
    let tree_id = index.write_tree().unwrap();
    drop(index);
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    let tree = source_repo.find_tree(tree_id).unwrap();
    let head = source_repo.head().unwrap().target().unwrap();
    let parent = source_repo.find_commit(head).unwrap();
    source_repo.commit(Some("HEAD"), &sig, &sig, "Add new\n", &tree, &[&parent]).unwrap();

    let tool = GitTool::new(vec![target_temp.path().to_path_buf()]).unwrap();

    let result = tool
        .execute(
            tool_context(),
            json!({
                "action": "fetch",
                "repo_path": target_path,
                "remote": "origin"
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    assert_eq!(result.output["action"], "fetch");
}
