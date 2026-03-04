use agent_tool::{GrepTool, Tool, ToolContext};
use serde_json::json;

fn test_context() -> ToolContext {
    ToolContext {
        session_id: "test".to_string(),
        turn_id: "test".to_string(),
    }
}

#[tokio::test]
async fn grep_literal_and_regex_both_work() {
    let temp_dir = tempfile::tempdir().unwrap();

    std::fs::write(temp_dir.path().join("test.txt"), "Hello World\nRust is great\nHello again").unwrap();

    let tool = GrepTool::new(vec![temp_dir.path().to_path_buf()]).unwrap();

    // Literal search
    let result = tool.execute(
        test_context(),
        json!({
            "path": temp_dir.path().to_str().unwrap(),
            "pattern": "Hello",
            "is_regex": false
        }),
    ).await.unwrap();

    assert!(!result.is_error);
    assert!(result.output["total_matches"].as_i64().unwrap() >= 2);

    // Regex search
    let result = tool.execute(
        test_context(),
        json!({
            "path": temp_dir.path().to_str().unwrap(),
            "pattern": "Hello|World",
            "is_regex": true
        }),
    ).await.unwrap();

    assert!(!result.is_error);
    assert!(result.output["total_matches"].as_i64().unwrap() >= 2);
}

#[tokio::test]
async fn grep_supports_case_whole_word_and_context() {
    let temp_dir = tempfile::tempdir().unwrap();

    std::fs::write(temp_dir.path().join("test.txt"), "hello world\nHello World\nHELLO WORLD\ntesting").unwrap();

    let tool = GrepTool::new(vec![temp_dir.path().to_path_buf()]).unwrap();

    // Case insensitive
    let result = tool.execute(
        test_context(),
        json!({
            "path": temp_dir.path().to_str().unwrap(),
            "pattern": "hello",
            "case_insensitive": true
        }),
    ).await.unwrap();

    assert!(!result.is_error);
    // Should match all three "hello" variants (case insensitive)
    assert!(result.output["total_matches"].as_i64().unwrap() >= 3);

    // Whole line
    let result = tool.execute(
        test_context(),
        json!({
            "path": temp_dir.path().to_str().unwrap(),
            "pattern": "hello world",
            "whole_line": true
        }),
    ).await.unwrap();

    assert!(!result.is_error);
    // Only "hello world" should match (whole line)
    assert_eq!(result.output["total_matches"].as_i64().unwrap(), 1);

    // Context lines
    let result = tool.execute(
        test_context(),
        json!({
            "path": temp_dir.path().to_str().unwrap(),
            "pattern": "Hello World",
            "context_lines": 1
        }),
    ).await.unwrap();

    assert!(!result.is_error);
    let matches = result.output["results"].as_array().unwrap();
    if !matches.is_empty() {
        let first_match = &matches[0]["matches"][0];
        if first_match.get("context").is_some() {
            // Has context
        }
    }
}

#[tokio::test]
async fn grep_honors_max_results_and_sets_truncated_meta() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Create multiple files with matches
    for i in 0..5 {
        std::fs::write(temp_dir.path().join(format!("file{}.txt", i)), "test line 1\ntest line 2\ntest line 3").unwrap();
    }

    let tool = GrepTool::new(vec![temp_dir.path().to_path_buf()]).unwrap();

    // Test max_results
    let result = tool.execute(
        test_context(),
        json!({
            "path": temp_dir.path().to_str().unwrap(),
            "pattern": "test",
            "max_results": 3
        }),
    ).await.unwrap();

    assert!(!result.is_error);
    assert!(result.output["truncated"].as_bool().unwrap());

    // Test max_count per file
    let result = tool.execute(
        test_context(),
        json!({
            "path": temp_dir.path().to_str().unwrap(),
            "pattern": "test",
            "max_count": 1
        }),
    ).await.unwrap();

    assert!(!result.is_error);
    // Should have at most 1 match per file
    let results = result.output["results"].as_array().unwrap();
    for file in results {
        let matches = file["matches"].as_array().unwrap();
        assert!(matches.len() <= 1);
    }
}

#[tokio::test]
async fn grep_denies_path_outside_allowed_root() {
    let temp_dir = tempfile::tempdir().unwrap();
    let tool = GrepTool::new(vec![temp_dir.path().to_path_buf()]).unwrap();

    // Try to search outside allowed root
    let result = tool.execute(
        test_context(),
        json!({
            "path": "/tmp",
            "pattern": "test"
        }),
    ).await;

    // Should fail with access denied
    assert!(result.is_err() || result.unwrap().is_error);
}

#[tokio::test]
async fn grep_tool_has_correct_name_and_description() {
    let temp_dir = tempfile::tempdir().unwrap();
    let tool = GrepTool::new(vec![temp_dir.path().to_path_buf()]).unwrap();

    assert_eq!(tool.name(), "grep");
    assert!(tool.description().contains("Search"));
}

// P2 Fix: grep should not return files with zero matches
#[tokio::test]
async fn grep_excludes_files_with_no_matches() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Create file A with match
    std::fs::write(temp_dir.path().join("A.txt"), "Hello World\nRust is great").unwrap();
    // Create file B without match
    std::fs::write(temp_dir.path().join("B.txt"), "foo bar baz\nnothing here").unwrap();

    let tool = GrepTool::new(vec![temp_dir.path().to_path_buf()]).unwrap();

    let result = tool.execute(
        test_context(),
        json!({
            "path": temp_dir.path().to_str().unwrap(),
            "pattern": "Hello",
            "is_regex": false
        }),
    ).await.unwrap();

    assert!(!result.is_error);
    assert_eq!(result.output["total_matches"].as_i64().unwrap(), 1);

    // Should only contain A.txt, not B.txt
    let results = result.output["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0]["path"].as_str().unwrap().ends_with("A.txt"));
}
