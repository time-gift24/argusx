use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use walkdir::WalkDir;

use crate::builtin::fs::guard::FsGuard;
use crate::builtin::fs::error::FsError;
use crate::context::{ToolContext, ToolResult};
use crate::error::ToolError;
use crate::spec::ToolSpec;
use crate::trait_def::Tool;

pub struct GrepTool {
    guard: FsGuard,
}

impl GrepTool {
    pub fn new(allowed_roots: Vec<PathBuf>) -> Result<Self, FsError> {
        let guard = FsGuard::new(allowed_roots)?;
        Ok(Self { guard })
    }

    /// Get default grep tool with current directory as allowed root
    pub fn default() -> Result<Self, FsError> {
        Self::new(vec![std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))])
    }
}

#[derive(Debug, Deserialize)]
struct GrepArgs {
    path: String,
    pattern: String,
    #[serde(default)]
    is_regex: bool,
    #[serde(default)]
    case_insensitive: bool,
    #[serde(default)]
    whole_line: bool,
    #[serde(default = "default_max_results")]
    max_results: Option<usize>,
    #[serde(default)]
    context_lines: Option<usize>,
    #[serde(default)]
    max_count: Option<usize>,
}

fn default_max_results() -> Option<usize> {
    Some(100)
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search for patterns in files using regex or literal match"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory or file to search"
                    },
                    "pattern": {
                        "type": "string",
                        "description": "Search pattern (regex or literal)"
                    },
                    "is_regex": {
                        "type": "boolean",
                        "description": "Treat pattern as regex (default: false)",
                        "default": false
                    },
                    "case_insensitive": {
                        "type": "boolean",
                        "description": "Case insensitive search",
                        "default": false
                    },
                    "whole_line": {
                        "type": "boolean",
                        "description": "Match whole line only",
                        "default": false
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of matches"
                    },
                    "context_lines": {
                        "type": "integer",
                        "description": "Number of context lines to include"
                    },
                    "max_count": {
                        "type": "integer",
                        "description": "Maximum matches per file"
                    }
                },
                "required": ["path", "pattern"]
            }),
        }
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        args: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let args: GrepArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        // Authorize the base path
        let authorized_path = self.guard.authorize_existing(&args.path).await
            .map_err(|e| map_fs_error(e))?;

        // Build regex pattern
        let pattern = if args.is_regex {
            args.pattern.clone()
        } else {
            // Escape for literal search
            regex::escape(&args.pattern)
        };

        // Add anchors if whole_line is set
        let pattern = if args.whole_line {
            format!("^{}$", pattern)
        } else {
            pattern
        };

        // Build regex with case sensitivity
        let regex_pattern = if args.case_insensitive {
            format!("(?i){}", pattern)
        } else {
            pattern
        };

        let re = regex::Regex::new(&regex_pattern)
            .map_err(|e| ToolError::InvalidArgs(format!("invalid pattern: {}", e)))?;

        let max_results = args.max_results.unwrap_or(100);
        let context_lines = args.context_lines.unwrap_or(0);
        let max_count = args.max_count;

        let mut results = Vec::new();
        let mut total_matches = 0usize;

        let walk = WalkDir::new(&authorized_path)
            .follow_links(false)
            .max_depth(10);

        for entry in walk.into_iter().filter_map(|e| e.ok()) {

            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // Read file content
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Calculate remaining slots for this file
            let remaining = max_results.saturating_sub(total_matches);
            // Use min of max_count (per-file) and remaining (global)
            let file_max = max_count.map(|c| c.min(remaining)).unwrap_or(remaining);

            let file_path = path.to_path_buf();
            let file_matches = search_content(&content, &re, context_lines, Some(file_max))?;

            total_matches += file_matches.len();
            results.push(json!({
                "path": file_path.to_string_lossy(),
                "matches": file_matches,
            }));

            if total_matches >= max_results {
                break;
            }
        }

        Ok(ToolResult::ok(json!({
            "results": results,
            "total_matches": total_matches,
            "truncated": total_matches >= max_results,
        })))
    }
}

fn search_content(
    content: &str,
    re: &regex::Regex,
    context_lines: usize,
    max_count: Option<usize>,
) -> Result<Vec<serde_json::Value>, std::io::Error> {
    let max_count = max_count.unwrap_or(usize::MAX);
    let mut matches = Vec::new();
    let mut line_matches = 0usize;

    for (line_num, line) in content.lines().enumerate() {
        if line_matches >= max_count {
            break;
        }

        if re.is_match(line) {
            // Get context lines
            let context_start = line_num.saturating_sub(context_lines);
            let context_end = (line_num + context_lines + 1).min(content.lines().count());
            let context: Vec<String> = content.lines()
                .skip(context_start)
                .take(context_end - context_start)
                .map(|s| s.to_string())
                .collect();

            matches.push(json!({
                "line_number": line_num + 1,
                "line": line,
                "context": context,
            }));
            line_matches += 1;
        }
    }

    Ok(matches)
}

fn map_fs_error(e: FsError) -> ToolError {
    match e {
        FsError::AccessDenied(p, _) => ToolError::ExecutionFailed(format!("Access denied: {}", p)),
        FsError::NotFound(p) => ToolError::ExecutionFailed(format!("Not found: {}", p)),
        FsError::InvalidPath(p) => ToolError::InvalidArgs(p),
        FsError::InvalidRoot(p, _) => ToolError::ExecutionFailed(format!("Invalid root: {}", p)),
        FsError::Io(p, msg) => ToolError::ExecutionFailed(format!("IO error on {}: {}", p, msg)),
    }
}
