use agent_tool::{GlobTool, Tool, ToolContext};
use serde_json::json;

fn test_context() -> ToolContext {
    ToolContext {
        session_id: "test".to_string(),
        turn_id: "test".to_string(),
    }
}

#[tokio::test]
async fn glob_matches_pattern_and_excludes_paths() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Create test files
    std::fs::write(temp_dir.path().join("a.rs"), "mod a;").unwrap();
    std::fs::write(temp_dir.path().join("b.rs"), "mod b;").unwrap();
    std::fs::write(temp_dir.path().join("c.txt"), "text").unwrap();

    let tool = GlobTool::new(vec![temp_dir.path().to_path_buf()]).unwrap();

    // Match *.rs
    let result = tool
        .execute(
            test_context(),
            json!({
                "path": temp_dir.path().to_str().unwrap(),
                "pattern": "*.rs"
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    assert_eq!(result.output["count"].as_i64().unwrap(), 2);
}

#[tokio::test]
async fn glob_honors_max_depth_and_max_results() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Create nested directories with files
    let subdir = temp_dir.path().join("subdir");
    std::fs::create_dir(&subdir).unwrap();
    std::fs::write(temp_dir.path().join("root.txt"), "root").unwrap();
    std::fs::write(subdir.join("nested.txt"), "nested").unwrap();

    let tool = GlobTool::new(vec![temp_dir.path().to_path_buf()]).unwrap();

    // Test max_depth = 1 (should not find nested.txt)
    let result = tool
        .execute(
            test_context(),
            json!({
                "path": temp_dir.path().to_str().unwrap(),
                "pattern": "*.txt",
                "max_depth": 1
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    // Only root.txt should be found
    assert!(result.output["results"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| { r["path"].as_str().unwrap().contains("root.txt") }));

    // Test max_results
    let result = tool
        .execute(
            test_context(),
            json!({
                "path": temp_dir.path().to_str().unwrap(),
                "pattern": "*.txt",
                "max_results": 1
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    assert_eq!(result.output["count"].as_i64().unwrap(), 1);
    assert!(result.output["truncated"].as_bool().unwrap());
}

#[tokio::test]
async fn glob_honors_min_max_bytes_filters() {
    let temp_dir = tempfile::tempdir().unwrap();

    std::fs::write(temp_dir.path().join("small.txt"), "hi").unwrap();
    std::fs::write(
        temp_dir.path().join("large.txt"),
        "this is a larger file content",
    )
    .unwrap();

    let tool = GlobTool::new(vec![temp_dir.path().to_path_buf()]).unwrap();

    // Test min_size
    let result = tool
        .execute(
            test_context(),
            json!({
                "path": temp_dir.path().to_str().unwrap(),
                "pattern": "*.txt",
                "min_size": 10
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    assert_eq!(result.output["count"].as_i64().unwrap(), 1);
    assert!(result.output["results"].as_array().unwrap()[0]["path"]
        .as_str()
        .unwrap()
        .contains("large.txt"));

    // Test max_size
    let result = tool
        .execute(
            test_context(),
            json!({
                "path": temp_dir.path().to_str().unwrap(),
                "pattern": "*.txt",
                "max_size": 5
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    assert_eq!(result.output["count"].as_i64().unwrap(), 1);
    assert!(result.output["results"].as_array().unwrap()[0]["path"]
        .as_str()
        .unwrap()
        .contains("small.txt"));
}

#[tokio::test]
async fn glob_denies_path_outside_allowed_root() {
    let temp_dir = tempfile::tempdir().unwrap();
    let tool = GlobTool::new(vec![temp_dir.path().to_path_buf()]).unwrap();

    // Try to search outside allowed root
    let result = tool
        .execute(
            test_context(),
            json!({
                "path": "/tmp",
                "pattern": "*.txt"
            }),
        )
        .await;

    // Should fail with access denied
    assert!(result.is_err() || result.unwrap().is_error);
}

#[tokio::test]
async fn glob_tool_has_correct_name_and_description() {
    let temp_dir = tempfile::tempdir().unwrap();
    let tool = GlobTool::new(vec![temp_dir.path().to_path_buf()]).unwrap();

    assert_eq!(tool.name(), "glob");
    assert!(tool.description().contains("pattern"));
}

#[tokio::test]
async fn glob_matches_relative_paths_and_excludes_directories() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Create nested structure
    let src_dir = temp_dir.path().join("src");
    let target_dir = temp_dir.path().join("target");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::create_dir(&target_dir).unwrap();

    std::fs::write(src_dir.join("main.rs"), "fn main()").unwrap();
    std::fs::write(src_dir.join("lib.rs"), "pub fn lib()").unwrap();
    std::fs::write(target_dir.join("output.rs"), "generated").unwrap();

    let tool = GlobTool::new(vec![temp_dir.path().to_path_buf()]).unwrap();

    // Test: match "src/**/*.rs" - should find files in src subdirectory
    let result = tool
        .execute(
            test_context(),
            json!({
                "path": temp_dir.path().to_str().unwrap(),
                "pattern": "src/**/*.rs"
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    assert_eq!(result.output["count"].as_i64().unwrap(), 2);

    // Test: exclude "target/**" pattern
    let result = tool
        .execute(
            test_context(),
            json!({
                "path": temp_dir.path().to_str().unwrap(),
                "pattern": "**/*.rs",
                "exclude": "target/**"
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    // Should find main.rs and lib.rs but NOT target/output.rs
    assert_eq!(result.output["count"].as_i64().unwrap(), 2);
    let paths: Vec<&str> = result.output["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["path"].as_str().unwrap())
        .collect();
    assert!(!paths.iter().any(|p| p.contains("target/")));
}
