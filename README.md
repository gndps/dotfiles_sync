# Dotfiles Manager

A clean, hassle-free dotfiles manager built in Rust with native git integration. Manages your configuration files across machines with proper conflict resolution support.

## Features

- 🚀 **Simple CLI** - Easy-to-use commands for managing dotfiles
- 🔄 **Robust Sync Workflow** - 6-step bidirectional sync with automatic backups
- 🔀 **Smart Git Integration** - Auto-commits, rebase strategy, conflict protection
- 📦 **Flexible Tracking** - Use pre-configured stubs OR track files directly
- 🔒 **Encryption Support** - AES-256-GCM encryption for sensitive files
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

## Quick Start

### 1. Initialize Repository

```bash
# In an empty directory or existing dotfiles repo
dotfiles init

# Or specify a path
dotfiles init ~/my-dotfiles
```

This creates:
- `dotfiles.config.json` - Main configuration with tracked files
- `custom_db/` - Custom stub definitions
- `.backup/` - **Local-only backup directory (gitignored, unencrypted)**
- `.gitignore` - Auto-configured
- `.git/` - Git repository with initial commit
- `~/.dotfiles.local.config.json` - **Global config saved in your home directory**

**Important**: After initialization, the dotfiles directory path is saved to `~/.dotfiles.local.config.json` in your home directory. This allows you to run **all dotfiles commands from anywhere** without being in the dotfiles directory!

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

# Add encrypted files for sensitive data
dotfiles add --encrypt ~/.ssh/config

# Scan system for available dotfiles
dotfiles scan

# See all available stubs
dotfiles list --all
```

### 3. Sync Your Files

```bash
# Full bidirectional sync (syncs ALL files - encrypted and non-encrypted)
# Works from ANY directory!
dotfiles sync

# Or use specific operations
dotfiles pull         # Pull from remote
dotfiles sync_local   # Sync repo → home only
dotfiles push         # Push to remote

# All commands work from anywhere:
cd /tmp
dotfiles add vim      # Still works!
dotfiles status       # Shows your dotfiles status
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
   ├─ Create parent directories if needed (first sync)
   └─ Decrypt encrypted files on-the-fly

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

#### `dotfiles add <stub|path> [--encrypt] [--password <pwd>]`
Add configuration files using a stub name OR direct path.

```bash
# Using stubs
dotfiles add git                 # Add ~/.gitconfig
dotfiles add vim                 # Add ~/.vimrc
dotfiles add zsh                 # Add ~/.zshrc

# Using direct paths
dotfiles add ~/.zshrc            # Add any file directly
dotfiles add ~/.config/nvim      # Add directories too

# With encryption
dotfiles add --encrypt ~/.ssh/config
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
Full robust bidirectional sync with automatic backups. **Always syncs ALL files** (encrypted and non-encrypted).

```bash
# Sync everything (all files)
dotfiles sync

# Change dotfiles directory and save to global config
dotfiles sync --dir ~/my-dotfiles
# This updates ~/.dotfiles.local.config.json
# All subsequent commands will use this directory automatically
# Works from anywhere after this!
```

**Note**: Sync always processes all tracked files. If you have encrypted files and the encryption key is not found, you'll be prompted for your seed phrase once.

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
      "path": "~/.gitconfig",
      "encrypted": false
    },
    {
      "stub": null,
      "path": "~/.zshrc",
      "encrypted": false
    },
    {
      "stub": "ssh",
      "path": "~/.ssh/config",
      "encrypted": true
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

## Encryption

Encryption uses a **BIP39 12-word seed phrase** for maximum simplicity and security. No passwords to remember - just write down your seed phrase once!

### Adding Your First Encrypted File

When you add your first encrypted file, the tool will:
1. Generate a 12-word BIP39 seed phrase
2. Display it prominently with warnings
3. Derive an encryption key from the seed phrase
4. **Save the encryption key to `~/.dotfiles.encryption.key` in your HOME directory (NEVER in repo!)**
5. Create a marker file `.dotfiles.encryption.enabled` in the repo to indicate encryption is used
6. Commit and push the marker file (the actual key stays on your machine)

```bash
# Add with encryption (first time)
dotfiles add --encrypt ~/.ssh/config

# You'll see:
# ═══════════════════════════════════════════════════════════════
#                   🔐 ENCRYPTION SEED PHRASE                   
# ═══════════════════════════════════════════════════════════════
# 
# ⚠️  CRITICAL: SAVE THIS SEED PHRASE NOW! ⚠️
# 
# This is your 12-word BIP39 seed phrase:
# 
#  1. word1      2. word2      3. word3
#  4. word4      5. word5      6. word6
#  7. word7      8. word8      9. word9
# 10. word10    11. word11    12. word12
# 
# ⚠️  IMPORTANT SECURITY NOTICE:
#   • You will NOT see this seed phrase again
#   • Write it down on paper (NOT digitally)
#   • Keep it in a safe place
#   • You need this to decrypt files on new machines
#   • Anyone with this phrase can decrypt your files
```

### Adding More Encrypted Files

After the first time, encryption is seamless:

```bash
# Add more encrypted files (uses existing key)
dotfiles add --encrypt ~/.aws/credentials
dotfiles add --encrypt ~/.gnupg/gpg.conf
dotfiles add --encrypt ~/.config/secrets
```

### How It Works

- **Seed Phrase**: BIP39 standard 12-word mnemonic (easy to write down)
- **Algorithm**: AES-256-GCM encryption
- **Key Derivation**: PBKDF2-HMAC-SHA256 (100,000 iterations)
- **Storage**: Encrypted files stored as `.enc` in repo
- **Encryption Key**: Stored in `~/.dotfiles.encryption.key` in your HOME directory (**NEVER in repo!**)
- **Marker File**: `.dotfiles.encryption.enabled` in repo indicates encryption is used
- **Security**: Only you have the key; new machines require seed phrase
- **Sync**: Automatic once key is set up (no password prompts on same machine)

### Setting Up on a New Machine

When you clone your dotfiles to a new machine with encrypted files:

```bash
# Clone your dotfiles
git clone https://github.com/you/dotfiles.git ~/dotfiles
cd ~/dotfiles

# Initialize
dotfiles init

# Sync will detect encrypted files and prompt for seed phrase
dotfiles sync --all

# 🔐 Enter your 12-word seed phrase to decrypt files:
#    (Enter all 12 words separated by spaces)
# 
#    Seed phrase: word1 word2 word3 ... word12
```

The tool will:
1. Detect the `.dotfiles.encryption.enabled` marker file
2. **Prompt for your 12-word seed phrase (one time only)**
3. Derive the encryption key from your seed phrase
4. **Save the key to `~/.dotfiles.encryption.key` in your HOME directory (not repo!)**
5. Decrypt all encrypted files to your home directory

**Security Note**: The encryption key is stored locally on each machine. Without your seed phrase, no one can decrypt your files on a new machine!

### Security Model

**Critical Security Design:**
- 🔒 Encryption key is **NEVER** stored in the repository
- 🏠 Key is stored in `~/.dotfiles.encryption.key` in your home directory
- 📝 Only a marker file (`.dotfiles.encryption.enabled`) is in the repo
- 🔑 New machines require your 12-word seed phrase to decrypt files
- ⚠️ Keep your seed phrase safe - it's the only way to recover access!

**What's in the Repo:**
- ✅ Encrypted files (`.enc` extension)
- ✅ Marker file indicating encryption is used
- ❌ NO encryption keys or passwords

**What's on Your Machine:**
- 🔑 Encryption key in `~/.dotfiles.encryption.key`
- 📝 Your 12-word seed phrase (write it down!)

### Syncing Encrypted Files

```bash
# Sync always processes ALL files (encrypted + non-encrypted)
dotfiles sync

# First time on a new machine? 
# You'll be prompted for your seed phrase once, then it's automatic!
# No password prompts after initial setup!
```

### Backup Security

**Local Backups are Unencrypted:**
- 📁 `.backup/` directory contains **unencrypted** copies of all files
- 🚫 `.backup/` is in `.gitignore` - **NEVER pushed to remote**
- 🏠 Backups stay on your local machine only
- 🆘 Allows emergency file recovery without seed phrase
- ⚠️ Keep your machine secure - backups contain sensitive data in plain text

**Why unencrypted local backups?**
- Emergency recovery if you forget seed phrase
- Quick file restoration without decryption
- Safe because they never leave your machine

### Security Model

**Critical Security Design:**
- � Encryption key is **NEVER** stored in the repository
- 🏠 Key is stored in `~/.dotfiles.encryption.key` in your home directory
- 📝 Only a marker file (`.dotfiles.encryption.enabled`) is in the repo
- 🔑 New machines require your 12-word seed phrase to decrypt files
- ⚠️ Keep your seed phrase safe - it's the only way to recover access!

**What's in the Repo:**
- ✅ Encrypted files (`.enc` extension)
- ✅ Marker file indicating encryption is used
- ❌ NO encryption keys or passwords
- ❌ NO backups (they're local-only)

**What's on Your Machine:**
- 🔑 Encryption key in `~/.dotfiles.encryption.key`
- 📝 Your 12-word seed phrase (write it down!)
- 📁 Unencrypted backups in `.backup/` (local-only, gitignored)

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
- Committed to repo and pushed to remote
- **✅ SECURITY:** Encrypted files are backed up **encrypted** (with `.enc` extension)
  - Your sensitive data remains encrypted even in backups
  - Backup directory is safe to push to public repositories
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

```bash
# Clone your dotfiles
git clone https://github.com/you/dotfiles.git ~/dotfiles
cd ~/dotfiles

# Initialize (creates local config, reads tracked files from JSON)
dotfiles init

# First sync - creates missing files/directories
dotfiles sync --all
# Will prompt for your 12-word seed phrase if you have encrypted files

# Or scan to see what's available
dotfiles scan
```

### Daily Usage

```bash
# Make changes to your configs
vim ~/.vimrc
vim ~/.gitconfig

# Sync everything (automatic commit, pull, backup, export, push)
cd ~/dotfiles
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

### Working with Encrypted Files

```bash
# First time: Add sensitive files with encryption
# This generates and displays your 12-word seed phrase
dotfiles add --encrypt ~/.ssh/config
# ⚠️  SAVE THE SEED PHRASE THAT IS DISPLAYED!

# Add more encrypted files (uses existing encryption key)
dotfiles add --encrypt ~/.aws/credentials
dotfiles add --encrypt ~/.config/secrets

# Sync encrypted files (no password prompts!)
dotfiles sync --all

# Or sync only encrypted files
dotfiles sync --encrypted

# On a new machine, you'll be prompted once for your seed phrase
# After that, syncing is automatic
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
4. **Encrypt sensitive files** - Use `--encrypt` for SSH keys, AWS credentials, etc.
5. **Trust the backups** - Pre-export snapshots protect you automatically
6. **Resolve conflicts carefully** - Home directory is protected until you resolve
7. **Use local config** - Machine-specific settings go in `dotfiles.local.config.json`
8. **Regular syncing** - Run `dotfiles sync` often to avoid large conflicts

## Git Integration

This tool automates git operations for safety:

```bash
# Sync handles everything automatically
dotfiles sync
# - Auto-commits with timestamp
# - Pulls with rebase
# - Creates backup commits
# - Pushes everything including backups

# But you can still use git directly
cd ~/dotfiles
git log                    # View history
git diff                   # See changes
git log --all -- .backup/  # View backup history
git branch feature         # Create branches
```

### What Sync Does Automatically

```bash
# Behind the scenes, sync runs:
git add -A
git commit -m "dotfiles sync: 2024-01-02 15:30:00"
git pull --rebase origin main
git add -A  # Backup files
git commit -m "backup: pre-export snapshot 2024-01-02 15:30:00"
git push origin main
```

### Empty Remote Handling

```bash
# First push to empty remote? No problem!
dotfiles sync
# Automatically detects empty remote
# Uses 'git push -u origin main' for first push
# Subsequent syncs use regular push
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
