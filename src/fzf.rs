use std::io::Write;
use std::process::{Command, Stdio};

use crate::error::{Result, WtError};
use crate::git::Worktree;

pub enum FzfResult {
    Selected(String),
    New(String),
    Cancelled,
}

pub fn run_fzf(worktrees: &[Worktree]) -> Result<FzfResult> {
    // Check if fzf exists
    if Command::new("which")
        .arg("fzf")
        .output()
        .map(|o| !o.status.success())
        .unwrap_or(true)
    {
        return Err(WtError::FzfNotFound);
    }

    let mut child = Command::new("fzf")
        .args([
            "--print-query",
            "--header",
            "Select worktree or type new branch name",
            "--preview",
            "echo 'Path: {2}'",
            "--with-nth",
            "1",
            "--delimiter",
            "\t",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    // Write worktree list to fzf stdin
    if let Some(mut stdin) = child.stdin.take() {
        for wt in worktrees {
            let line = format!("{}\t{}\n", wt.branch, wt.path.display());
            let _ = stdin.write_all(line.as_bytes());
        }
    }

    let output = child.wait_with_output()?;

    // fzf exit codes:
    // 0 = selection made
    // 1 = no match (but we have --print-query so we get the query)
    // 2 = error
    // 130 = cancelled (Ctrl-C or Esc)

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    match output.status.code() {
        Some(0) => {
            // User selected an existing item
            if lines.len() >= 2 {
                // Second line is the selection (first is query)
                let selection = lines[1].split('\t').next().unwrap_or("").to_string();
                Ok(FzfResult::Selected(selection))
            } else if !lines.is_empty() {
                // Just query, treat as new
                let query = lines[0].trim().to_string();
                if query.is_empty() {
                    Ok(FzfResult::Cancelled)
                } else {
                    Ok(FzfResult::New(query))
                }
            } else {
                Ok(FzfResult::Cancelled)
            }
        }
        Some(1) => {
            // No match - user typed something and pressed Enter
            if !lines.is_empty() {
                let query = lines[0].trim().to_string();
                if query.is_empty() {
                    Ok(FzfResult::Cancelled)
                } else {
                    Ok(FzfResult::New(query))
                }
            } else {
                Ok(FzfResult::Cancelled)
            }
        }
        Some(130) | Some(2) | None => Ok(FzfResult::Cancelled),
        Some(_) => Ok(FzfResult::Cancelled),
    }
}
