use agent_tool::builtin::fs::guard::FsGuard;

#[tokio::test]
async fn guard_denies_path_outside_allowed_roots() {
    let temp_dir = tempfile::tempdir().unwrap();
    let allowed = vec![temp_dir.path().to_path_buf()];
    let guard = FsGuard::new(allowed);

    // Path outside allowed roots should be denied
    let result = guard.authorize_existing("/etc/passwd").await;
    assert!(result.is_err(), "Expected denial for path outside allowed roots");
}

#[tokio::test]
async fn guard_denies_dotdot_traversal_escape() {
    let temp_dir = tempfile::tempdir().unwrap();
    let allowed = vec![temp_dir.path().to_path_buf()];
    let guard = FsGuard::new(allowed);

    // Create a subdirectory
    let subdir = temp_dir.path().join("subdir");
    std::fs::create_dir(&subdir).unwrap();

    // Attempt to escape via ../
    let escape_path = subdir.join("..").join("..").join("..");
    let result = guard.authorize_existing(escape_path.to_str().unwrap()).await;
    assert!(result.is_err(), "Expected denial for dotdot traversal escape");
}

#[tokio::test]
async fn guard_denies_symlink_escape_target() {
    let temp_dir = tempfile::tempdir().unwrap();
    let allowed = vec![temp_dir.path().to_path_buf()];
    let guard = FsGuard::new(allowed);

    // Create a file outside allowed root
    let outside_file = tempfile::NamedTempFile::new().unwrap();
    let outside_path = outside_file.path().to_path_buf();

    // Create symlink inside temp dir pointing to outside
    let link_path = temp_dir.path().join("link");
    std::os::unix::fs::symlink(&outside_path, &link_path).unwrap();

    // Access via symlink should be denied
    let result = guard.authorize_existing(link_path.to_str().unwrap()).await;
    assert!(result.is_err(), "Expected denial for symlink escape");
}
