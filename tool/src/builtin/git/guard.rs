use super::error::GitError;
use crate::builtin::fs::error::FsError;
use crate::builtin::fs::guard::FsGuard;
use git2::Repository;
use std::path::{Path, PathBuf};

pub struct GitGuard {
    fs: FsGuard,
}

impl GitGuard {
    pub fn new(allowed_roots: Vec<PathBuf>) -> Result<Self, FsError> {
        Ok(Self {
            fs: FsGuard::new(allowed_roots)?,
        })
    }

    pub async fn authorize_repo(&self, repo_path: &str) -> Result<(PathBuf, Repository), GitError> {
        let path = self
            .fs
            .authorize_existing(repo_path)
            .await
            .map_err(|e| GitError::AccessDenied(e.to_string()))?;

        let repo = Repository::open_ext(
            &path,
            git2::RepositoryOpenFlags::NO_SEARCH,
            Vec::<&str>::new(),
        )
        .map_err(|_| GitError::NotRepo(repo_path.to_string()))?;

        Ok((path, repo))
    }

    pub async fn authorize_clone_target(&self, target_path: &str) -> Result<PathBuf, GitError> {
        let path = self
            .fs
            .authorize_maybe_new(target_path)
            .await
            .map_err(|e| GitError::AccessDenied(e.to_string()))?;

        // If path exists, it must be an empty directory
        if path.exists() {
            if !path.is_dir() {
                return Err(GitError::InvalidPath(
                    "clone target must be a directory".to_string(),
                ));
            }
            let is_empty = std::fs::read_dir(&path)
                .map_err(|e| GitError::InvalidPath(e.to_string()))?
                .next()
                .is_none();
            if !is_empty {
                return Err(GitError::InvalidPath(
                    "clone target directory is not empty".to_string(),
                ));
            }
        }

        Ok(path)
    }

    pub fn validate_repo_relative_paths(
        &self,
        _repo: &Repository,
        paths: &[String],
    ) -> Result<Vec<PathBuf>, GitError> {
        let mut validated = Vec::new();
        for p in paths {
            let path = Path::new(p);

            // Reject absolute paths
            if path.is_absolute() {
                return Err(GitError::InvalidPath(format!(
                    "absolute path not allowed: {}",
                    p
                )));
            }

            // Reject paths containing ..
            if path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                return Err(GitError::InvalidPath(format!(
                    "parent directory traversal not allowed: {}",
                    p
                )));
            }

            validated.push(path.to_path_buf());
        }
        Ok(validated)
    }
}
