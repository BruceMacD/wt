use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use crate::error::{Result, WtError};

fn get_script(shell: &str) -> Option<&'static str> {
    match shell {
        "bash" => Some(BASH_SCRIPT),
        "zsh" => Some(ZSH_SCRIPT),
        "fish" => Some(FISH_SCRIPT),
        _ => None,
    }
}

fn get_script_extension(shell: &str) -> &'static str {
    match shell {
        "fish" => "fish",
        _ => "sh",
    }
}

fn detect_shell() -> Option<String> {
    env::var("SHELL")
        .ok()
        .and_then(|s| s.rsplit('/').next().map(|s| s.to_string()))
}

fn get_shell_config_path(shell: &str) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    match shell {
        "bash" => {
            let bashrc = home.join(".bashrc");
            if bashrc.exists() {
                Some(bashrc)
            } else {
                Some(home.join(".bash_profile"))
            }
        }
        "zsh" => Some(home.join(".zshrc")),
        "fish" => Some(home.join(".config/fish/config.fish")),
        _ => None,
    }
}

fn get_data_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| {
        WtError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "home directory not found",
        ))
    })?;
    Ok(home.join(".local/share/wt"))
}

fn get_bin_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| {
        WtError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "home directory not found",
        ))
    })?;
    Ok(home.join(".local/bin"))
}

pub fn install() -> Result<()> {
    let shell = detect_shell().ok_or_else(|| {
        WtError::GitCommand("could not detect shell from $SHELL".to_string())
    })?;

    let script = get_script(&shell).ok_or_else(|| {
        WtError::GitCommand(format!("unsupported shell: {}. Supported: bash, zsh, fish", shell))
    })?;

    // 1. Get current executable path
    let current_exe = env::current_exe()?;

    // 2. Create ~/.local/bin if needed and copy binary
    let bin_dir = get_bin_dir()?;
    fs::create_dir_all(&bin_dir)?;

    let target_bin = bin_dir.join("wt");
    if current_exe != target_bin {
        fs::copy(&current_exe, &target_bin)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&target_bin)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&target_bin, perms)?;
        }
        eprintln!("Installed binary to {}", target_bin.display());
    } else {
        eprintln!("Binary already at {}", target_bin.display());
    }

    // 3. Write shell script to ~/.local/share/wt/wt.sh
    let data_dir = get_data_dir()?;
    fs::create_dir_all(&data_dir)?;

    let script_file = data_dir.join(format!("wt.{}", get_script_extension(&shell)));
    fs::write(&script_file, script)?;
    eprintln!("Wrote shell integration to {}", script_file.display());

    // 4. Add source line to shell config
    let config_path = get_shell_config_path(&shell).ok_or_else(|| {
        WtError::GitCommand(format!("could not find config for shell: {}", shell))
    })?;

    let source_line = if shell == "fish" {
        format!("source {}", script_file.display())
    } else {
        format!("source \"{}\"", script_file.display())
    };

    let config_content = fs::read_to_string(&config_path).unwrap_or_default();
    if config_content.contains(".local/share/wt") {
        eprintln!("Shell already configured in {}", config_path.display());
    } else {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config_path)?;

        writeln!(file)?;
        writeln!(file, "# wt - git worktree manager")?;
        writeln!(file, "{}", source_line)?;

        eprintln!("Added source line to {}", config_path.display());
    }

    // 5. Check if ~/.local/bin is in PATH
    let path = env::var("PATH").unwrap_or_default();
    let bin_dir_str = bin_dir.to_string_lossy();
    if !path.split(':').any(|p| p == bin_dir_str) {
        eprintln!();
        eprintln!("Note: {} is not in your PATH", bin_dir.display());
        eprintln!("Add this to your shell config:");
        eprintln!("  export PATH=\"$HOME/.local/bin:$PATH\"");
    }

    eprintln!();
    eprintln!("Done! Restart your shell or run:");
    eprintln!("  source {}", config_path.display());

    Ok(())
}

const BASH_SCRIPT: &str = r#"
wt() {
    local output
    output=$(command wt "$@" 2>&1)
    local code=$?

    if [[ $code -eq 0 && -d "$output" ]]; then
        cd "$output" || return 1
    elif [[ -n "$output" ]]; then
        echo "$output"
    fi
    return $code
}
"#;

const ZSH_SCRIPT: &str = r#"
wt() {
    local output
    output=$(command wt "$@" 2>&1)
    local code=$?

    if [[ $code -eq 0 && -d "$output" ]]; then
        cd "$output" || return 1
    elif [[ -n "$output" ]]; then
        echo "$output"
    fi
    return $code
}
"#;

const FISH_SCRIPT: &str = r#"
function wt
    set -l output (command wt $argv 2>&1)
    set -l code $status

    if test $code -eq 0 -a -d "$output"
        cd "$output"
    else if test -n "$output"
        echo "$output"
    end
    return $code
end
"#;
