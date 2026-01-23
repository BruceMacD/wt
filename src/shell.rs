use std::env;

use crate::error::{Result, WtError};

fn detect_shell() -> Option<String> {
    env::var("SHELL")
        .ok()
        .and_then(|s| s.rsplit('/').next().map(|s| s.to_string()))
}

pub fn print_alias() -> Result<()> {
    let shell = detect_shell().ok_or_else(|| {
        WtError::GitCommand("could not detect shell from $SHELL".to_string())
    })?;

    let alias = match shell.as_str() {
        "fish" => r#"function wt; cd (worktree $argv); end"#,
        _ => r#"wt() { cd "$(worktree "$@")"; }"#,
    };

    println!("{}", alias);
    eprintln!("\nAdd this to your shell config (~/.bashrc, ~/.zshrc, etc.)");

    Ok(())
}
