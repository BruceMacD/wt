```
           __
 _      __/ /_
| | /| / / __/
| |/ |/ / /_
|__/|__/\__/
```

# wt - Git Worktree Manager

A fast, interactive Git worktree manager powered by [fzf](https://github.com/junegunn/fzf). Create, switch, and clean up worktrees without remembering any `git worktree` commands.

## Install

```bash
cargo install --path .
worktree alias
```

Add the printed alias to your shell config (`~/.bashrc`, `~/.zshrc`, etc.):

```bash
wt() { cd "$(worktree "$@")"; }
```

### Building from source

```bash
git clone https://github.com/brucemacd/wt.git
cd wt
cargo build --release
cp target/release/worktree ~/.local/bin/
```

## Commands

| Command | Description |
|---|---|
| `wt` | Open interactive picker — select an existing worktree or type a new branch name |
| `wt new <name>` | Create or switch to a worktree by name |
| `wt exit` | Return to the main repo directory |
| `wt remove <name>` | Delete a worktree (alias: `wt rm`) |
| `wt prefix "feature/"` | Set a prefix applied to all new branch names |
| `wt prefix` | Show the current prefix |
| `wt prefix ""` | Clear the prefix |

## How it works

1. Run `wt` inside any Git repo
2. An fzf picker shows your existing worktrees
3. Select one to switch, or type a new branch name and press Enter
4. New worktrees are created at `~/.wt/<repo>/<branch>/`

## Example workflow

```bash
# Start a feature
wt                        # type "feature-xyz", press Enter
# ... write code ...
git push -u origin feature-xyz
gh pr create

# Switch context
wt                        # pick another worktree

# Cleanup after merge
wt exit
wt remove feature-xyz
```
