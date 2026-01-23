# wt - Git Worktree Manager

## Overview

A Rust CLI tool for managing git worktrees with fzf integration. Provides fast switching between worktrees and easy creation of new ones.

## Commands

| Command | Description |
|---------|-------------|
| `wt` | Open fzf fuzzy finder to select/create worktree |
| `wt exit` | Return to the main git repository directory |
| `wt list` | List all worktrees (no fzf) |
| `wt remove <name>` | Remove a worktree |
| `wt init <shell>` | Output shell function for bash/zsh integration |

## Behavior

### `wt` (default)

1. Check if current directory is inside a git repository
2. List all existing worktrees for this repo
3. Pipe worktree names to `fzf` with `--print-query` flag
4. Based on selection:
   - **Match selected**: Automatically `cd` to that worktree
   - **No match, Enter pressed**: Create new worktree + branch, then `cd` to it

### New Worktree Creation

When creating a new worktree from fzf query:
```bash
git worktree add -b <query> ~/.wt/<repo>/<query>
```
This creates:
- A new branch named `<query>` (based on current HEAD)
- A worktree directory at `~/.wt/<repo>/<query>`
- Checks out the new branch in that worktree

### Worktree Storage

All worktrees stored in `~/.wt/<repo-name>/<branch-name>/`

```
~/.wt/
  my-project/
    feature-auth/      # branch: feature-auth
    fix-bug-123/       # branch: fix-bug-123
```

## Shell Integration (Automatic cd)

Since subprocesses can't change parent shell directory, we use a shell function wrapper. User adds to their shell config:

```bash
# ~/.bashrc or ~/.zshrc
eval "$(wt init bash)"   # or: eval "$(wt init zsh)"
```

This defines a `wt` function that:
1. Runs the `wt` binary
2. If output is a directory path, automatically `cd`s to it
3. Otherwise displays the output as-is

**User experience**: Type `wt` → pick worktree → instantly in that directory. No visible `cd` command.

### Generated Shell Function

```bash
wt() {
  local output
  output=$(command wt "$@" 2>&1)
  local code=$?

  if [[ $code -eq 0 && -d "$output" ]]; then
    cd "$output"
  else
    echo "$output"
    return $code
  fi
}
```

## Rust Project Structure

```
src/
  main.rs          # CLI entry, argument parsing (clap)
  git.rs           # Git operations (find repo, list/create worktrees)
  fzf.rs           # fzf subprocess with --print-query
  shell.rs         # Shell init script generation
  error.rs         # Error types
Cargo.toml
```

### Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
dirs = "5"
thiserror = "1"
```

## Implementation Steps

1. **Project setup** - `cargo init`, add dependencies
2. **Git module** - Repo detection, worktree list/create/remove
3. **fzf module** - Spawn fzf, handle selection vs new query
4. **Shell module** - Generate init scripts for bash/zsh
5. **CLI wiring** - Connect commands with clap
6. **README** - Installation and usage instructions

## Edge Cases

- Not in git repo → Error: "Not a git repository"
- fzf not installed → Error: "fzf required: brew install fzf"
- Branch already exists → Use existing branch, create worktree only
- Worktree already exists → Just cd to it
- Branch name with `/` → Replace with `-` for directory name

---

## Using Worktrees for Pull Requests

### Recommended Workflow

1. **Start from main worktree**
   ```bash
   cd ~/projects/my-repo
   git fetch origin
   ```

2. **Create feature branch + worktree**
   ```bash
   wt
   # type: feature-user-auth
   # press Enter → creates branch + worktree, cd's into it
   ```

3. **Work in isolation**
   - Each worktree = separate working directory
   - No stashing needed when switching tasks
   - Run builds/tests independently

4. **Push and create PR**
   ```bash
   git push -u origin feature-user-auth
   gh pr create
   ```

5. **Switch contexts freely**
   ```bash
   wt           # fuzzy find another worktree
   wt exit      # back to main repo
   ```

6. **Review someone's PR**
   ```bash
   git fetch origin
   wt
   # type their branch name → creates local worktree
   ```

7. **Cleanup after merge**
   ```bash
   wt exit
   wt remove feature-user-auth
   git branch -d feature-user-auth
   ```

### Benefits

- **Parallel work**: Multiple PRs in progress simultaneously
- **Clean context**: No uncommitted changes blocking switches
- **Fast iteration**: Instant switching with fzf
- **Easy reviews**: Checkout PR branches without disruption
