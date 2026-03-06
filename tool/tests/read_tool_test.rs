use tool::{ReadTool, Tool, ToolContext};
use serde_json::json;

fn test_context() -> ToolContext {
    ToolContext {
        session_id: "test".to_string(),
        turn_id: "test".to_string(),
    }
}

#[tokio::test]
async fn read_text_mode_returns_full_content() {
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    std::fs::write(&test_file, "Hello\nWorld\nTest").unwrap();

    // Create ReadTool with temp_dir as allowed root
    let tool = ReadTool::new(vec![temp_dir.path().to_path_buf()]).unwrap();
    let result = tool
        .execute(
            test_context(),
            json!({
                "path": test_file.to_str().unwrap(),
                "mode": "text"
            }),
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    assert_eq!(result.output["content"], "Hello\nWorld\nTest");
}

#[tokio::test]
async fn read_head_tail_and_lines_modes_work() {
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file = temp_dir.path().join("lines.txt");
    std::fs::write(&test_file, "line1\nline2\nline3\nline4\nline5").unwrap();

    let tool = ReadTool::new(vec![temp_dir.path().to_path_buf()]).unwrap();

    // Head mode
    let result = tool
        .execute(
            test_context(),
            json!({
                "path": test_file.to_str().unwrap(),
                "mode": "head",
                "limit": 2
            }),
        )
        .await
        .unwrap();
    assert!(!result.is_error);
    assert_eq!(result.output["lines"].as_array().unwrap().len(), 2);

    // Tail mode
    let result = tool
        .execute(
            test_context(),
            json!({
                "path": test_file.to_str().unwrap(),
                "mode": "tail",
                "limit": 2
            }),
        )
        .await
        .unwrap();
    assert!(!result.is_error);
    assert_eq!(result.output["lines"].as_array().unwrap().len(), 2);

    // Lines mode with offset and limit
    let result = tool
        .execute(
            test_context(),
            json!({
                "path": test_file.to_str().unwrap(),
                "mode": "lines",
                "offset": 1,
                "limit": 2
            }),
        )
        .await
        .unwrap();
    assert!(!result.is_error);
    assert_eq!(result.output["lines"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn read_stat_and_list_modes_work() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Test stat on file
    let test_file = temp_dir.path().join("stat_test.txt");
    std::fs::write(&test_file, "content").unwrap();

    let tool = ReadTool::new(vec![temp_dir.path().to_path_buf()]).unwrap();

    let result = tool
        .execute(
            test_context(),
            json!({
                "path": test_file.to_str().unwrap(),
                "mode": "stat"
            }),
        )
        .await
        .unwrap();
    assert!(!result.is_error);
    assert!(result.output["is_file"].as_bool().unwrap());
    assert!(!result.output["is_dir"].as_bool().unwrap());

    // Test list on directory
    let result = tool
        .execute(
            test_context(),
            json!({
                "path": temp_dir.path().to_str().unwrap(),
                "mode": "list"
            }),
        )
        .await
        .unwrap();
    assert!(!result.is_error);
    assert!(result.output["entries"].is_array());
}

#[tokio::test]
async fn read_batch_mode_returns_per_path_results() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Create multiple files
    std::fs::write(temp_dir.path().join("a.txt"), "content A").unwrap();
    std::fs::write(temp_dir.path().join("b.txt"), "content B").unwrap();

    let tool = ReadTool::new(vec![temp_dir.path().to_path_buf()]).unwrap();

    // Batch read directory
    let result = tool
        .execute(
            test_context(),
            json!({
                "path": temp_dir.path().to_str().unwrap(),
                "mode": "batch"
            }),
        )
        .await
        .unwrap();
    assert!(!result.is_error);
    assert_eq!(result.output["count"].as_i64().unwrap(), 2);
}

#[tokio::test]
async fn read_tool_has_correct_name_and_description() {
    let temp_dir = tempfile::tempdir().unwrap();
    let tool = ReadTool::new(vec![temp_dir.path().to_path_buf()]).unwrap();
    assert_eq!(tool.name(), "read");
    assert!(tool.description().contains("Read-only"));
    assert!(tool.spec().input_schema.get("properties").is_some());
}

#[tokio::test]
async fn read_denies_path_outside_allowed_root() {
    let temp_dir = tempfile::tempdir().unwrap();
    let tool = ReadTool::new(vec![temp_dir.path().to_path_buf()]).unwrap();

    // Try to read /etc/passwd which is outside allowed root
    let result = tool
        .execute(
            test_context(),
            json!({
                "path": "/etc/passwd",
                "mode": "text"
            }),
        )
        .await;

    // Should fail with access denied
    assert!(result.is_err() || result.unwrap().is_error);
}
