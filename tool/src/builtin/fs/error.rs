use thiserror::Error;

#[derive(Error, Debug)]
pub enum FsError {
    #[error("Access denied: {0} - {1}")]
    AccessDenied(String, String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Invalid root: {0} - {1}")]
    InvalidRoot(String, String),

    #[error("IO error: {0} - {1}")]
    Io(String, String),

    #[error("Path not found: {0}")]
    NotFound(String),
}
