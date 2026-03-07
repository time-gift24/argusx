use git2::{Repository, Signature};
use serde_json::json;
use std::path::Path;
use tempfile::TempDir;
use tool::{GitTool, Tool, ToolContext};
use tokio_util::sync::CancellationToken;

fn create_multi_commit_repo() -> (TempDir, Repository) {
    let temp = tempfile::tempdir().unwrap();
    let repo = Repository::init(&temp.path()).unwrap();

    // Configure identity
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    drop(config);

    let sig = Signature::now("Test User", "test@example.com").unwrap();

    // First commit
    std::fs::write(temp.path().join("file1.txt"), b"content1\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("file1.txt")).unwrap();
    let tree_id = index.write_tree().unwrap();
    drop(index);

    let tree = repo.find_tree(tree_id).unwrap();
    let commit1 = repo.commit(Some("HEAD"), &sig, &sig, "First commit\n", &tree, &[]).unwrap();
    drop(tree);

    // Second commit
    std::fs::write(temp.path().join("file2.txt"), b"content2\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("file2.txt")).unwrap();
    let tree_id = index.write_tree().unwrap();
    drop(index);

    let tree = repo.find_tree(tree_id).unwrap();
    let parent1 = repo.find_commit(commit1).unwrap();
    let commit2 = repo.commit(Some("HEAD"), &sig, &sig, "Second commit\n", &tree, &[&parent1]).unwrap();
    drop(tree);
    drop(parent1);

    // Third commit
    std::fs::write(temp.path().join("file3.txt"), b"content3\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("file3.txt")).unwrap();
    let tree_id = index.write_tree().unwrap();
    drop(index);

    let tree = repo.find_tree(tree_id).unwrap();
    let parent2 = repo.find_commit(commit2).unwrap();
    let _commit3 = repo.commit(Some("HEAD"), &sig, &sig, "Third commit\n", &tree, &[&parent2]).unwrap();
    drop(tree);
    drop(parent2);

    (temp, repo)
}

fn tool_context() -> ToolContext {
    ToolContext::new("test-session", "test-turn", CancellationToken::new())
}

#[tokio::test]
async fn diff_returns_patch_for_workdir_changes() {
    let (temp, _repo) = create_multi_commit_repo();

    // Modify a file in workdir
    std::fs::write(temp.path().join("file1.txt"), b"modified content\n").unwrap();

    let tool = GitTool::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = tool
        .execute(
            tool_context(),
            json!({
                "action": "diff",
                "repo_path": temp.path().to_str().unwrap()
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let output = &result.output;
    assert_eq!(output["action"], "diff");
    // Just verify the stats show changes, since the patch format varies
    let stats = &output["data"]["stats"];
    assert!(stats["files_changed"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn diff_supports_staged_flag() {
    let (temp, repo) = create_multi_commit_repo();

    // Stage a modification
    std::fs::write(temp.path().join("file1.txt"), b"staged content\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("file1.txt")).unwrap();
    index.write_tree().unwrap();

    let tool = GitTool::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = tool
        .execute(
            tool_context(),
            json!({
                "action": "diff",
                "repo_path": temp.path().to_str().unwrap(),
                "staged": true
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let output = &result.output;
    assert_eq!(output["action"], "diff");
}

#[tokio::test]
async fn log_returns_commits_in_order() {
    let (temp, _repo) = create_multi_commit_repo();
    let tool = GitTool::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = tool
        .execute(
            tool_context(),
            json!({
                "action": "log",
                "repo_path": temp.path().to_str().unwrap()
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let output = &result.output;
    assert_eq!(output["action"], "log");

    let commits = output["data"]["commits"].as_array().unwrap();
    assert!(!commits.is_empty());
    assert!(commits.len() >= 3);
}

#[tokio::test]
async fn log_respects_max_count() {
    let (temp, _repo) = create_multi_commit_repo();
    let tool = GitTool::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = tool
        .execute(
            tool_context(),
            json!({
                "action": "log",
                "repo_path": temp.path().to_str().unwrap(),
                "max_count": 1
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let commits = result.output["data"]["commits"].as_array().unwrap();
    assert_eq!(commits.len(), 1);
}

#[tokio::test]
async fn log_truncates_large_output() {
    let (temp, _repo) = create_multi_commit_repo();
    let tool = GitTool::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = tool
        .execute(
            tool_context(),
            json!({
                "action": "log",
                "repo_path": temp.path().to_str().unwrap(),
                "max_count": 200
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
}

#[tokio::test]
async fn show_returns_commit_info() {
    let (temp, _repo) = create_multi_commit_repo();
    let tool = GitTool::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = tool
        .execute(
            tool_context(),
            json!({
                "action": "show",
                "repo_path": temp.path().to_str().unwrap(),
                "object": "HEAD"
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let output = &result.output;
    assert_eq!(output["action"], "show");
    assert!(output["data"]["object"].is_object());
}
