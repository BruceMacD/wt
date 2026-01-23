use std::path::PathBuf;
use std::process::Command;

use crate::error::{Result, WtError};

#[derive(Debug, Clone)]
pub struct Worktree {
    pub path: PathBuf,
    pub branch: String,
    pub is_main: bool,
}

pub fn find_git_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;

    if !output.status.success() {
        return Err(WtError::NotAGitRepo);
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(path))
}

pub fn get_repo_name() -> Result<String> {
    let root = find_git_root()?;
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "repo".to_string());
    Ok(name)
}

pub fn get_main_worktree() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()?;

    if !output.status.success() {
        return Err(WtError::NotAGitRepo);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            // First worktree listed is the main one
            return Ok(PathBuf::from(path));
        }
    }

    Err(WtError::NotAGitRepo)
}

pub fn list_worktrees() -> Result<Vec<Worktree>> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()?;

    if !output.status.success() {
        return Err(WtError::NotAGitRepo);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;

    for line in stdout.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            // Save previous worktree if complete
            if let (Some(path), Some(branch)) = (current_path.take(), current_branch.take()) {
                worktrees.push(Worktree {
                    path,
                    branch,
                    is_main: worktrees.is_empty(),
                });
            }
            current_path = Some(PathBuf::from(path));
            current_branch = None;
        } else if let Some(branch_ref) = line.strip_prefix("branch ") {
            // refs/heads/main -> main
            let branch = branch_ref
                .strip_prefix("refs/heads/")
                .unwrap_or(branch_ref)
                .to_string();
            current_branch = Some(branch);
        } else if line == "detached" {
            current_branch = Some("(detached)".to_string());
        }
    }

    // Don't forget the last worktree
    if let (Some(path), Some(branch)) = (current_path, current_branch) {
        worktrees.push(Worktree {
            path,
            branch,
            is_main: worktrees.is_empty(),
        });
    }

    Ok(worktrees)
}

pub fn get_worktree_base_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| {
        WtError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "home directory not found",
        ))
    })?;
    let repo_name = get_repo_name()?;
    Ok(home.join(".wt").join(repo_name))
}

pub fn branch_exists(branch: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["show-ref", "--verify", "--quiet", &format!("refs/heads/{}", branch)])
        .output()?;

    Ok(output.status.success())
}

pub fn create_worktree(branch_name: &str) -> Result<PathBuf> {
    let base_path = get_worktree_base_path()?;

    // Sanitize branch name for directory (replace / with -)
    let dir_name = branch_name.replace('/', "-");
    let worktree_path = base_path.join(&dir_name);

    // Check if worktree already exists at this path
    if worktree_path.exists() {
        // Check if it's already a valid worktree
        let worktrees = list_worktrees()?;
        for wt in &worktrees {
            if wt.path == worktree_path {
                // Already exists, just return the path
                return Ok(worktree_path);
            }
        }
        // Directory exists but not a worktree - this is a problem
        return Err(WtError::WorktreeExists(worktree_path));
    }

    // Create parent directories
    if let Some(parent) = worktree_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Check if branch exists
    let branch_exists = branch_exists(branch_name)?;

    let output = if branch_exists {
        // Branch exists, just create worktree
        Command::new("git")
            .args(["worktree", "add", worktree_path.to_str().unwrap(), branch_name])
            .output()?
    } else {
        // Create new branch with worktree
        Command::new("git")
            .args([
                "worktree",
                "add",
                "-b",
                branch_name,
                worktree_path.to_str().unwrap(),
            ])
            .output()?
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(WtError::GitCommand(stderr.to_string()));
    }

    // Checkout the branch in the new worktree
    let checkout_output = Command::new("git")
        .args(["checkout", branch_name])
        .current_dir(&worktree_path)
        .output()?;

    if !checkout_output.status.success() {
        let stderr = String::from_utf8_lossy(&checkout_output.stderr);
        return Err(WtError::GitCommand(stderr.to_string()));
    }

    Ok(worktree_path)
}

pub fn remove_worktree(name: &str) -> Result<()> {
    let worktrees = list_worktrees()?;

    // Find matching worktree
    let worktree = worktrees
        .iter()
        .find(|wt| wt.branch == name || wt.path.ends_with(name))
        .ok_or_else(|| WtError::WorktreeNotFound(name.to_string()))?;

    if worktree.is_main {
        return Err(WtError::GitCommand("cannot remove main worktree".to_string()));
    }

    let output = Command::new("git")
        .args(["worktree", "remove", worktree.path.to_str().unwrap()])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(WtError::GitCommand(stderr.to_string()));
    }

    Ok(())
}

pub fn find_worktree_by_name(name: &str) -> Result<Option<Worktree>> {
    let worktrees = list_worktrees()?;
    Ok(worktrees
        .into_iter()
        .find(|wt| wt.branch == name || wt.path.ends_with(name)))
}
