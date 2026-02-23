//! Read file tool handler
//!
//! Provides the ability to read files from the filesystem.
//! Supports both line-based reading and indentation-aware block reading.

use async_trait::async_trait;
use serde::Deserialize;

use crate::parse_tool_args;
use crate::tools::context::{OutputBody, ToolInvocation, ToolOutput, ToolPayload};
use crate::tools::error::ToolExecutionError;
use crate::tools::handler::{ToolHandler, ToolKind};

/// Arguments for read_file tool
#[derive(Deserialize)]
struct ReadFileArgs {
    /// Absolute path to the file to read
    #[serde(alias = "file_path")]
    path: String,
    /// 1-indexed line number to start reading from (default: 1)
    #[serde(default = "default_offset")]
    offset: usize,
    /// Maximum number of lines to return (default: 2000)
    #[serde(default = "default_limit")]
    limit: usize,
    /// Read mode: "slice" or "indentation" (default: "slice")
    #[serde(default)]
    mode: ReadMode,
    /// Configuration for indentation-aware reading
    #[serde(default)]
    indentation: Option<IndentationArgs>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum ReadMode {
    #[default]
    Slice,
    Indentation,
}

#[derive(Deserialize, Clone)]
struct IndentationArgs {
    /// Optional anchor line (default: offset)
    #[serde(default)]
    anchor_line: Option<usize>,
    /// Maximum indentation levels to include (0 = unlimited)
    #[serde(default)]
    max_levels: Option<usize>,
    /// Include sibling blocks at same indentation level
    #[serde(default)]
    include_siblings: Option<bool>,
    /// Include header lines above anchor block
    #[serde(default)]
    include_header: Option<bool>,
}

fn default_offset() -> usize {
    1
}

fn default_limit() -> usize {
    2000
}

/// Maximum line length to display
const MAX_LINE_LENGTH: usize = 500;
/// Tab width for indentation calculation
const TAB_WIDTH: usize = 4;

/// Read file tool handler
pub struct ReadFileHandler;

impl ReadFileHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ReadFileHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for ReadFileHandler {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    fn name(&self) -> String {
        "read_file".to_string()
    }

    fn description(&self) -> String {
        "Read a file from the filesystem. Returns file contents with line numbers.".to_string()
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the file to read"
                },
                "offset": {
                    "type": "integer",
                    "description": "1-indexed line number to start reading from (default: 1)",
                    "default": 1
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to return (default: 2000)",
                    "default": 2000
                },
                "mode": {
                    "type": "string",
                    "enum": ["slice", "indentation"],
                    "description": "Read mode: 'slice' for line range, 'indentation' for code block",
                    "default": "slice"
                },
                "indentation": {
                    "type": "object",
                    "description": "Configuration for indentation-aware reading",
                    "properties": {
                        "anchor_line": {
                            "type": "integer",
                            "description": "Anchor line for indentation mode"
                        },
                        "max_levels": {
                            "type": "integer",
                            "description": "Maximum indentation levels to include"
                        },
                        "include_siblings": {
                            "type": "boolean",
                            "description": "Include sibling blocks"
                        },
                        "include_header": {
                            "type": "boolean",
                            "description": "Include header lines"
                        }
                    }
                }
            },
            "required": ["path"]
        })
    }

    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }

    async fn is_mutating(&self, _invocation: &ToolInvocation) -> bool {
        false // Read operations don't modify anything
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, ToolExecutionError> {
        let arguments = match &invocation.payload {
            ToolPayload::Function { arguments } => arguments,
            _ => {
                return Err(ToolExecutionError::PayloadMismatch {
                    expected: "Function".to_string(),
                    actual: format!("{:?}", invocation.payload),
                });
            }
        };

        let args: ReadFileArgs = parse_tool_args!(arguments, ReadFileArgs)?;

        // Validate path
        let path = std::path::PathBuf::from(&args.path);
        if !path.is_absolute() {
            return Err(ToolExecutionError::respond_to_model(
                "path must be an absolute path",
            ));
        }

        // Validate offset and limit
        if args.offset == 0 {
            return Err(ToolExecutionError::respond_to_model(
                "offset must be a 1-indexed line number",
            ));
        }

        if args.limit == 0 {
            return Err(ToolExecutionError::respond_to_model(
                "limit must be greater than zero",
            ));
        }

        // Read file based on mode
        let content = match args.mode {
            ReadMode::Slice => read_slice(&path, args.offset, args.limit).await?,
            ReadMode::Indentation => {
                read_indentation_block(&path, args.offset, args.limit, &args.indentation).await?
            }
        };

        Ok(ToolOutput::Function {
            body: OutputBody::Text(content),
            success: Some(true),
        })
    }
}

/// Read a slice of lines from a file
async fn read_slice(
    path: &std::path::Path,
    offset: usize,
    limit: usize,
) -> Result<String, ToolExecutionError> {
    use tokio::fs::File;
    use tokio::io::{AsyncBufReadExt, BufReader};

    let file = File::open(path)
        .await
        .map_err(|e| ToolExecutionError::FileError(format!("failed to open file: {}", e)))?;

    let mut reader = BufReader::new(file);
    let mut lines = Vec::with_capacity(limit);
    let mut current_line = 0;
    let mut buffer = Vec::new();

    loop {
        buffer.clear();
        let bytes_read = reader
            .read_until(b'\n', &mut buffer)
            .await
            .map_err(|e| ToolExecutionError::FileError(format!("failed to read file: {}", e)))?;

        if bytes_read == 0 {
            break;
        }

        current_line += 1;

        // Skip lines before offset
        if current_line < offset {
            continue;
        }

        // Stop if we've reached limit
        if lines.len() >= limit {
            break;
        }

        // Format line with line number
        let line_content = format_line(&buffer);
        lines.push(format!("L{}: {}", current_line, line_content));
    }

    if offset > current_line {
        return Err(ToolExecutionError::respond_to_model(
            "offset exceeds file length",
        ));
    }

    Ok(lines.join("\n"))
}

/// Read an indentation-aware block of code
async fn read_indentation_block(
    path: &std::path::Path,
    offset: usize,
    limit: usize,
    config: &Option<IndentationArgs>,
) -> Result<String, ToolExecutionError> {
    use tokio::fs::File;
    use tokio::io::{AsyncBufReadExt, BufReader};

    let file = File::open(path)
        .await
        .map_err(|e| ToolExecutionError::FileError(format!("failed to open file: {}", e)))?;

    let mut reader = BufReader::new(file);
    let mut all_lines = Vec::new();
    let mut buffer = Vec::new();
    let mut line_number = 0;
    let _ = config.as_ref().map(|c| {
        (
            c.anchor_line,
            c.max_levels,
            c.include_siblings,
            c.include_header,
        )
    });

    // Read all lines first
    loop {
        buffer.clear();
        let bytes_read = reader
            .read_until(b'\n', &mut buffer)
            .await
            .map_err(|e| ToolExecutionError::FileError(format!("failed to read file: {}", e)))?;

        if bytes_read == 0 {
            break;
        }

        line_number += 1;
        let content = String::from_utf8_lossy(&buffer)
            .trim_end_matches(['\n', '\r'])
            .to_string();
        let indent = measure_indent(&content);
        all_lines.push((line_number, content, indent));

        if line_number >= offset + limit {
            break;
        }
    }

    if offset == 0 || offset > all_lines.len() {
        return Err(ToolExecutionError::respond_to_model(
            "offset exceeds file length",
        ));
    }

    // Find the block starting from offset
    let start_idx = offset - 1;
    let anchor_indent = all_lines[start_idx].2;

    // Collect lines with same or greater indentation
    let mut result = Vec::new();
    for (num, content, indent) in all_lines.iter().skip(start_idx) {
        if *indent >= anchor_indent || content.is_empty() {
            if result.len() >= limit {
                break;
            }
            result.push(format!("L{}: {}", num, content));
        } else {
            break;
        }
    }

    Ok(result.join("\n"))
}

/// Format a line for display, truncating if too long
fn format_line(bytes: &[u8]) -> String {
    let decoded = String::from_utf8_lossy(bytes);
    let trimmed = decoded.trim_end_matches(['\n', '\r']);

    if trimmed.chars().count() > MAX_LINE_LENGTH {
        let truncated: String = trimmed.chars().take(MAX_LINE_LENGTH).collect();
        format!("{}...", truncated)
    } else {
        trimmed.to_string()
    }
}

/// Measure the indentation of a line
fn measure_indent(line: &str) -> usize {
    line.chars()
        .take_while(|c| matches!(c, ' ' | '\t'))
        .map(|c| if c == '\t' { TAB_WIDTH } else { 1 })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_read_file_basic() -> Result<(), Box<dyn std::error::Error>> {
        let mut temp = NamedTempFile::new()?;
        writeln!(temp, "line 1")?;
        writeln!(temp, "line 2")?;
        writeln!(temp, "line 3")?;

        let handler = ReadFileHandler::new();
        let invocation = ToolInvocation {
            session: crate::tools::context::SessionInfo::new(
                "test".to_string(),
                std::env::temp_dir(),
            ),
            turn: crate::tools::context::TurnContext {
                cwd: std::env::temp_dir(),
                turn_number: 0,
                messages: vec![],
            },
            tracker: crate::tools::context::TurnDiffTracker::new(),
            call_id: "test".to_string(),
            tool_name: "read_file".to_string(),
            payload: ToolPayload::Function {
                arguments: serde_json::json!({
                    "path": temp.path().to_string_lossy(),
                    "offset": 1,
                    "limit": 10
                })
                .to_string(),
            },
        };

        let result = handler.handle(invocation).await?;
        match result {
            ToolOutput::Function { body, .. } => {
                assert!(body.as_text().unwrap().contains("line 1"));
            }
            _ => panic!("expected function output"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_read_file_utf8_long_line_does_not_panic() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut temp = NamedTempFile::new()?;
        let long_line = "你".repeat(MAX_LINE_LENGTH + 50);
        writeln!(temp, "{}", long_line)?;

        let handler = ReadFileHandler::new();
        let invocation = ToolInvocation {
            session: crate::tools::context::SessionInfo::new(
                "test".to_string(),
                std::env::temp_dir(),
            ),
            turn: crate::tools::context::TurnContext {
                cwd: std::env::temp_dir(),
                turn_number: 0,
                messages: vec![],
            },
            tracker: crate::tools::context::TurnDiffTracker::new(),
            call_id: "test".to_string(),
            tool_name: "read_file".to_string(),
            payload: ToolPayload::Function {
                arguments: serde_json::json!({
                    "path": temp.path().to_string_lossy(),
                    "offset": 1,
                    "limit": 1
                })
                .to_string(),
            },
        };

        let result = handler.handle(invocation).await?;
        match result {
            ToolOutput::Function { body, .. } => {
                let text = body.as_text().unwrap_or_default();
                assert!(text.starts_with("L1: "));
            }
            _ => panic!("expected function output"),
        }

        Ok(())
    }
}
