# Dotfiles Manager

A clean, hassle-free dotfiles manager built in Rust with native git integration. Manages your configuration files across machines with proper conflict resolution support.

## Features

- 🚀 **Simple CLI** - Easy-to-use commands for managing dotfiles
- 🔄 **Robust Sync Workflow** - 6-step bidirectional sync with automatic backups
- 🔀 **Smart Git Integration** - Auto-commits, rebase strategy, conflict protection
- 📦 **Flexible Tracking** - Use pre-configured stubs OR track files directly
- 💾 **Automatic Backups** - Pre-export snapshots stored in repo before every sync
- 🔍 **System Scanning** - Discover available dotfiles on your system
- 🛡️ **Data Safety First** - Never overwrites home directory on conflicts
- ⚡ **Fast & Reliable** - Built in Rust for speed and reliability
- 🌍 **Cross-platform** - Works on macOS, Linux, and Windows

## Installation

### From Source

```bash
cargo install --path .
```

### From Release

#### Homebrew
```bash
# Homebrew (macOS/Linux)
brew install gndps/tap/dotfiles_sync
```

#### Shell
```bash
# Shell
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/gndps/dotfiles_sync/releases/latest/download/dotfiles_sync-installer.sh | sh
```

## How the Dotfiles Directory Works

The tool manages files through a dedicated **dotfiles directory** — a plain git repository that stores copies of your config files. The relationship looks like this:

```
~/.gitconfig          ←──── sync ────→   ~/dotfiles/.gitconfig
~/.tmux.conf          ←──── sync ────→   ~/dotfiles/.tmux.conf
~/Library/.../settings.json  ←── sync →  ~/dotfiles/Library/.../settings.json

                                          ~/dotfiles/  (git repo)
                                               │
                                          git push/pull
                                               │
                                          github.com/you/dotfiles
```

You **must** initialize a dotfiles directory before any syncing can happen. This directory is the single source of truth. All other dotfiles commands work relative to it — and once initialized, they work from **any directory** on your machine.

## Quick Start

### 1. Create and Initialize the Dotfiles Directory

```bash
# In an empty directory you want to use as your dotfiles repo
mkdir ~/dotfiles && cd ~/dotfiles
dotfiles init

# Or let init create the directory for you
dotfiles init ~/dotfiles
```

This creates the dotfiles directory and registers it as the active repo on your machine:
- `dotfiles.config.json` - Tracked files list (committed to git, shared across machines)
- `custom_db/` - Custom stub definitions
- `.backup/` - **Local-only backup snapshots (gitignored)**
- `.gitignore` - Auto-configured
- `.git/` - Git repository with initial commit
- `~/.dotfiles.local.config.json` - **Machine-local config (path to your dotfiles dir)**

**After init, all dotfiles commands work from anywhere on this machine** — the path is stored in `~/.dotfiles.local.config.json`.

**Note**: The tool embeds 600+ application configurations from the [mackup repository](https://github.com/lra/mackup) in the binary.

### 2. Add Configuration Files

```bash
# Add files using pre-configured stubs
dotfiles add git      # Adds ~/.gitconfig
dotfiles add vim      # Adds ~/.vimrc
dotfiles add tmux     # Adds ~/.tmux.conf

# Or add files directly without stubs
dotfiles add ~/.zshrc
dotfiles add ~/.config/nvim

# Scan system for available dotfiles
dotfiles scan

# See all available stubs
dotfiles list --all
```

### 3. Connect a Git Remote (recommended)

```bash
# Create a repo on GitHub, then:
cd ~/dotfiles
git remote add origin git@github.com:you/dotfiles.git
```

Once a remote is added, **`dotfiles sync` automatically pushes to it** — no manual push needed. Without a remote, sync still works as a local backup.

### 4. Sync Your Files

```bash
# Full bidirectional sync: imports from home, commits, pulls, exports to home, pushes
dotfiles sync

# Works from ANY directory after init!
cd /tmp
dotfiles sync         # Still works!
dotfiles status       # Shows status from anywhere

# Individual operations
dotfiles pull         # Pull from remote only
dotfiles sync-local   # Copy repo → home only (no git)
dotfiles push         # Push to remote only
```

## How It Works

The dotfiles manager uses a **robust 6-step sync workflow** with automatic backups and rebase strategy to ensure safe, conflict-free synchronization:

### Architecture Overview

```
┌─────────────┐         ┌──────────────┐         ┌─────────────┐
│   Home Dir  │  sync   │  Repository  │   git   │   Remote    │
│  (DIR1)     │ ◄─────► │   (DIR2)     │ ◄─────► │   (GitHub)  │
└─────────────┘         └──────────────┘         └─────────────┘
  ~/.gitconfig            .gitconfig               origin/main
  ~/.vimrc                .vimrc
```

### Robust Sync Flow

When you run `dotfiles sync`, here's what happens:

```
Step 1/6: Import (Home → Repo)
   ├─ Copy tracked files from home directory to repository
   └─ Repository now has your latest changes

Step 2/6: Auto-Commit
   ├─ git add -A
   ├─ git commit -m "dotfiles sync: 2024-01-02 15:30:00"
   └─ Changes committed with timestamp

Step 3/6: Pull with Rebase
   ├─ Check if remote has commits (skip if empty)
   ├─ git pull --rebase origin main
   ├─ SAFETY LOCK: On conflict, STOPS before touching home
   └─ User resolves conflicts manually

Step 4/6: Backup Current Home Files
   ├─ Create timestamped snapshot: .backup/20240102_153000/
   ├─ Copy current home files before overwriting
   ├─ git commit backup
   └─ Emergency recovery available if needed

Step 5/6: Export (Repo → Home)
   ├─ Copy resolved files from repo to home directory
   └─ Create parent directories if needed (first sync)

Step 6/6: Push to Remote
   ├─ git push origin main (or -u for first push)
   └─ Changes and backups synced to remote
```

### Conflict Resolution & Safety

If a merge conflict occurs during sync:

```bash
$ dotfiles sync
→ Step 1/6: Importing local changes...
→ Step 2/6: Committing local changes...
✓ Local changes committed
→ Step 3/6: Pulling updates from remote...
✗ Merge conflict during update!

SAFETY LOCK ENGAGED: Home directory was NOT updated.
  1. Go to repository directory
  2. Resolve conflicts manually
  3. Run 'git rebase --continue'
  4. Run 'dotfiles sync' again
```

**Key Safety Features:**
- ✅ Home directory is NEVER touched if conflicts occur
- ✅ Automatic backups before every export
- ✅ All changes committed before pulling
- ✅ Backups stored in repo and pushed to remote

**Example conflict resolution:**

```bash
cd ~/my-dotfiles

# Open conflicted file
vim .gitconfig

# You'll see standard git conflict markers:
<<<<<<< HEAD
[user]
    name = John Doe (from remote)
=======
[user]
    name = Jane Doe (from local)
>>>>>>> dotfiles sync: 2024-01-02 15:30:00

# Edit to keep what you want:
[user]
    name = John Doe

# Resolve the conflict
git add .gitconfig
git rebase --continue

# Continue sync
dotfiles sync
```

## Commands Reference

### Repository Management

#### `dotfiles init [path] [--tag <tag>]`
Initialize a new dotfiles repository.

```bash
dotfiles init                    # Current directory
dotfiles init ~/my-dotfiles      # Specific path
dotfiles init --tag work         # Use tag for custom stubs
```

Creates initial commit, configures `.gitignore`, sets up directory structure, and **saves the dotfiles directory path to `~/.dotfiles.local.config.json`**. After initialization, all dotfiles commands work from any directory!

### File Management

#### `dotfiles add <stub|path>`
Add configuration files using a stub name OR direct path.

```bash
# Using stubs
dotfiles add git                 # Add ~/.gitconfig
dotfiles add vim                 # Add ~/.vimrc
dotfiles add zsh                 # Add ~/.zshrc

# Using direct paths
dotfiles add ~/.zshrc            # Add any file directly
dotfiles add ~/.config/nvim      # Add directories too
```

#### `dotfiles remove <stub|path>` (alias: `rm`)
Stop tracking files for a stub or path.

```bash
# Remove by stub name
dotfiles remove vim
dotfiles rm tmux

# Remove by direct path
dotfiles remove ~/.zshrc
dotfiles rm ~/.config/nvim
```

#### `dotfiles list` (alias: `ls`)
Show currently tracked files.

```bash
dotfiles list                    # Show tracked files
dotfiles list --all              # Show all available stubs
dotfiles ls -a                   # Short form
```

#### `dotfiles status`
Check sync status of tracked files.

```bash
dotfiles status

# Output shows:
# ✓ file (in sync)
# ✗ file (out of sync)
# ⚠ file (missing in home)
# ? file (missing in repo)
```

#### `dotfiles create <stub> <paths...> [--tag <tag>]`
Create a custom stub for files not in the database.

```bash
dotfiles create myapp ~/.myapprc ~/.config/myapp/config
dotfiles create work-tools ~/.work-config --tag work
dotfiles add myapp
```

#### `dotfiles scan`
Scan system for available dotfiles and show their status.

```bash
dotfiles scan

# Output shows:
# ✓ Synced - Tracked and files match
# ⚠ Out of Sync - Tracked but files differ  
# ○ Unmanaged - Files exist but not tracked
```

Perfect for discovering what dotfiles exist on a new machine!

### Sync Operations

#### `dotfiles sync [--dir <path>]`
Full robust bidirectional sync with automatic backups.

```bash
# Sync everything (all files)
dotfiles sync

# Change dotfiles directory and save to global config
dotfiles sync --dir ~/my-dotfiles
# This updates ~/.dotfiles.local.config.json
# All subsequent commands will use this directory automatically
# Works from anywhere after this!
```

**6-Step Process:**
1. Import changes from home to repo
2. Auto-commit with timestamp
3. Pull with rebase (skips if remote empty or no remote)
4. Create backup snapshot
5. Export from repo to home
6. Push to remote (including backups)

**Use this when:**
- You want to sync everything safely
- You've made changes locally and want to share them
- You want to get remote updates
- First sync to empty remote repository

#### `dotfiles sync_local`
Sync from repository to home directory only (no git operations).

```bash
dotfiles sync_local
```

**Use this when:**
- You just pulled manually and want to update home
- You're testing changes in the repo
- You want to restore files from repo

#### `dotfiles pull`
Pull changes from remote repository.

```bash
dotfiles pull
```

**Use this when:**
- You want to fetch remote changes only
- You'll manually resolve any issues

#### `dotfiles push`
Push changes to remote repository.

```bash
dotfiles push
```

**Use this when:**
- You've manually committed changes
- You want to share your changes without pulling

**Note:** Push will fail if there are unresolved merge conflicts.

## Configuration

### `dotfiles.config.json`

Main configuration file (committed to repo):

```json
{
  "use_xdg": false,
  "repo_path": "/Users/you/dotfiles",
  "home_path": "/Users/you",
  "tag": null,
  "tracked_files": [
    {
      "stub": "git",
      "path": "~/.gitconfig"
    },
    {
      "stub": null,
      "path": "~/.zshrc"
    },
    {
      "stub": "ssh",
      "path": "~/.ssh/config"
    }
  ]
}
```

### `dotfiles.local.config.json`

Local overrides (gitignored, machine-specific):

```json
{
  "use_xdg": true,
  "tag": "work"
}
```

**Note:** Local config overrides main config, perfect for machine-specific settings.

## Automatic Backups

Every time files are exported to your home directory, a backup is created first:

```
.backup/
├── 20240102_153045/      # Timestamped snapshot
│   ├── .gitconfig
│   ├── .zshrc
│   └── .config/
│       └── nvim/
├── 20240102_183012/      # Another snapshot
│   └── ...
```

**Features:**
- Created before every export (Step 4 of sync)
- Timestamped: `YYYYMMDD_HHMMSS`
- Local-only (gitignored, never pushed to remote)
- Manual recovery if needed

**Emergency Recovery:**
```bash
cd ~/dotfiles/.backup/20240102_153045
cp -r .gitconfig ~/
```

## Custom Stub Database

The `custom_db/` directory contains your custom stub definitions:

```
custom_db/
├── applications/           # Application names
│   ├── myapp.conf         # name = My Application
│   └── work-tools.conf    # name = Work Tools
└── default_configs/        # File paths for each stub
    ├── myapp.conf         # ~/.myapprc
    └── work-tools.conf    # ~/.work-config

# Tagged custom stubs
custom_db/
└── work/                   # --tag work
    ├── applications/
    └── default_configs/
```

### Adding Custom Stubs

You can add your own application configs:

**Method 1: Use `create` command (recommended)**
```bash
dotfiles create myapp ~/.myapprc ~/.config/myapp/config
dotfiles create work-tools ~/.work-config --tag work
```

**Method 2: Manual creation**
```bash
# Create stub files
echo "name = My Application" > custom_db/applications/myapp.conf
echo "~/.myapprc" > custom_db/default_configs/myapp.conf

# Add to tracking
dotfiles add myapp
```

**Default Database:**
The tool embeds 600+ default stubs from mackup in the binary. No need to download anything!

## Real-World Workflows

### Setting Up on a New Machine

On a new machine, the dotfiles directory doesn't exist yet. Clone it from your remote and register it:

```bash
# 1. Clone your existing dotfiles repo
git clone https://github.com/you/dotfiles.git ~/dotfiles
cd ~/dotfiles

# 2. Register it as the active dotfiles directory on this machine
#    (saves path to ~/.dotfiles.local.config.json so commands work from anywhere)
dotfiles init

# 3. Restore all tracked files to your home directory
dotfiles sync
```

`dotfiles init` on an existing repo is safe — it skips re-initialization but always registers the path on the current machine. Without this step, other commands won't know where your dotfiles directory is.

### Daily Usage

```bash
# Make changes to your configs
vim ~/.vimrc
vim ~/.gitconfig

# Sync everything — works from anywhere!
# Automatically: imports changes → commits → pulls remote → exports to home → pushes
dotfiles sync

# Check what's tracked and status
dotfiles list
dotfiles status

# Discover new dotfiles on your system
dotfiles scan
```

### Handling Conflicts

```bash
# Conflict occurs during sync
dotfiles sync
# → Step 3/6: Pulling updates from remote...
# ✗ Merge conflict during update!
# SAFETY LOCK ENGAGED: Home directory was NOT updated.

# Go to repo and resolve
cd ~/dotfiles
git status              # See conflicted files
vim .vimrc              # Resolve conflicts
git add .vimrc
git rebase --continue  # Continue rebase

# Continue sync (will create backup and export)
dotfiles sync
# ✓ Sync completed successfully!
```

### Emergency Recovery from Backup

```bash
# Something went wrong after export?
cd ~/dotfiles/.backup
ls -la  # See all timestamped backups

# Restore from specific backup
cd 20240102_153045
cp -r .gitconfig ~/.gitconfig
cp -r .config ~/

# Backups are also in git history
git log --all -- .backup/
```

### Sharing Configs Across Team

```bash
# Team member pushes changes
# You pull and review
dotfiles pull
dotfiles status    # See what changed
git diff           # Review changes

# Apply to your home directory
dotfiles sync_local
```

## Troubleshooting

### "Not in a dotfiles repository"

```bash
# Make sure you're in the repository directory
cd ~/dotfiles

# Or initialize if needed
dotfiles init
```

### "Merge conflict detected"

The tool won't auto-resolve conflicts. Manually resolve them:

```bash
cd ~/dotfiles
git status                    # See conflicts
vim <conflicted-file>         # Resolve
git add <resolved-file>
git rebase --continue         # Complete the rebase
dotfiles sync                 # Continue
```

### "No remote configured" - Local-Only Backup

```bash
# Sync works fine without remote (local backup only)
dotfiles sync
# ⚠️  No remote repository configured - backup is LOCAL ONLY
#    Add a remote with: git remote add origin <url>

# Add a remote when ready
cd ~/dotfiles
git remote add origin git@github.com:you/dotfiles.git

# Next sync will push everything (including backups)
dotfiles sync
```

### "Remote is empty" - First Push

```bash
# First sync to empty remote works automatically
dotfiles sync
# → Remote is empty - skipping pull (first push)
# → Step 6/6: Pushing to remote (including backups)...
# ✓ Pushed successfully (set upstream tracking)
```

### Files not syncing

```bash
# Check status
dotfiles status

# Verify tracking
dotfiles list

# Check git status
cd ~/dotfiles
git status
```

## Best Practices

1. **Let sync handle git** - The tool auto-commits with timestamps
2. **Use scan on new machines** - Discover available dotfiles: `dotfiles scan`
3. **Track directly when needed** - Use `dotfiles add ~/.file` for quick additions
4. **Trust the backups** - Pre-export snapshots protect you automatically
5. **Resolve conflicts carefully** - Home directory is protected until you resolve
6. **Use local config** - Machine-specific settings go in `dotfiles.local.config.json`
8. **Regular syncing** - Run `dotfiles sync` often to avoid large conflicts

## Git Integration

`dotfiles sync` is a fully automated git workflow — no manual git commands needed for day-to-day use.

### What Happens During `dotfiles sync`

```bash
# Behind the scenes:
git add -A
git commit -m "dotfiles sync: 2024-01-02 15:30:00"   # auto-commit
git pull --rebase origin main                          # safe merge
# → export tracked files to home directory
git push origin main                                   # auto-push (if remote configured)
```

### Automatic Push Behavior

| Remote configured? | Behavior |
|---|---|
| Yes | `sync` automatically pushes after every export |
| No | `sync` works locally only — warns you that backup is local-only |

To add a remote at any time:
```bash
cd ~/dotfiles
git remote add origin git@github.com:you/dotfiles.git
# Next `dotfiles sync` will push automatically
```

### Empty Remote / First Push

```bash
# First sync to empty remote? No problem!
dotfiles sync
# → Detects empty remote, skips pull
# → Uses 'git push -u origin main' to set upstream
# → All subsequent syncs use regular push
```

### Manual Git Access

You can still use git directly in the dotfiles directory:
```bash
cd ~/dotfiles
git log                    # View sync history
git diff                   # See uncommitted changes
git log --all -- .backup/  # View backup history
git branch feature         # Create branches
```

## Development

### Building

```bash
cargo build --release
```

### Testing

```bash
cargo test
```

### Contributing

Contributions welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Add tests for new features
4. Submit a pull request

## License

MIT License - See LICENSE file for details

## Acknowledgments

- Inspired by [mackup](https://github.com/lra/mackup) for the stub-based approach
- Built with Rust for reliability and cross-platform support

## Support

- **Issues**: [GitHub Issues](https://github.com/yourusername/dotfiles/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/dotfiles/discussions)
- **Documentation**: This README and `dotfiles --help`

---

**Made with ❤️ for the dotfiles community**
