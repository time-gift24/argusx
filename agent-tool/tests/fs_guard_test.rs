use agent_tool::builtin::fs::guard::FsGuard;
use std::path::PathBuf;

#[tokio::test]
async fn guard_denies_path_outside_allowed_roots() {
    let temp_dir = tempfile::tempdir().unwrap();
    let allowed = vec![temp_dir.path().to_path_buf()];
    let guard = FsGuard::new(allowed).unwrap();

    // Create a file outside allowed roots
    let outside_path = "/etc/passwd";
    let result = guard.authorize_existing(outside_path).await;
    assert!(
        result.is_err(),
        "Expected denial for path outside allowed roots"
    );
}

#[tokio::test]
async fn guard_denies_dotdot_traversal_escape() {
    let temp_dir = tempfile::tempdir().unwrap();
    let allowed = vec![temp_dir.path().to_path_buf()];
    let guard = FsGuard::new(allowed).unwrap();

    // Create a subdirectory
    let subdir = temp_dir.path().join("subdir");
    std::fs::create_dir(&subdir).unwrap();

    // Attempt to escape via ../
    let escape_path = subdir.join("..").join("..").join("..");
    let result = guard
        .authorize_existing(escape_path.to_str().unwrap())
        .await;
    assert!(
        result.is_err(),
        "Expected denial for dotdot traversal escape"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn guard_denies_symlink_escape_target() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir().unwrap();
    let allowed = vec![temp_dir.path().to_path_buf()];
    let guard = FsGuard::new(allowed).unwrap();

    // Create a file outside allowed root
    let outside_file = tempfile::NamedTempFile::new().unwrap();
    let outside_path = outside_file.path().to_path_buf();

    // Create symlink inside temp dir pointing to outside
    let link_path = temp_dir.path().join("link");
    symlink(&outside_path, &link_path).unwrap();

    // Access via symlink should be denied
    let result = guard.authorize_existing(link_path.to_str().unwrap()).await;
    assert!(result.is_err(), "Expected denial for symlink escape");
}

#[tokio::test]
async fn guard_allows_path_within_allowed_roots() {
    let temp_dir = tempfile::tempdir().unwrap();
    let allowed = vec![temp_dir.path().to_path_buf()];
    let guard = FsGuard::new(allowed).unwrap();

    // Create a file inside allowed root
    let inside_path = temp_dir.path().join("test.txt");
    std::fs::write(&inside_path, "test content").unwrap();

    // Access should be allowed
    let result = guard
        .authorize_existing(inside_path.to_str().unwrap())
        .await;
    assert!(
        result.is_ok(),
        "Expected allowed for path within allowed roots"
    );
}

#[tokio::test]
async fn guard_authorize_maybe_new_for_new_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let allowed = vec![temp_dir.path().to_path_buf()];
    let guard = FsGuard::new(allowed).unwrap();

    // Path to a non-existent file
    let new_file_path = temp_dir.path().join("new_file.txt");

    // Should allow creating in allowed directory
    let result = guard
        .authorize_maybe_new(new_file_path.to_str().unwrap())
        .await;
    assert!(
        result.is_ok(),
        "Expected allowed for new file in allowed directory"
    );
}

#[tokio::test]
async fn guard_authorize_maybe_new_denies_parent_outside() {
    let temp_dir = tempfile::tempdir().unwrap();
    let allowed = vec![temp_dir.path().to_path_buf()];
    let guard = FsGuard::new(allowed).unwrap();

    // Path to file in parent directory (should be denied)
    let outside_path = "/tmp/new_file.txt";

    // Should deny because parent is outside allowed roots
    let result = guard.authorize_maybe_new(outside_path).await;
    assert!(
        result.is_err(),
        "Expected denial for parent outside allowed roots"
    );
}

#[tokio::test]
async fn guard_new_rejects_invalid_root() {
    // Non-existent directory should fail
    let invalid_root = PathBuf::from("/nonexistent/path/that/does/not/exist");
    let result = FsGuard::new(vec![invalid_root]);
    assert!(result.is_err(), "Expected error for non-existent root");
}

#[tokio::test]
async fn guard_new_rejects_empty_roots() {
    let result = FsGuard::new(vec![]);
    assert!(result.is_err(), "Expected error for empty roots");
}

#[tokio::test]
async fn guard_denies_nonexistent_path_in_existing() {
    let temp_dir = tempfile::tempdir().unwrap();
    let allowed = vec![temp_dir.path().to_path_buf()];
    let guard = FsGuard::new(allowed).unwrap();

    // Path that doesn't exist should return NotFound
    let nonexistent = temp_dir.path().join("nonexistent.txt");
    let result = guard
        .authorize_existing(nonexistent.to_str().unwrap())
        .await;
    assert!(result.is_err());
    // Check it's a NotFound error, not AccessDenied
    if let Err(e) = result {
        assert!(matches!(
            e,
            agent_tool::builtin::fs::error::FsError::NotFound(_)
        ));
    }
}
