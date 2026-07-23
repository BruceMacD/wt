use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::git::Worktree;

pub fn attach_worktree_hints(worktrees: &mut [Worktree]) {
    let paths: Vec<_> = worktrees
        .iter()
        .filter(|worktree| worktree.is_codex_managed())
        .map(|worktree| worktree.path.clone())
        .collect();

    if paths.is_empty() {
        return;
    }

    let Some(hints) = load_worktree_hints(&paths) else {
        return;
    };

    for worktree in worktrees {
        if let Some(hint) = hints.get(&worktree.path) {
            worktree.set_hint(hint.clone());
        }
    }
}

fn load_worktree_hints(paths: &[PathBuf]) -> Option<HashMap<PathBuf, String>> {
    let codex_home = env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".codex")))?;
    let database = find_latest_state_database(&codex_home)?;

    let quoted_paths: Vec<_> = paths
        .iter()
        .filter_map(|path| path.to_str())
        .map(quote_sql_string)
        .collect();

    if quoted_paths.is_empty() {
        return None;
    }

    // Hex encoding keeps tabs and newlines in titles from corrupting the row format.
    // Ordering newest-first lets the parser retain the latest primary chat per worktree.
    let query = format!(
        "SELECT hex(cwd), hex(title) \
         FROM threads \
         WHERE cwd IN ({}) \
           AND title != '' \
           AND substr(source, 1, 1) != '{{' \
         ORDER BY updated_at DESC;",
        quoted_paths.join(", ")
    );

    let output = Command::new("sqlite3")
        .args(["-readonly", "-batch", "-separator", "\t"])
        .arg(database)
        .arg(query)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(parse_hint_rows(&String::from_utf8_lossy(&output.stdout)))
}

fn find_latest_state_database(codex_home: &Path) -> Option<PathBuf> {
    fs::read_dir(codex_home)
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let version = state_database_version(&entry.file_name())?;
            Some((version, entry.path()))
        })
        .max_by_key(|(version, _)| *version)
        .map(|(_, path)| path)
}

fn state_database_version(file_name: &std::ffi::OsStr) -> Option<u64> {
    let file_name = file_name.to_str()?;
    file_name
        .strip_prefix("state_")?
        .strip_suffix(".sqlite")?
        .parse()
        .ok()
}

fn quote_sql_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn parse_hint_rows(output: &str) -> HashMap<PathBuf, String> {
    let mut hints = HashMap::new();

    for line in output.lines() {
        let Some((path, title)) = line.split_once('\t') else {
            continue;
        };
        let (Some(path), Some(title)) = (decode_hex(path), decode_hex(title)) else {
            continue;
        };
        let title = normalize_title(&title);
        if title.is_empty() {
            continue;
        }

        hints.entry(PathBuf::from(path)).or_insert(title);
    }

    hints
}

fn decode_hex(value: &str) -> Option<String> {
    if value.len() & 1 == 1 {
        return None;
    }

    let bytes: Option<Vec<_>> = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(pair, 16).ok()
        })
        .collect();

    String::from_utf8(bytes?).ok()
}

fn normalize_title(title: &str) -> String {
    const MAX_CHARS: usize = 80;

    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut characters = title.chars();
    let shortened: String = characters.by_ref().take(MAX_CHARS).collect();

    if characters.next().is_some() {
        format!("{}…", shortened.trim_end())
    } else {
        shortened
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_normalizes_newest_hint_for_each_path() {
        let output = "\
2F55736572732F62727563652F2E636F6465782F776F726B74726565732F333039362F6F6C6C616D612E636F6D\t557064617465204D697870616E656C20207465616D0A7573657220747261636B696E67\n\
2F55736572732F62727563652F2E636F6465782F776F726B74726565732F333039362F6F6C6C616D612E636F6D\t4F6C64207469746C65\n";

        let hints = parse_hint_rows(output);

        assert_eq!(
            hints.get(&PathBuf::from(
                "/Users/bruce/.codex/worktrees/3096/ollama.com"
            )),
            Some(&"Update Mixpanel team user tracking".to_string())
        );
    }

    #[test]
    fn ignores_malformed_rows_for_graceful_fallback() {
        let hints = parse_hint_rows("not-hex\talso-not-hex\nmissing-separator\n");

        assert!(hints.is_empty());
    }

    #[test]
    fn recognizes_versioned_state_databases() {
        assert_eq!(
            state_database_version(std::ffi::OsStr::new("state_5.sqlite")),
            Some(5)
        );
        assert_eq!(
            state_database_version(std::ffi::OsStr::new("state.sqlite")),
            None
        );
    }
}
