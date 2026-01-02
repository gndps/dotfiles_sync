# Quick Start Guide

Get up and running with dotfiles manager in 5 minutes.

## Installation

```bash
# Clone this repository
git clone https://github.com/yourusername/dotfiles.git
cd dotfiles

# Build and install
cargo install --path .
```

## First Time Setup

### 1. Initialize Your Dotfiles Repository

```bash
# Create a directory for your dotfiles
mkdir ~/my-dotfiles
cd ~/my-dotfiles

# Initialize
dotfiles init
```

This creates:
- `dotfiles.conf` - Tracks your files
- `.dotfiles.config` - Configuration
- `config_db/` - Available application stubs
- `.git/` - Git repository

### 2. Add Your First Config File

```bash
# Add git configuration
dotfiles add git

# Check status
dotfiles status
```

### 3. Connect to GitHub (Optional but Recommended)

```bash
# Create a new repo on GitHub
# Then connect it:
git remote add origin git@github.com:yourusername/my-dotfiles.git
git add .
git commit -m "Initial dotfiles commit"
git push -u origin main
```

### 4. Add More Configs

```bash
# See what's available
dotfiles list --all

# Add more tools
dotfiles add vim
dotfiles add tmux
dotfiles add zsh

# Check what's tracked
dotfiles list
```

### 5. Your First Sync

```bash
# Commit and sync
git add .
git commit -m "Added vim, tmux, zsh configs"
dotfiles sync
```

## Daily Workflow

### Making Changes

```bash
# Edit your configs normally
vim ~/.vimrc
vim ~/.gitconfig

# When ready to sync
cd ~/my-dotfiles
dotfiles sync
```

That's it! The `sync` command:
1. Copies your changes to the repo
2. Pulls from remote
3. Merges changes
4. Updates your home directory
5. Pushes to remote

### Setting Up on Another Machine

```bash
# Clone your dotfiles
git clone git@github.com:yourusername/my-dotfiles.git ~/my-dotfiles
cd ~/my-dotfiles

# Initialize (reads existing config)
dotfiles init

# Sync to home directory
dotfiles sync_local
```

## Common Commands

```bash
dotfiles status        # Check sync status
dotfiles list          # Show tracked files
dotfiles list --all    # Show available stubs
dotfiles add <stub>    # Start tracking a config
dotfiles remove <stub> # Stop tracking a config
dotfiles sync          # Full bidirectional sync
dotfiles pull          # Just pull from remote
dotfiles push          # Just push to remote
```

## Custom Configs

For apps not in the database:

```bash
dotfiles create myapp ~/.myapprc ~/.config/myapp/config
dotfiles add myapp
```

## Handling Conflicts

If you see "Merge conflict detected":

```bash
# Go to repo directory
cd ~/my-dotfiles

# See conflicted files
git status

# Edit and resolve conflicts
vim <conflicted-file>

# Mark as resolved
git add <conflicted-file>
git stash drop

# Continue sync
dotfiles sync
```

## Tips

1. **Commit often** - Use regular git commits for history
2. **Test first** - Use `dotfiles status` before syncing
3. **One repo per machine type** - Different configs for work/personal
4. **Use branches** - For experimental configs
5. **Backup sensitive data** - Don't commit secrets

## Troubleshooting

### Command not found
```bash
# Make sure it's installed
cargo install --path .

# Or add to PATH
export PATH="$HOME/.cargo/bin:$PATH"
```

### Not initialized error
```bash
# Make sure you're in your dotfiles directory
cd ~/my-dotfiles

# Or initialize if needed
dotfiles init
```

### Sync issues
```bash
# Check status first
dotfiles status

# Check git status
git status

# If stuck in merge, resolve conflicts then:
git add .
git stash drop
dotfiles sync
```

## Next Steps

- Read the full [README.md](README.md) for detailed documentation
- Check [ARCHITECTURE.md](ARCHITECTURE.md) to understand how it works
- Star the repo and share with friends!

---

**Questions?** Open an issue on GitHub!
