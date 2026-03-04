use thiserror::Error;

#[derive(Error, Debug)]
pub enum FsError {
    #[error("Access denied: {0} - {1}")]
    AccessDenied(String, String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Path not found: {0}")]
    NotFound(String),
}
