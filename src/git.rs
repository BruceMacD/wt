use std::path::PathBuf;
use std::process::Command;
use std::env;
use std::fs;

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
    let root = get_main_worktree()?;
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

    // Check if branch exists (without prefix)
    let existing_branch = branch_exists(branch_name)?;

    // Apply prefix for new branches
    let final_branch_name = if existing_branch {
        branch_name.to_string()
    } else {
        match get_prefix()? {
            Some(prefix) => format!("{}{}", prefix, branch_name),
            None => branch_name.to_string(),
        }
    };

    // Sanitize branch name for directory (replace / with -)
    let dir_name = final_branch_name.replace('/', "-");
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

    let output = if existing_branch {
        // Branch exists, just create worktree
        Command::new("git")
            .args(["worktree", "add", worktree_path.to_str().unwrap(), &final_branch_name])
            .output()?
    } else {
        // Create new branch with worktree
        Command::new("git")
            .args([
                "worktree",
                "add",
                "-b",
                &final_branch_name,
                worktree_path.to_str().unwrap(),
            ])
            .output()?
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(WtError::GitCommand(stderr.to_string()));
    }

    Ok(worktree_path)
}

pub fn remove_worktree(name: &str) -> Result<Vec<String>> {
    let worktrees = list_worktrees()?;
    let prefix = get_prefix()?;

    // Build list of names to match: input and prefix+input
    let mut names_to_match = vec![name.to_string()];
    if let Some(ref p) = prefix {
        let prefixed_name = format!("{}{}", p, name);
        if prefixed_name != name {
            names_to_match.push(prefixed_name);
        }
    }

    // Find all matching worktrees
    let matching: Vec<_> = worktrees
        .iter()
        .filter(|wt| {
            names_to_match.iter().any(|n| wt.branch == *n || wt.path.ends_with(n))
        })
        .collect();

    if matching.is_empty() {
        return Err(WtError::WorktreeNotFound(name.to_string()));
    }

    let mut removed = Vec::new();

    // Remove all matching worktrees
    for worktree in matching {
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

        removed.push(worktree.branch.clone());
    }

    Ok(removed)
}

pub fn find_worktree_by_name(name: &str) -> Result<Option<Worktree>> {
    let worktrees = list_worktrees()?;
    Ok(worktrees
        .into_iter()
        .find(|wt| wt.branch == name || wt.path.ends_with(name)))
}

pub fn get_prefix() -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["config", "--get", "worktree.prefix"])
        .output()?;

    if output.status.success() {
        let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if prefix.is_empty() {
            Ok(None)
        } else {
            Ok(Some(prefix))
        }
    } else {
        Ok(None)
    }
}

pub fn set_prefix(prefix: &str) -> Result<()> {
    if prefix.is_empty() {
        // Clear the prefix
        let _ = Command::new("git")
            .args(["config", "--unset", "worktree.prefix"])
            .output()?;
    } else {
        let output = Command::new("git")
            .args(["config", "worktree.prefix", prefix])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WtError::GitCommand(stderr.to_string()));
        }
    }
    Ok(())
}

fn get_last_worktree_file() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| {
        WtError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "home directory not found",
        ))
    })?;
    let repo_name = get_repo_name()?;
    Ok(home.join(".wt").join(repo_name).join(".last"))
}

pub fn get_current_worktree() -> Result<Option<Worktree>> {
    let cwd = env::current_dir()?;
    let worktrees = list_worktrees()?;

    // Find which worktree contains the current directory
    Ok(worktrees.into_iter().find(|wt| cwd.starts_with(&wt.path)))
}

pub fn save_last_worktree(branch: &str) -> Result<()> {
    let file = get_last_worktree_file()?;
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(file, branch)?;
    Ok(())
}

pub fn get_last_worktree() -> Result<Option<String>> {
    let file = get_last_worktree_file()?;
    if file.exists() {
        let content = fs::read_to_string(file)?;
        let branch = content.trim().to_string();
        if branch.is_empty() {
            Ok(None)
        } else {
            Ok(Some(branch))
        }
    } else {
        Ok(None)
    }
}
