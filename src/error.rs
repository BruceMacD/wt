use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WtError {
    #[error("not a git repository (or any parent up to mount point)")]
    NotAGitRepo,

    #[error("fzf not found. Install it: brew install fzf")]
    FzfNotFound,

    #[error("git command failed: {0}")]
    GitCommand(String),

    #[error("worktree not found: {0}")]
    WorktreeNotFound(String),

    #[error("worktree already exists at: {0}")]
    WorktreeExists(PathBuf),

    #[error("{0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, WtError>;
