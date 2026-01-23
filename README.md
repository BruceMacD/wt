# wt

Git worktree manager with fzf integration.

## Install

```bash
cargo build --release
./target/release/wt init
```

Restart your shell.

## Usage

```bash
wt              # Open fzf picker - select existing or type new branch name
wt exit         # Return to main repo directory
wt list         # Show all worktrees
wt remove NAME  # Delete a worktree
```

## How it works

1. Run `wt` in any git repo
2. fzf opens with existing worktrees
3. Select one to switch, or type a new branch name and press Enter
4. New worktrees are created at `~/.wt/<repo>/<branch>/` with a matching branch

## Requirements

- [fzf](https://github.com/junegunn/fzf) - `brew install fzf`

## PR Workflow

```bash
# Start feature
wt                      # type: feature-xyz, press Enter
# ... work ...
git push -u origin feature-xyz
gh pr create

# Switch contexts
wt                      # pick another worktree

# Cleanup after merge
wt exit
wt remove feature-xyz
```
