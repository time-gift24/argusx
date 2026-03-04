use std::path::{Path, PathBuf};
use std::fs;
use crate::builtin::fs::error::FsError;

pub struct FsGuard {
    allowed_roots: Vec<PathBuf>,
}

impl FsGuard {
    pub fn new(allowed_roots: Vec<PathBuf>) -> Self {
        // Normalize paths to absolute form
        let roots: Vec<PathBuf> = allowed_roots
            .into_iter()
            .map(|p| {
                if p.is_relative() {
                    std::env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .join(p)
                } else {
                    p
                }
            })
            .map(|p| fs::canonicalize(&p).unwrap_or(p))
            .collect();
        Self { allowed_roots: roots }
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

        // Resolve symlinks to get real path
        let real_path = fs::canonicalize(&abs_path)
            .map_err(|e| FsError::AccessDenied(path.to_string(), e.to_string()))?;

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

    /// Authorize a potentially new path - checks parent directory
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

        // Get parent directory
        let parent = abs_path.parent()
            .ok_or_else(|| FsError::InvalidPath(path.to_string()))?;

        // Resolve parent symlinks to get real parent path
        let real_parent = fs::canonicalize(parent)
            .map_err(|e| FsError::AccessDenied(path.to_string(), e.to_string()))?;

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
