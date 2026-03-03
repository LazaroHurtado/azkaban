use color_eyre::eyre::{Result, WrapErr};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeInfo {
    pub name: String,
    pub path: PathBuf,
    pub branch: String,
    pub is_main: bool,
}

/// List all git worktrees for a repository.
/// Uses `git worktree list --porcelain` for reliable parsing since
/// git2 doesn't have great worktree enumeration support.
pub fn list_worktrees(project_path: &Path) -> Result<Vec<WorktreeInfo>> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(project_path)
        .output()
        .wrap_err("Failed to run git worktree list")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        color_eyre::eyre::bail!("git worktree list failed: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;
    let mut is_bare = false;
    let mut is_first = true;

    for line in stdout.lines() {
        if line.starts_with("worktree ") {
            // Save previous worktree if any
            if let Some(path) = current_path.take() {
                let branch = current_branch.take().unwrap_or_else(|| "HEAD".to_string());
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                if !is_bare {
                    worktrees.push(WorktreeInfo {
                        name,
                        path,
                        branch,
                        is_main: is_first,
                    });
                }
                is_first = false;
            }
            is_bare = false;
            current_path = Some(PathBuf::from(line.strip_prefix("worktree ").unwrap()));
        } else if line.starts_with("branch ") {
            let branch_ref = line.strip_prefix("branch ").unwrap();
            // Convert refs/heads/main -> main
            current_branch = Some(
                branch_ref
                    .strip_prefix("refs/heads/")
                    .unwrap_or(branch_ref)
                    .to_string(),
            );
        } else if line == "bare" {
            is_bare = true;
        }
    }

    // Don't forget the last worktree
    if let Some(path) = current_path.take() {
        let branch = current_branch.take().unwrap_or_else(|| "HEAD".to_string());
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        if !is_bare {
            worktrees.push(WorktreeInfo {
                name,
                path,
                branch,
                is_main: is_first,
            });
        }
    }

    Ok(worktrees)
}

/// Create a new git worktree for a branch.
/// The worktree is created inside the project directory under `.worktrees/<branch_name>`
/// so that it's automatically visible via the existing Docker bind mount.
pub fn create_worktree(project_path: &Path, branch_name: &str) -> Result<WorktreeInfo> {
    let worktree_dir = project_path.join(".worktrees");
    std::fs::create_dir_all(&worktree_dir)
        .wrap_err("Failed to create .worktrees directory")?;

    let worktree_path = worktree_dir.join(branch_name);
    let path_str = worktree_path.to_string_lossy().to_string();

    // Strategy 1: existing branch
    // Strategy 2: new branch from HEAD
    // Strategy 3: orphan branch (empty repo, no commits)
    let strategies: Vec<Vec<&str>> = vec![
        vec!["worktree", "add", &path_str, branch_name],
        vec!["worktree", "add", "-b", branch_name, &path_str, "HEAD"],
        vec!["worktree", "add", "--orphan", "-b", branch_name, &path_str],
    ];

    let mut last_err = String::new();
    for args in &strategies {
        let output = Command::new("git")
            .args(args)
            .current_dir(project_path)
            .output()
            .wrap_err("Failed to run git worktree add")?;

        if output.status.success() {
            return Ok(WorktreeInfo {
                name: branch_name.to_string(),
                path: worktree_path,
                branch: branch_name.to_string(),
                is_main: false,
            });
        }
        last_err = String::from_utf8_lossy(&output.stderr).to_string();
    }

    color_eyre::eyre::bail!("Failed to create worktree: {last_err}")
}

/// Remove a git worktree.
pub fn remove_worktree(project_path: &Path, worktree_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["worktree", "remove", "--force", &worktree_path.to_string_lossy()])
        .current_dir(project_path)
        .output()
        .wrap_err("Failed to remove worktree")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        color_eyre::eyre::bail!("Failed to remove worktree: {stderr}");
    }

    Ok(())
}
