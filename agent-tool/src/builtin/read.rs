use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::builtin::fs::guard::FsGuard;
use crate::builtin::fs::error::FsError;
use crate::context::{ToolContext, ToolResult};
use crate::error::ToolError;
use crate::spec::ToolSpec;
use crate::trait_def::Tool;

pub struct ReadTool {
    guard: FsGuard,
}

impl ReadTool {
    pub fn new(allowed_roots: Vec<std::path::PathBuf>) -> Result<Self, FsError> {
        let guard = FsGuard::new(allowed_roots)?;
        Ok(Self { guard })
    }

    /// Get default read tool with current directory as allowed root
    pub fn default() -> Result<Self, FsError> {
        Self::new(vec![std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))])
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ReadMode {
    Text,
    Lines,
    Head,
    Tail,
    Stat,
    List,
    Batch,
}

#[derive(Debug, Deserialize)]
struct ReadArgs {
    path: String,
    #[serde(default = "default_mode")]
    mode: ReadMode,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    offset: Option<usize>,
}

fn default_mode() -> ReadMode {
    ReadMode::Text
}

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str {
        "read"
    }

    fn description(&self) -> &str {
        "Read-only filesystem operations: text, lines, head/tail, stat, list, batch"
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
                        "description": "Path to the file or directory"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["text", "lines", "head", "tail", "stat", "list", "batch"],
                        "description": "Read mode: text (default), lines, head, tail, stat, list, batch",
                        "default": "text"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Limit number of lines (for lines/head/tail modes)"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Offset for line-based modes"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        args: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let args: ReadArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        // Authorize the path using FsGuard
        let authorized_path = self.guard.authorize_existing(&args.path).await
            .map_err(|e| map_fs_error(e))?;

        let result = match args.mode {
            ReadMode::Text => read_text(&authorized_path).await,
            ReadMode::Lines => read_lines(&authorized_path, args.offset, args.limit).await,
            ReadMode::Head => read_head(&authorized_path, args.limit.unwrap_or(10)).await,
            ReadMode::Tail => read_tail(&authorized_path, args.limit.unwrap_or(10)).await,
            ReadMode::Stat => read_stat(&authorized_path).await,
            ReadMode::List => read_list(&authorized_path).await,
            ReadMode::Batch => read_batch(&authorized_path).await,
        };

        result.map_err(|e| ToolError::ExecutionFailed(e.to_string()))
    }
}

async fn read_text(path: &std::path::Path) -> Result<ToolResult, std::io::Error> {
    let content = tokio::fs::read_to_string(path).await?;
    Ok(ToolResult::ok(json!({ "content": content, "path": path.to_string_lossy() })))
}

async fn read_lines(path: &std::path::Path, offset: Option<usize>, limit: Option<usize>) -> Result<ToolResult, std::io::Error> {
    let content = tokio::fs::read_to_string(path).await?;
    let lines: Vec<&str> = content.lines().collect();

    let start = offset.unwrap_or(0).min(lines.len());
    let end = limit.map(|l| (start + l).min(lines.len())).unwrap_or(lines.len());

    let selected: Vec<String> = lines[start..end].iter().map(|s| s.to_string()).collect();
    Ok(ToolResult::ok(json!({
        "lines": selected,
        "path": path.to_string_lossy(),
        "total_lines": lines.len(),
        "offset": start,
    })))
}

async fn read_head(path: &std::path::Path, n: usize) -> Result<ToolResult, std::io::Error> {
    read_lines(path, Some(0), Some(n)).await
}

async fn read_tail(path: &std::path::Path, n: usize) -> Result<ToolResult, std::io::Error> {
    let content = tokio::fs::read_to_string(path).await?;
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(n);
    let selected: Vec<String> = lines[start..].iter().map(|s| s.to_string()).collect();
    Ok(ToolResult::ok(json!({
        "lines": selected,
        "path": path.to_string_lossy(),
        "total_lines": lines.len(),
    })))
}

async fn read_stat(path: &std::path::Path) -> Result<ToolResult, std::io::Error> {
    let metadata = tokio::fs::metadata(path).await?;
    Ok(ToolResult::ok(json!({
        "path": path.to_string_lossy(),
        "size": metadata.len(),
        "is_file": metadata.is_file(),
        "is_dir": metadata.is_dir(),
        "modified": metadata.modified().ok().map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()),
        "accessed": metadata.accessed().ok().map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()),
    })))
}

async fn read_list(path: &std::path::Path) -> Result<ToolResult, std::io::Error> {
    if !path.is_dir() {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "path is not a directory"));
    }
    let mut entries = Vec::new();
    let mut dir = tokio::fs::read_dir(path).await?;
    while let Some(entry) = dir.next_entry().await? {
        let file_type = entry.file_type().await?;
        entries.push(json!({
            "name": entry.file_name().to_string_lossy(),
            "is_dir": file_type.is_dir(),
            "is_file": file_type.is_file(),
        }));
    }
    Ok(ToolResult::ok(json!({
        "entries": entries,
        "path": path.to_string_lossy(),
        "count": entries.len(),
    })))
}

async fn read_batch(path: &std::path::Path) -> Result<ToolResult, std::io::Error> {
    // Batch mode: read multiple files if path is a directory, or single file
    if path.is_dir() {
        let mut results = Vec::new();
        let mut dir = tokio::fs::read_dir(path).await?;
        while let Some(entry) = dir.next_entry().await? {
            let entry_path = entry.path();
            if entry_path.is_file() {
                if let Ok(content) = tokio::fs::read_to_string(&entry_path).await {
                    results.push(json!({
                        "path": entry_path.to_string_lossy(),
                        "content": content,
                        "error": null,
                    }));
                } else {
                    results.push(json!({
                        "path": entry_path.to_string_lossy(),
                        "content": null,
                        "error": "failed to read",
                    }));
                }
            }
        }
        Ok(ToolResult::ok(json!({
            "results": results,
            "count": results.len(),
        })))
    } else {
        let content = tokio::fs::read_to_string(path).await?;
        Ok(ToolResult::ok(json!({
            "results": [{
                "path": path.to_string_lossy(),
                "content": content,
                "error": null,
            }],
            "count": 1,
        })))
    }
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
