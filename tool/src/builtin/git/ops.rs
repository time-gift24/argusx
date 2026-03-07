use super::error::GitError;
use super::types::GitArgs;
use super::guard::GitGuard;
use crate::context::ToolResult;
use git2::{Repository, StatusOptions, BranchType, ErrorCode};
use serde_json::json;

pub async fn execute(
    guard: &GitGuard,
    args: GitArgs,
) -> Result<ToolResult, GitError> {
    match args {
        GitArgs::Status { repo_path, include_untracked } => {
            status(guard, &repo_path, include_untracked).await
        }
        GitArgs::Diff { repo_path, staged, revision_range, paths, max_bytes } => {
            diff(guard, &repo_path, staged, revision_range.as_deref(), &paths, max_bytes).await
        }
        GitArgs::Log { repo_path, max_count, revision_range, oneline } => {
            log(guard, &repo_path, max_count, revision_range.as_deref(), oneline).await
        }
        GitArgs::Show { repo_path, object, max_bytes } => {
            show(guard, &repo_path, &object, max_bytes).await
        }
        GitArgs::BranchList { repo_path } => {
            branch_list(guard, &repo_path).await
        }
        GitArgs::RemoteList { repo_path } => {
            remote_list(guard, &repo_path).await
        }
        GitArgs::WorktreeList { repo_path } => {
            worktree_list(guard, &repo_path).await
        }
        GitArgs::Add { repo_path, paths } => {
            add(guard, &repo_path, &paths).await
        }
        GitArgs::Commit { repo_path, message, allow_empty } => {
            commit(guard, &repo_path, &message, allow_empty).await
        }
        GitArgs::BranchCreate { repo_path, branch, start_point, checkout } => {
            branch_create(guard, &repo_path, &branch, start_point.as_deref(), checkout).await
        }
        GitArgs::Checkout { repo_path, branch } => {
            checkout(guard, &repo_path, &branch).await
        }
        GitArgs::Clone { url, target_path, branch } => {
            clone(guard, &url, &target_path, branch.as_deref()).await
        }
        GitArgs::Fetch { repo_path, remote, prune } => {
            fetch(guard, &repo_path, &remote, prune).await
        }
    }
}

// ============================================================================
// Status Action
// ============================================================================

async fn status(
    guard: &GitGuard,
    repo_path: &str,
    include_untracked: bool,
) -> Result<ToolResult, GitError> {
    let (path, repo) = guard.authorize_repo(repo_path).await?;

    let mut options = StatusOptions::new();
    options
        .include_untracked(include_untracked)
        .include_ignored(false)
        .recurse_untracked_dirs(true)
        .show(git2::StatusShow::Workdir);

    let statuses = repo.statuses(Some(&mut options))?;

    let mut files = Vec::new();
    for entry in statuses.iter() {
        let status = entry.status();
        let path_str = entry.path().unwrap_or("unknown").to_string();

        // CURRENT has value 0, so we check if status is empty
        let status_type = if status.is_empty() {
            continue;
        } else if status.contains(git2::Status::INDEX_NEW) {
            "added"
        } else if status.contains(git2::Status::INDEX_MODIFIED) {
            "staged"
        } else if status.contains(git2::Status::INDEX_DELETED) {
            "staged_deleted"
        } else if status.contains(git2::Status::WT_NEW) {
            "untracked"
        } else if status.contains(git2::Status::WT_MODIFIED) {
            "modified"
        } else if status.contains(git2::Status::WT_DELETED) {
            "deleted"
        } else if status.contains(git2::Status::CONFLICTED) {
            "conflicted"
        } else {
            "other"
        };

        files.push(json!({
            "path": path_str,
            "status": status_type
        }));
    }

    let is_clean = files.is_empty();
    let current_branch = get_current_branch(&repo)?;

    Ok(ToolResult::ok(json!({
        "action": "status",
        "repo_path": path.to_string_lossy(),
        "data": {
            "files": files,
            "current_branch": current_branch,
            "is_clean": is_clean
        },
        "warnings": []
    })))
}

// ============================================================================
// Branch List Action
// ============================================================================

async fn branch_list(
    guard: &GitGuard,
    repo_path: &str,
) -> Result<ToolResult, GitError> {
    let (path, repo) = guard.authorize_repo(repo_path).await?;

    let mut branches = Vec::new();
    let head = get_current_branch(&repo)?;

    for branch_result in repo.branches(Some(BranchType::Local))? {
        let (branch, branch_type) = branch_result?;
        if branch_type != BranchType::Local {
            continue;
        }

        let name = branch.name()?.unwrap_or("unknown").to_string();
        let is_head = branch.is_head();

        // Get commit info
        let (commit_id, summary) = if let Ok(commit) = branch.get().peel_to_commit() {
            let id = commit.id().to_string();
            let summary = commit.summary().unwrap_or("").to_string();
            (Some(id), Some(summary))
        } else {
            (None, None)
        };

        branches.push(json!({
            "name": name,
            "is_head": is_head,
            "commit_id": commit_id,
            "summary": summary
        }));
    }

    Ok(ToolResult::ok(json!({
        "action": "branch_list",
        "repo_path": path.to_string_lossy(),
        "data": {
            "branches": branches,
            "head": head
        },
        "warnings": []
    })))
}

// ============================================================================
// Remote List Action
// ============================================================================

async fn remote_list(
    guard: &GitGuard,
    repo_path: &str,
) -> Result<ToolResult, GitError> {
    let (path, repo) = guard.authorize_repo(repo_path).await?;

    let mut remotes = Vec::new();

    for remote_name in repo.remotes()?.iter() {
        if let Some(name) = remote_name {
            let url = repo.find_remote(name)?
                .url()
                .map(|s| s.to_string());

            remotes.push(json!({
                "name": name,
                "url": url
            }));
        }
    }

    Ok(ToolResult::ok(json!({
        "action": "remote_list",
        "repo_path": path.to_string_lossy(),
        "data": {
            "remotes": remotes
        },
        "warnings": []
    })))
}

// ============================================================================
// Worktree List Action
// ============================================================================

async fn worktree_list(
    guard: &GitGuard,
    repo_path: &str,
) -> Result<ToolResult, GitError> {
    let (path, repo) = guard.authorize_repo(repo_path).await?;

    let mut worktrees = Vec::new();

    // For non-bare repos, the main worktree is the repo path itself
    if !repo.is_bare() {
        let head = get_current_branch(&repo)?;
        worktrees.push(json!({
            "path": path.to_string_lossy(),
            "is_main": true,
            "head": head,
            "branch": head
        }));
    }

    // List linked worktrees
    match repo.worktrees() {
        Ok(wts) => {
            for wt_name in wts.iter() {
                if let Some(name) = wt_name {
                    if let Ok(wt) = repo.find_worktree(name) {
                        let wt_path = wt.path().to_string_lossy().to_string();
                        worktrees.push(json!({
                            "path": wt_path,
                            "is_main": false,
                            "head": name,
                            "branch": name
                        }));
                    }
                }
            }
        }
        Err(e) if e.code() == ErrorCode::BareRepo => {
            // Bare repos don't have worktrees in the traditional sense
        }
        Err(e) => return Err(GitError::OperationFailed(e.to_string())),
    }

    Ok(ToolResult::ok(json!({
        "action": "worktree_list",
        "repo_path": path.to_string_lossy(),
        "data": {
            "worktrees": worktrees
        },
        "warnings": []
    })))
}

// ============================================================================
// Helper Functions
// ============================================================================

fn get_current_branch(repo: &Repository) -> Result<Option<String>, GitError> {
    let head = repo.head().ok();

    if let Some(head_ref) = head {
        let is_detached = head_ref.target().is_some() && head_ref.shorthand().is_none();

        if is_detached {
            if let Some(target) = head_ref.target() {
                return Ok(Some(format!("detached at {}", target)));
            }
        }

        Ok(head_ref.shorthand().map(|s| s.to_string()))
    } else {
        Ok(None)
    }
}

// ============================================================================
// Placeholder Actions (implemented in later tasks)
// ============================================================================

async fn diff(
    guard: &GitGuard,
    repo_path: &str,
    _staged: bool,
    _revision_range: Option<&str>,
    _paths: &[String],
    _max_bytes: usize,
) -> Result<ToolResult, GitError> {
    let (path, _repo) = guard.authorize_repo(repo_path).await?;
    // Placeholder - actual implementation in Task 6
    Ok(ToolResult::ok(json!({
        "action": "diff",
        "repo_path": path.to_string_lossy(),
        "data": {
            "patch": "",
            "stats": { "files_changed": 0, "insertions": 0, "deletions": 0 },
            "truncated": false
        },
        "warnings": []
    })))
}

async fn log(
    guard: &GitGuard,
    repo_path: &str,
    _max_count: usize,
    _revision_range: Option<&str>,
    _oneline: bool,
) -> Result<ToolResult, GitError> {
    let (path, _repo) = guard.authorize_repo(repo_path).await?;
    // Placeholder - actual implementation in Task 6
    Ok(ToolResult::ok(json!({
        "action": "log",
        "repo_path": path.to_string_lossy(),
        "data": {
            "commits": [],
            "truncated": false
        },
        "warnings": []
    })))
}

async fn show(
    guard: &GitGuard,
    repo_path: &str,
    _object: &str,
    _max_bytes: usize,
) -> Result<ToolResult, GitError> {
    let (path, _repo) = guard.authorize_repo(repo_path).await?;
    // Placeholder - actual implementation in Task 6
    Ok(ToolResult::ok(json!({
        "action": "show",
        "repo_path": path.to_string_lossy(),
        "data": {
            "object": null,
            "patch": "",
            "truncated": false
        },
        "warnings": []
    })))
}

async fn add(
    guard: &GitGuard,
    repo_path: &str,
    paths: &[String],
) -> Result<ToolResult, GitError> {
    let (path, repo) = guard.authorize_repo(repo_path).await?;
    let _validated = guard.validate_repo_relative_paths(&repo, paths)?;
    // Placeholder - actual implementation in Task 7
    Ok(ToolResult::ok(json!({
        "action": "add",
        "repo_path": path.to_string_lossy(),
        "data": {
            "staged_paths": paths
        },
        "warnings": []
    })))
}

async fn commit(
    guard: &GitGuard,
    repo_path: &str,
    _message: &str,
    _allow_empty: bool,
) -> Result<ToolResult, GitError> {
    let (path, _repo) = guard.authorize_repo(repo_path).await?;
    // Placeholder - actual implementation in Task 7
    Ok(ToolResult::ok(json!({
        "action": "commit",
        "repo_path": path.to_string_lossy(),
        "data": {
            "commit_id": "placeholder",
            "summary": ""
        },
        "warnings": []
    })))
}

async fn branch_create(
    guard: &GitGuard,
    repo_path: &str,
    _branch: &str,
    _start_point: Option<&str>,
    _checkout: bool,
) -> Result<ToolResult, GitError> {
    let (path, _repo) = guard.authorize_repo(repo_path).await?;
    // Placeholder - actual implementation in Task 8
    Ok(ToolResult::ok(json!({
        "action": "branch_create",
        "repo_path": path.to_string_lossy(),
        "data": {
            "branch": null,
            "checked_out": false
        },
        "warnings": []
    })))
}

async fn checkout(
    guard: &GitGuard,
    repo_path: &str,
    _branch: &str,
) -> Result<ToolResult, GitError> {
    let (path, _repo) = guard.authorize_repo(repo_path).await?;
    // Placeholder - actual implementation in Task 8
    Ok(ToolResult::ok(json!({
        "action": "checkout",
        "repo_path": path.to_string_lossy(),
        "data": {
            "branch": null,
            "head": null
        },
        "warnings": []
    })))
}

async fn clone(
    guard: &GitGuard,
    _url: &str,
    target_path: &str,
    _branch: Option<&str>,
) -> Result<ToolResult, GitError> {
    let _path = guard.authorize_clone_target(target_path).await?;
    // Placeholder - actual implementation in Task 9
    Ok(ToolResult::ok(json!({
        "action": "clone",
        "repo_path": target_path,
        "data": {
            "repo_path": target_path,
            "head": null,
            "remote": null
        },
        "warnings": []
    })))
}

async fn fetch(
    guard: &GitGuard,
    repo_path: &str,
    _remote: &str,
    _prune: bool,
) -> Result<ToolResult, GitError> {
    let (path, _repo) = guard.authorize_repo(repo_path).await?;
    // Placeholder - actual implementation in Task 9
    Ok(ToolResult::ok(json!({
        "action": "fetch",
        "repo_path": path.to_string_lossy(),
        "data": {
            "remote": null,
            "updated_refs": [],
            "pruned_refs": []
        },
        "warnings": []
    })))
}
