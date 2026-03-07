use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("git_access_denied: {0}")]
    AccessDenied(String),

    #[error("git_not_repo: {0}")]
    NotRepo(String),

    #[error("git_invalid_path: {0}")]
    InvalidPath(String),

    #[error("git_branch_exists: {0}")]
    BranchExists(String),

    #[error("git_branch_not_found: {0}")]
    BranchNotFound(String),

    #[error("git_dirty_worktree: {0}")]
    DirtyWorktree(String),

    #[error("git_unmerged_state: {0}")]
    UnmergedState(String),

    #[error("git_nothing_to_commit: {0}")]
    NothingToCommit(String),

    #[error("git_identity_missing: {0}")]
    IdentityMissing(String),

    #[error("git_auth_required: {0}")]
    AuthRequired(String),

    #[error("git_network: {0}")]
    Network(String),

    #[error("git_cancelled: {0}")]
    Cancelled(String),

    #[error("git_unsupported_state: {0}")]
    UnsupportedState(String),

    #[error("git_operation_failed: {0}")]
    OperationFailed(String),
}

impl From<GitError> for crate::error::ToolError {
    fn from(e: GitError) -> Self {
        crate::error::ToolError::ExecutionFailed(e.to_string())
    }
}

impl From<git2::Error> for GitError {
    fn from(e: git2::Error) -> Self {
        GitError::OperationFailed(e.to_string())
    }
}
