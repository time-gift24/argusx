use std::path::{Path, PathBuf};
use std::fs;
use std::io;
use crate::builtin::fs::error::FsError;

pub struct FsGuard {
    allowed_roots: Vec<PathBuf>,
}

impl FsGuard {
    /// Create a new FsGuard with the given allowed roots.
    /// Returns an error if any root cannot be canonicalized.
    pub fn new(allowed_roots: Vec<PathBuf>) -> Result<Self, FsError> {
        // Normalize paths to absolute form
        let mut roots = Vec::new();
        for p in allowed_roots {
            let abs_path = if p.is_relative() {
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(p)
            } else {
                p
            };

            // Canonicalize and fail if it doesn't exist or can't be resolved
            let canonical = fs::canonicalize(&abs_path)
                .map_err(|e| FsError::InvalidRoot(abs_path.to_string_lossy().to_string(), e.to_string()))?;
            roots.push(canonical);
        }

        if roots.is_empty() {
            return Err(FsError::InvalidRoot(String::new(), "no valid roots provided".to_string()));
        }

        Ok(Self { allowed_roots: roots })
    }

    /// Authorize an existing path - returns the resolved path or error
    pub async fn authorize_existing(&self, path: &str) -> Result<PathBuf, FsError> {
        let input_path = Path::new(path);

        // Convert to absolute if relative
        let abs_path = if input_path.is_relative() {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(input_path)
        } else {
            input_path.to_path_buf()
        };

        // Check if file exists
        if !abs_path.exists() {
            return Err(FsError::NotFound(path.to_string()));
        }

        // Resolve symlinks to get real path
        let real_path = fs::canonicalize(&abs_path)
            .map_err(|e| map_io_error(&e, path))?;

        // Check if real_path is within allowed roots
        for root in &self.allowed_roots {
            if real_path.starts_with(root) {
                return Ok(real_path);
            }
        }

        Err(FsError::AccessDenied(
            path.to_string(),
            format!("path {:?} is outside allowed roots", real_path),
        ))
    }

    /// Authorize a potentially new path - checks parent directory and handles existing symlinks
    pub async fn authorize_maybe_new(&self, path: &str) -> Result<PathBuf, FsError> {
        let input_path = Path::new(path);

        // Convert to absolute if relative
        let abs_path = if input_path.is_relative() {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(input_path)
        } else {
            input_path.to_path_buf()
        };

        // If the target already exists, treat it as an existing file authorization
        // This prevents symlink escape: if target is a symlink to outside, it will be denied
        if abs_path.exists() {
            return self.authorize_existing(path).await;
        }

        // Get parent directory
        let parent = abs_path.parent()
            .ok_or_else(|| FsError::InvalidPath(path.to_string()))?;

        // Resolve parent symlinks to get real parent path
        let real_parent = fs::canonicalize(parent)
            .map_err(|e| map_io_error(&e, path))?;

        // Check if real parent is within allowed roots
        for root in &self.allowed_roots {
            if real_parent.starts_with(root) {
                return Ok(abs_path);
            }
        }

        Err(FsError::AccessDenied(
            path.to_string(),
            format!("parent directory {:?} is outside allowed roots", real_parent),
        ))
    }
}

/// Map IO errors to FsError based on error kind
fn map_io_error(e: &io::Error, path: &str) -> FsError {
    match e.kind() {
        io::ErrorKind::NotFound => FsError::NotFound(path.to_string()),
        io::ErrorKind::PermissionDenied => FsError::AccessDenied(path.to_string(), e.to_string()),
        _ => FsError::Io(path.to_string(), e.to_string()),
    }
}
