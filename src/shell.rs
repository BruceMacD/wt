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
        "fish" => r#"function wt
    if test (count $argv) -gt 0; and contains -- $argv[1] list remove rm prefix alias help -h --help
        worktree $argv
        return
    end
    if test (count $argv) -gt 1; and contains -- $argv[2] -h --help
        worktree $argv
        return
    end
    set -l dir (worktree $argv)
    or return
    test -z "$dir"; or cd "$dir"
end"#,
        _ => r#"wt() {
    case "$1" in
        list|remove|rm|prefix|alias|help|-h|--help)
            worktree "$@"
            ;;
        *)
            case "$2" in
                -h|--help) worktree "$@" ;;
                *)
                    local dir
                    dir="$(worktree "$@")" || return
                    [ -z "$dir" ] || cd "$dir"
                    ;;
            esac
            ;;
    esac
}"#,
    };

    println!("{}", alias);
    eprintln!("\nAdd this to your shell config (~/.bashrc, ~/.zshrc, etc.)");

    Ok(())
}
