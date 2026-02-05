mod error;
mod fzf;
mod git;
mod shell;

use clap::{Parser, Subcommand};
use std::process::ExitCode;

use crate::error::WtError;
use crate::fzf::FzfResult;

#[derive(Parser)]
#[command(name = "worktree")]
#[command(about = "Git worktree manager with fzf integration")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Return to the main git repository directory
    Exit,
    /// Remove a worktree
    #[command(visible_alias = "rm")]
    Remove {
        /// Name of the worktree/branch to remove (defaults to last switched-from worktree)
        name: Option<String>,
    },
    /// Set or show branch name prefix
    Prefix {
        /// Prefix to add to new branch names (omit to show current, use "" to clear)
        value: Option<String>,
    },
    /// Print the shell alias for wt
    Alias,
    /// Create or switch to a worktree
    New {
        /// Name of the worktree/branch to create or switch to
        name: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        None => run_default(),
        Some(Commands::Exit) => run_exit(),
        Some(Commands::Remove { name }) => run_remove(name),
        Some(Commands::Prefix { value }) => run_prefix(value),
        Some(Commands::Alias) => run_alias(),
        Some(Commands::New { name }) => run_create_or_switch(&name),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn run_create_or_switch(name: &str) -> Result<(), WtError> {
    git::find_git_root()?;

    // Check if worktree already exists
    if let Some(wt) = git::find_worktree_by_name(name)? {
        println!("{}", wt.path.display());
    } else {
        // Create new worktree
        eprintln!("Creating worktree for branch: {}", name);
        let path = git::create_worktree(name)?;
        println!("{}", path.display());
    }

    Ok(())
}

fn run_default() -> Result<(), WtError> {
    // Ensure we're in a git repo
    git::find_git_root()?;

    // Get current worktree before switching
    let current = git::get_current_worktree()?;

    // Get all worktrees
    let worktrees = git::list_worktrees()?;

    // Run fzf
    match fzf::run_fzf(&worktrees)? {
        FzfResult::Selected(branch) => {
            // Save current worktree as "last" before switching (skip main)
            if let Some(ref cur) = current {
                if !cur.is_main {
                    let _ = git::save_last_worktree(&cur.branch);
                }
            }
            // Find and switch to selected worktree
            if let Some(wt) = git::find_worktree_by_name(&branch)? {
                println!("{}", wt.path.display());
            }
        }
        FzfResult::New(branch_name) => {
            // Save current worktree as "last" before switching (skip main)
            if let Some(ref cur) = current {
                if !cur.is_main {
                    let _ = git::save_last_worktree(&cur.branch);
                }
            }
            // Create new worktree with this branch name
            eprintln!("Creating worktree for branch: {}", branch_name);
            let path = git::create_worktree(&branch_name)?;
            println!("{}", path.display());
        }
        FzfResult::Cancelled => {
            // User cancelled, do nothing
        }
    }

    Ok(())
}

fn run_exit() -> Result<(), WtError> {
    // Save current worktree as "last" before going back to main
    if let Some(cur) = git::get_current_worktree()? {
        if !cur.is_main {
            let _ = git::save_last_worktree(&cur.branch);
        }
    }
    let main_worktree = git::get_main_worktree()?;
    println!("{}", main_worktree.display());
    Ok(())
}

fn run_remove(name: Option<String>) -> Result<(), WtError> {
    let target = match name {
        Some(n) => n,
        None => git::get_last_worktree()?
            .ok_or_else(|| WtError::GitCommand("no previous worktree to remove".to_string()))?,
    };

    // Don't allow removing the worktree you're currently in
    if let Some(cur) = git::get_current_worktree()? {
        if !cur.is_main && cur.branch == target {
            return Err(WtError::GitCommand(
                "cannot remove the current worktree, run `wt exit` first".to_string(),
            ));
        }
    }

    let removed = git::remove_worktree(&target)?;
    for branch in &removed {
        eprintln!("Removed worktree: {}", branch);
    }
    Ok(())
}

fn run_prefix(value: Option<String>) -> Result<(), WtError> {
    match value {
        Some(v) => {
            git::set_prefix(&v)?;
            if v.is_empty() {
                eprintln!("Cleared prefix");
            } else {
                eprintln!("Set prefix: {}", v);
            }
        }
        None => {
            if let Some(prefix) = git::get_prefix()? {
                println!("{}", prefix);
            } else {
                eprintln!("No prefix set");
            }
        }
    }
    Ok(())
}

fn run_alias() -> Result<(), WtError> {
    shell::print_alias()
}
