use async_trait::async_trait;
use globset::{Glob, GlobMatcher};
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use walkdir::WalkDir;

use crate::builtin::fs::error::FsError;
use crate::builtin::fs::guard::FsGuard;
use crate::context::{ToolContext, ToolResult};
use crate::error::ToolError;
use crate::spec::ToolSpec;
use crate::trait_def::Tool;

pub struct GlobTool {
    guard: FsGuard,
}

impl GlobTool {
    pub fn new(allowed_roots: Vec<PathBuf>) -> Result<Self, FsError> {
        let guard = FsGuard::new(allowed_roots)?;
        Ok(Self { guard })
    }

    /// Build a glob tool with the current directory as the allowed root.
    pub fn from_current_dir() -> Result<Self, FsError> {
        Self::new(vec![std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))])
    }
}

#[derive(Debug, Deserialize)]
struct GlobArgs {
    path: String,
    pattern: Option<String>,
    #[serde(default = "default_max_depth")]
    max_depth: Option<usize>,
    #[serde(default = "default_max_results")]
    max_results: Option<usize>,
    #[serde(default)]
    include: Option<String>,
    #[serde(default)]
    exclude: Option<String>,
    #[serde(default)]
    min_size: Option<u64>,
    #[serde(default)]
    max_size: Option<u64>,
}

fn default_max_depth() -> Option<usize> {
    Some(10)
}

fn default_max_results() -> Option<usize> {
    Some(100)
}

fn compile_glob(pattern: &str) -> Result<GlobMatcher, ToolError> {
    let glob = Glob::new(pattern)
        .map_err(|e| ToolError::InvalidArgs(format!("invalid pattern: {}", e)))?;
    Ok(glob.compile_matcher())
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "Find files by pattern with filters and limits"
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
                        "description": "Base directory to search"
                    },
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern (e.g., '*.rs', '**/*.txt')"
                    },
                    "max_depth": {
                        "type": "integer",
                        "description": "Maximum directory depth to traverse"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of results to return"
                    },
                    "include": {
                        "type": "string",
                        "description": "Additional include pattern"
                    },
                    "exclude": {
                        "type": "string",
                        "description": "Exclude pattern"
                    },
                    "min_size": {
                        "type": "integer",
                        "description": "Minimum file size in bytes"
                    },
                    "max_size": {
                        "type": "integer",
                        "description": "Maximum file size in bytes"
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
        let args: GlobArgs =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        // Authorize the base path
        let authorized_path = self
            .guard
            .authorize_existing(&args.path)
            .await
            .map_err(map_fs_error)?;

        if !authorized_path.is_dir() {
            return Err(ToolError::ExecutionFailed(
                "path is not a directory".to_string(),
            ));
        }

        // Compile glob patterns
        let glob = args.pattern.as_ref().map(|p| compile_glob(p)).transpose()?;
        let include_glob = args.include.as_ref().map(|p| compile_glob(p)).transpose()?;
        let exclude_glob = args.exclude.as_ref().map(|p| compile_glob(p)).transpose()?;

        let max_depth = args.max_depth.unwrap_or(10);
        let max_results = args.max_results.unwrap_or(100);
        let min_size = args.min_size;
        let max_size = args.max_size;

        let mut results = Vec::new();

        for entry in WalkDir::new(&authorized_path)
            .max_depth(max_depth)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if results.len() >= max_results {
                break;
            }

            let path = entry.path();

            // Skip directories
            if path.is_dir() {
                continue;
            }

            // Get relative path from base directory for proper pattern matching
            let relative_path = path
                .strip_prefix(&authorized_path)
                .map(|p| p.to_string_lossy().to_string())
                // Fallback to just filename if strip fails
                .unwrap_or_else(|_| {
                    path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                });

            // Check glob pattern against relative path
            if let Some(ref g) = glob {
                // Try matching relative path first, then filename as fallback
                if !g.is_match(&relative_path)
                    && !g.is_match(path.file_name().unwrap_or_default().to_str().unwrap_or(""))
                {
                    continue;
                }
            }

            // Check include pattern against relative path
            if let Some(ref inc) = include_glob
                && !inc.is_match(&relative_path)
                && !inc.is_match(path.file_name().unwrap_or_default().to_str().unwrap_or(""))
            {
                continue;
            }

            // Check exclude pattern against relative path
            if let Some(ref exc) = exclude_glob
                && (exc.is_match(&relative_path)
                    || exc.is_match(path.file_name().unwrap_or_default().to_str().unwrap_or("")))
            {
                continue;
            }

            // Check file size
            if let Ok(metadata) = path.metadata() {
                let size = metadata.len();
                if let Some(min) = min_size
                    && size < min
                {
                    continue;
                }
                if let Some(max) = max_size
                    && size > max
                {
                    continue;
                }
            }

            results.push(json!({
                "path": path.to_string_lossy(),
                "name": path.file_name().map(|n| n.to_string_lossy().to_string()),
            }));
        }

        Ok(ToolResult::ok(json!({
            "results": results,
            "count": results.len(),
            "truncated": results.len() >= max_results,
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
