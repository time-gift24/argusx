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
// History Actions (Task 6)
// ============================================================================

async fn diff(
    guard: &GitGuard,
    repo_path: &str,
    staged: bool,
    _revision_range: Option<&str>,
    _paths: &[String],
    max_bytes: usize,
) -> Result<ToolResult, GitError> {
    let (path, repo) = guard.authorize_repo(repo_path).await?;

    let mut diff_opts = git2::DiffOptions::new();
    diff_opts.include_untracked(false);

    let diff = if staged {
        // Diff between HEAD and index (staged changes)
        let head_tree = repo.head()
            .ok()
            .and_then(|h| h.target())
            .and_then(|oid| repo.find_commit(oid).ok())
            .and_then(|c| c.tree().ok());

        let head_tree = match head_tree {
            Some(t) => t,
            None => {
                // Empty tree for initial commit
                let empty_tree = repo.find_tree(repo.treebuilder(None)?.write()?)?;
                empty_tree
            }
        };

        repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut diff_opts))?
    } else {
        // Diff between HEAD tree and workdir (all changes including unstaged)
        let head_tree = repo.head()
            .ok()
            .and_then(|h| h.target())
            .and_then(|oid| repo.find_commit(oid).ok())
            .and_then(|c| c.tree().ok());

        match head_tree {
            Some(t) => repo.diff_tree_to_workdir(Some(&t), Some(&mut diff_opts))?,
            None => {
                // Empty repo - diff empty tree to workdir
                let empty_tree = repo.find_tree(repo.treebuilder(None)?.write()?)?;
                repo.diff_tree_to_workdir(Some(&empty_tree), Some(&mut diff_opts))?
            }
        }
    };

    // Generate patch string
    let mut patch = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let origin = line.origin();
        if origin == '+' || origin == '-' || origin == ' ' || origin == '\n' {
            if origin != '\n' {
                patch.push(origin);
            }
            if let Ok(content) = std::str::from_utf8(line.content()) {
                patch.push_str(content);
            }
        }
        true
    })?;

    // Get stats
    let stats = diff.stats()?;
    let (files_changed, insertions, deletions) = (stats.files_changed(), stats.insertions(), stats.deletions());

    // Truncate if needed
    let (patch, truncated) = truncate_string(patch, max_bytes);

    Ok(ToolResult::ok(json!({
        "action": "diff",
        "repo_path": path.to_string_lossy(),
        "data": {
            "patch": patch,
            "stats": {
                "files_changed": files_changed,
                "insertions": insertions,
                "deletions": deletions
            },
            "truncated": truncated
        },
        "warnings": []
    })))
}

async fn log(
    guard: &GitGuard,
    repo_path: &str,
    max_count: usize,
    _revision_range: Option<&str>,
    oneline: bool,
) -> Result<ToolResult, GitError> {
    let (path, repo) = guard.authorize_repo(repo_path).await?;

    // Cap max_count at 200
    let max_count = max_count.min(200).max(1);

    let mut commits = Vec::new();
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    let mut count = 0;
    for oid_result in revwalk {
        if count >= max_count {
            break;
        }

        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;

        let entry = if oneline {
            json!({
                "id": commit.id().to_string(),
                "summary": commit.summary().unwrap_or("")
            })
        } else {
            let author = commit.author();
            json!({
                "id": commit.id().to_string(),
                "summary": commit.summary().unwrap_or(""),
                "author": author.name().unwrap_or("unknown"),
                "email": author.email().unwrap_or(""),
                "time": commit.time().seconds()
            })
        };

        commits.push(entry);
        count += 1;
    }

    Ok(ToolResult::ok(json!({
        "action": "log",
        "repo_path": path.to_string_lossy(),
        "data": {
            "commits": commits,
            "truncated": false
        },
        "warnings": []
    })))
}

async fn show(
    guard: &GitGuard,
    repo_path: &str,
    object: &str,
    max_bytes: usize,
) -> Result<ToolResult, GitError> {
    let (path, repo) = guard.authorize_repo(repo_path).await?;

    // Try to parse as commit first
    let obj = repo.revparse_single(object)?;

    let (commit, tree) = if let Ok(commit) = obj.peel_to_commit() {
        let tree = commit.tree()?;
        (Some(commit), Some(tree))
    } else if let Ok(tree) = obj.peel_to_tree() {
        (None, Some(tree))
    } else {
        (None, None)
    };

    let mut patch = String::new();
    if let (Some(commit), Some(tree)) = (&commit, &tree) {
        // Get parent tree for diff
        let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

        let mut diff_opts = git2::DiffOptions::new();
        let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(tree), Some(&mut diff_opts))?;

        diff.print(git2::DiffFormat::Patch, |_, _, line| {
            match line.origin() {
                '+' | '-' | ' ' => {
                    patch.push(line.origin());
                    if let Ok(content) = std::str::from_utf8(line.content()) {
                        patch.push_str(content);
                    }
                }
                _ => {}
            }
            true
        })?;
    }

    let (patch, truncated) = truncate_string(patch, max_bytes);

    let object_info = if let Some(commit) = commit {
        let author = commit.author();
        json!({
            "id": commit.id().to_string(),
            "type": "commit",
            "summary": commit.summary().unwrap_or(""),
            "author": author.name().unwrap_or("unknown"),
            "email": author.email().unwrap_or(""),
            "time": commit.time().seconds()
        })
    } else if tree.is_some() {
        json!({
            "id": obj.id().to_string(),
            "type": "tree"
        })
    } else {
        json!({
            "id": obj.id().to_string(),
            "type": "unknown"
        })
    };

    Ok(ToolResult::ok(json!({
        "action": "show",
        "repo_path": path.to_string_lossy(),
        "data": {
            "object": object_info,
            "patch": patch,
            "truncated": truncated
        },
        "warnings": []
    })))
}

// ============================================================================
// Truncation Helper
// ============================================================================

fn truncate_string(input: String, max_bytes: usize) -> (String, bool) {
    if input.len() <= max_bytes {
        return (input, false);
    }

    // Find a valid UTF-8 boundary
    let mut end = max_bytes;
    while end > 0 && !input.is_char_boundary(end) {
        end -= 1;
    }

    (input[..end].to_string(), true)
}

// ============================================================================
// Write Actions (Task 7)
// ============================================================================

async fn add(
    guard: &GitGuard,
    repo_path: &str,
    paths: &[String],
) -> Result<ToolResult, GitError> {
    let (path, repo) = guard.authorize_repo(repo_path).await?;
    let validated_paths = guard.validate_repo_relative_paths(&repo, paths)?;

    let mut index = repo.index()?;
    let mut staged_paths = Vec::new();
    let mut warnings = Vec::new();

    for rel_path in validated_paths {
        let rel_str = rel_path.to_string_lossy().to_string();
        let abs_path = path.join(&rel_path);

        // Check if path exists
        if !abs_path.exists() {
            warnings.push(format!("path does not exist: {}", rel_str));
            continue;
        }

        // Add to index
        match index.add_path(&rel_path) {
            Ok(()) => staged_paths.push(rel_str),
            Err(e) => warnings.push(format!("failed to add {}: {}", rel_path.display(), e)),
        }
    }

    index.write()?;

    Ok(ToolResult::ok(json!({
        "action": "add",
        "repo_path": path.to_string_lossy(),
        "data": {
            "staged_paths": staged_paths
        },
        "warnings": warnings
    })))
}

async fn commit(
    guard: &GitGuard,
    repo_path: &str,
    message: &str,
    allow_empty: bool,
) -> Result<ToolResult, GitError> {
    let (path, repo) = guard.authorize_repo(repo_path).await?;

    // Get signature
    let sig = repo.signature()
        .map_err(|e| GitError::IdentityMissing(e.to_string()))?;

    // Get index tree
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    // Get parent commits
    let mut parents = Vec::new();
    if let Ok(head) = repo.head() {
        if let Some(oid) = head.target() {
            if let Ok(parent) = repo.find_commit(oid) {
                parents.push(parent);
            }
        }
    }

    // Check if there are changes to commit
    if !allow_empty {
        let head_tree = parents.get(0).map(|p| p.tree()).transpose()?;
        let diff = repo.diff_tree_to_tree(head_tree.as_ref(), Some(&tree), None)?;

        if diff.deltas().len() == 0 {
            return Err(GitError::NothingToCommit("no changes to commit".to_string()));
        }
    }

    // Create commit
    let parent_refs: Vec<_> = parents.iter().collect();
    let commit_id = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        message,
        &tree,
        &parent_refs
    )?;

    let summary = message.lines().next().unwrap_or("");

    Ok(ToolResult::ok(json!({
        "action": "commit",
        "repo_path": path.to_string_lossy(),
        "data": {
            "commit_id": commit_id.to_string(),
            "summary": summary
        },
        "warnings": []
    })))
}

async fn branch_create(
    guard: &GitGuard,
    repo_path: &str,
    branch: &str,
    start_point: Option<&str>,
    checkout: bool,
) -> Result<ToolResult, GitError> {
    let (path, repo) = guard.authorize_repo(repo_path).await?;

    // Check if branch already exists
    if repo.find_branch(branch, git2::BranchType::Local).is_ok() {
        return Err(GitError::BranchExists(branch.to_string()));
    }

    // Get commit to branch from
    let commit = match start_point {
        Some(sp) => {
            let obj = repo.revparse_single(sp)?;
            obj.peel_to_commit()?
        }
        None => {
            repo.head()
                .ok()
                .and_then(|h| h.target())
                .and_then(|oid| repo.find_commit(oid).ok())
                .ok_or_else(|| GitError::OperationFailed("no HEAD commit".to_string()))?
        }
    };

    // Create branch (force = false)
    let _branch_ref = repo.branch(branch, &commit, false)?;

    let checked_out = if checkout {
        // Checkout the branch with force to handle existing files
        let mut checkout_opts = git2::build::CheckoutBuilder::new();
        checkout_opts.force();
        let obj = commit.as_object();
        repo.checkout_tree(obj, Some(&mut checkout_opts))?;
        repo.set_head(&format!("refs/heads/{}", branch))?;
        true
    } else {
        false
    };

    Ok(ToolResult::ok(json!({
        "action": "branch_create",
        "repo_path": path.to_string_lossy(),
        "data": {
            "branch": branch,
            "commit_id": commit.id().to_string(),
            "checked_out": checked_out
        },
        "warnings": []
    })))
}

async fn checkout(
    guard: &GitGuard,
    repo_path: &str,
    branch: &str,
) -> Result<ToolResult, GitError> {
    let (path, repo) = guard.authorize_repo(repo_path).await?;

    // Check for uncommitted changes
    let head_tree = repo.head()
        .ok()
        .and_then(|h| h.target())
        .and_then(|oid| repo.find_commit(oid).ok())
        .and_then(|c| c.tree().ok());

    let mut diff_opts = git2::DiffOptions::new();
    let diff = repo.diff_tree_to_workdir(head_tree.as_ref(), Some(&mut diff_opts))?;

    if diff.deltas().count() > 0 {
        return Err(GitError::DirtyWorktree("uncommitted changes, please commit or stash first".to_string()));
    }

    // Find the branch
    let branch_ref = repo.find_branch(branch, git2::BranchType::Local)?;
    let commit = branch_ref.get().peel_to_commit()?;

    // Checkout with force option
    let mut checkout_opts = git2::build::CheckoutBuilder::new();
    checkout_opts.force();
    let obj = commit.as_object();
    repo.checkout_tree(obj, Some(&mut checkout_opts))?;
    repo.set_head(&format!("refs/heads/{}", branch))?;

    let new_head = get_current_branch(&repo)?;

    Ok(ToolResult::ok(json!({
        "action": "checkout",
        "repo_path": path.to_string_lossy(),
        "data": {
            "branch": branch,
            "head": new_head,
            "commit_id": commit.id().to_string()
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
