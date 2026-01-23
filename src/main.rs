mod error;
mod fzf;
mod git;
mod shell;

use clap::{Parser, Subcommand};
use std::process::ExitCode;

use crate::error::WtError;
use crate::fzf::FzfResult;

#[derive(Parser)]
#[command(name = "wt")]
#[command(about = "Git worktree manager with fzf integration")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Return to the main git repository directory
    Exit,
    /// List all worktrees
    List,
    /// Remove a worktree
    Remove {
        /// Name of the worktree/branch to remove
        name: String,
    },
    /// Install wt binary and configure shell integration
    Init,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        None => run_default(),
        Some(Commands::Exit) => run_exit(),
        Some(Commands::List) => run_list(),
        Some(Commands::Remove { name }) => run_remove(&name),
        Some(Commands::Init) => run_init(),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn run_default() -> Result<(), WtError> {
    // Ensure we're in a git repo
    git::find_git_root()?;

    // Get all worktrees
    let worktrees = git::list_worktrees()?;

    // Run fzf
    match fzf::run_fzf(&worktrees)? {
        FzfResult::Selected(branch) => {
            // Find and switch to selected worktree
            if let Some(wt) = git::find_worktree_by_name(&branch)? {
                println!("{}", wt.path.display());
            }
        }
        FzfResult::New(branch_name) => {
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
    let main_worktree = git::get_main_worktree()?;
    println!("{}", main_worktree.display());
    Ok(())
}

fn run_list() -> Result<(), WtError> {
    let worktrees = git::list_worktrees()?;

    for wt in worktrees {
        let marker = if wt.is_main { "*" } else { " " };
        println!("{} {} ({})", marker, wt.branch, wt.path.display());
    }

    Ok(())
}

fn run_remove(name: &str) -> Result<(), WtError> {
    git::remove_worktree(name)?;
    eprintln!("Removed worktree: {}", name);
    Ok(())
}

fn run_init() -> Result<(), WtError> {
    shell::install()
}
