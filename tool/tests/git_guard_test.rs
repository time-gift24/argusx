use git2::{Repository, Signature};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tool::builtin::git::guard::GitGuard;

fn setup_repo() -> (TempDir, Repository) {
    let temp = tempfile::tempdir().unwrap();
    let repo = Repository::init(temp.path()).unwrap();

    // Configure identity for commits
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();

    // Create initial commit
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    let mut index = repo.index().unwrap();
    std::fs::write(temp.path().join("README.md"), b"# Test Repo\n").unwrap();
    index.add_path(Path::new("README.md")).unwrap();

    let tree_id = index.write_tree().unwrap();
    // Drop index before creating tree to avoid borrow conflict
    drop(index);

    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit\n", &tree, &[])
        .unwrap();

    // Drop tree before returning
    drop(tree);

    (temp, repo)
}

#[tokio::test]
async fn authorize_repo_accepts_normal_repo() {
    let (temp, _repo) = setup_repo();
    let guard = GitGuard::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = guard.authorize_repo(temp.path().to_str().unwrap()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn authorize_repo_accepts_bare_repo() {
    let temp = tempfile::tempdir().unwrap();
    let _repo = Repository::init_bare(temp.path()).unwrap();

    let guard = GitGuard::new(vec![temp.path().to_path_buf()]).unwrap();
    let result = guard.authorize_repo(temp.path().to_str().unwrap()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn authorize_clone_target_rejects_non_empty_dir() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("existing.txt"), b"data\n").unwrap();

    let guard = GitGuard::new(vec![temp.path().to_path_buf()]).unwrap();
    let result = guard
        .authorize_clone_target(temp.path().to_str().unwrap())
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("not empty") || err.to_string().contains("git_invalid_path"));
}

#[tokio::test]
async fn authorize_clone_target_accepts_empty_dir() {
    let temp = tempfile::tempdir().unwrap();

    let guard = GitGuard::new(vec![temp.path().to_path_buf()]).unwrap();
    let result = guard
        .authorize_clone_target(temp.path().to_str().unwrap())
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn authorize_clone_target_accepts_nonexistent_path() {
    let temp = tempfile::tempdir().unwrap();
    let new_path = temp.path().join("new_repo");

    let guard = GitGuard::new(vec![temp.path().to_path_buf()]).unwrap();
    let result = guard
        .authorize_clone_target(new_path.to_str().unwrap())
        .await;

    assert!(result.is_ok());
}

#[test]
fn validate_repo_relative_paths_rejects_absolute() {
    let (temp, repo) = setup_repo();
    let guard = GitGuard::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = guard.validate_repo_relative_paths(&repo, &["/etc/passwd".to_string()]);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("absolute") || err.to_string().contains("git_invalid_path"));
}

#[test]
fn validate_repo_relative_paths_rejects_parent_traversal() {
    let (temp, repo) = setup_repo();
    let guard = GitGuard::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = guard.validate_repo_relative_paths(&repo, &["../../../etc/passwd".to_string()]);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("..") || err.to_string().contains("git_invalid_path"));
}

#[test]
fn validate_repo_relative_paths_accepts_valid_relative() {
    let (temp, repo) = setup_repo();
    let guard = GitGuard::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = guard
        .validate_repo_relative_paths(&repo, &["src/main.rs".to_string(), "README.md".to_string()]);

    assert!(result.is_ok());
    let paths = result.unwrap();
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0], PathBuf::from("src/main.rs"));
    assert_eq!(paths[1], PathBuf::from("README.md"));
}

#[tokio::test]
async fn authorize_repo_rejects_outside_allowed_roots() {
    let (temp, _repo) = setup_repo();
    let other_temp = tempfile::tempdir().unwrap();

    // Guard only allows temp.path(), not other_temp.path()
    let guard = GitGuard::new(vec![temp.path().to_path_buf()]).unwrap();

    let result = guard
        .authorize_repo(other_temp.path().to_str().unwrap())
        .await;
    assert!(result.is_err());
}
