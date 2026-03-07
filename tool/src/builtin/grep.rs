use async_trait::async_trait;
use grep::regex::RegexMatcher;
use grep::searcher::Searcher;
use grep::searcher::Sink;
use grep::searcher::SinkMatch;
use ignore::WalkBuilder;
use regex::escape;
use serde::Deserialize;
use serde_json::json;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;

use crate::builtin::fs::error::FsError;
use crate::builtin::fs::guard::FsGuard;
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

    /// Build a grep tool with the current directory as the allowed root.
    pub fn from_current_dir() -> Result<Self, FsError> {
        Self::new(vec![
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        ])
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
        let args: GrepArgs =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        // Authorize the base path
        let authorized_path = self
            .guard
            .authorize_existing(&args.path)
            .await
            .map_err(map_fs_error)?;

        // Build regex pattern
        let pattern = if args.is_regex {
            args.pattern.clone()
        } else {
            // Escape for literal search using regex escape
            escape(&args.pattern)
        };

        // Add anchors if whole_line is set
        let pattern = if args.whole_line {
            format!("^{}$", pattern)
        } else {
            pattern
        };

        // Build regex matcher with case sensitivity
        let regex_pattern = if args.case_insensitive {
            format!("(?i){}", pattern)
        } else {
            pattern
        };

        let matcher = RegexMatcher::new(&regex_pattern)
            .map_err(|e| ToolError::InvalidArgs(format!("invalid pattern: {}", e)))?;

        let matcher = Arc::new(matcher);
        let mut searcher = Searcher::new();

        let max_results = args.max_results.unwrap_or(100);
        let context_lines = args.context_lines.unwrap_or(0);
        let max_count = args.max_count;

        let mut results = Vec::new();
        let mut total_matches = 0usize;

        // Use ignore::WalkBuilder for file traversal (same as ripgrep)
        let walk = WalkBuilder::new(&authorized_path)
            .follow_links(false)
            .max_depth(Some(10))
            .build();

        for entry in walk {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let file_path = path.to_path_buf();

            // Calculate remaining slots for this file
            let remaining = max_results.saturating_sub(total_matches);
            let file_max = max_count.map(|c| c.min(remaining)).unwrap_or(remaining);

            // Search in this file using ripgrep's searcher
            let file_matches = search_file(
                &mut searcher,
                matcher.clone(),
                &file_path,
                context_lines,
                file_max,
            )?;

            // P2 Fix: Only include files with matches
            if !file_matches.is_empty() {
                total_matches += file_matches.len();
                results.push(json!({
                    "path": file_path.to_string_lossy(),
                    "matches": file_matches,
                }));
            }

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

fn search_file(
    searcher: &mut Searcher,
    matcher: Arc<RegexMatcher>,
    path: &std::path::Path,
    context_lines: usize,
    max_count: usize,
) -> Result<Vec<serde_json::Value>, std::io::Error> {
    let _file = std::fs::File::open(path)?;
    let _reader = std::io::BufReader::new(_file);

    let mut matches = Vec::new();
    let mut line_number = 0usize;
    let mut pending_context: Vec<String> = Vec::new();

    // Use ripgrep's sink to process the file
    struct RipgrepSink<'a> {
        matches: &'a mut Vec<serde_json::Value>,
        line_number: &'a mut usize,
        pending_context: &'a mut Vec<String>,
        _context_lines: usize,
        match_count: &'a mut usize,
        max_count: usize,
    }

    impl Sink for RipgrepSink<'_> {
        type Error = std::io::Error;

        fn matched(
            &mut self,
            _searcher: &Searcher,
            match_info: &SinkMatch,
        ) -> std::io::Result<bool> {
            *self.match_count += 1;

            // Get line number from the match
            if let Some(line) = match_info.line_number() {
                *self.line_number = line as usize;
            }

            // Get the matched line text - lines() returns bytes, convert to string
            let line_bytes = match_info.lines().next().unwrap_or(b"");
            let line_text = String::from_utf8_lossy(line_bytes).to_string();

            // Get context lines
            let context = self.pending_context.clone();

            self.matches.push(json!({
                "line_number": *self.line_number,
                "line": line_text,
                "context": context,
            }));

            if *self.match_count >= self.max_count {
                return Ok(false); // Stop searching
            }
            Ok(true) // Continue searching
        }
    }

    // Pre-read file to build context lines
    let file_content = std::fs::read_to_string(path)?;
    let all_lines: Vec<&str> = file_content.lines().collect();
    let total_lines = all_lines.len();

    let sink = RipgrepSink {
        matches: &mut matches,
        line_number: &mut line_number,
        pending_context: &mut pending_context,
        _context_lines: context_lines,
        match_count: &mut 0,
        max_count,
    };

    // Search the file using ripgrep's searcher
    let mut reader = Cursor::new(file_content.as_bytes());
    let _ = searcher.search_reader(&*matcher, &mut reader, sink);

    // Now add context lines by re-reading the file
    // We need to rebuild the matches with context
    // Note: SinkMatch::line_number() is 1-based, convert to 0-based for array indexing
    let mut matches_with_context = Vec::new();
    for m in &matches {
        let line_num = m.get("line_number").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        // Convert from 1-based to 0-based index
        let line_idx = line_num.saturating_sub(1);

        let context_start = line_idx.saturating_sub(context_lines);
        // Include context_lines after the matched line (exclusive of matched line)
        let context_end = (line_idx + 1 + context_lines).min(total_lines);

        let context: Vec<String> = (context_start..context_end)
            .filter(|&i| i != line_idx)
            .map(|i| all_lines.get(i).unwrap_or(&"").to_string())
            .collect();

        matches_with_context.push(json!({
            "line_number": line_num,
            "line": m.get("line").and_then(|v| v.as_str()).unwrap_or(""),
            "context": context,
        }));
    }

    Ok(matches_with_context)
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
