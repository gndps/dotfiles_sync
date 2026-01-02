# Mackup Database Sync

The dotfiles manager automatically syncs its configuration database from the [mackup repository](https://github.com/lra/mackup) during initialization, giving you instant access to 500+ pre-configured application stubs.

## How It Works

When you run `dotfiles init`, the tool:

1. **Temporarily clones** the mackup repository to a temp directory
2. **Parses** all `.cfg` files from mackup's applications directory
3. **Converts** them to the flat structure used by this tool
4. **Populates** the `config_db/` directory with all available stubs
5. **Cleans up** the temporary clone automatically

This happens automatically and requires no manual intervention.

## Usage

### Default Behavior (Automatic Sync)

```bash
dotfiles init
```

Output:
```
→ Initializing dotfiles repository...
✓ Created dotfiles.conf
✓ Created .dotfiles.config
✓ Created config_db directory structure

→ Syncing configuration database from mackup repository...
→ Cloning mackup repository (this may take a moment)...
✓ Mackup repository cloned
→ Found 586 application configurations
✓ Processed 564 applications, skipped 22
✓ Config database synced from mackup repository

✓ Initialized git repository
✓ Dotfiles repository initialized successfully!

Tip: Use 'dotfiles list --all' to see 500+ available application stubs
```

### Skip Mackup Sync

If you prefer to manually populate the config database or already have one:

```bash
dotfiles init --skip-mackup-sync
```

Output:
```
→ Initializing dotfiles repository...
✓ Created dotfiles.conf
✓ Created .dotfiles.config
✓ Created config_db directory structure
→ Skipped mackup database sync (use dotfiles add to manually add configs)
✓ Initialized git repository
✓ Dotfiles repository initialized successfully!
```

## What Gets Synced

The sync process imports configurations for hundreds of applications including:

### Development Tools
- **Editors**: vim, neovim, emacs, vscode, sublime-text, atom, etc.
- **IDEs**: intellij, pycharm, webstorm, android-studio, xcode, etc.
- **Version Control**: git, mercurial, subversion, etc.

### Shells & Terminals
- bash, zsh, fish
- tmux, screen
- iterm2, hyper, alacritty

### Cloud & DevOps
- aws, gcloud, azure
- docker, kubernetes, terraform
- ansible, vagrant

### Languages & Runtimes
- node (npm, yarn)
- python (pip, poetry)
- ruby (gem, bundler)
- go, rust (cargo)

### Utilities
- curl, wget, ssh
- gpg, keychain
- homebrew, apt

...and 500+ more!

## Architecture

### Mackup Repository Structure
```
mackup/
└── mackup/
    └── applications/
        ├── git.cfg
        ├── vim.cfg
        ├── tmux.cfg
        └── ... (500+ more)
```

### Our Config Database Structure
```
config_db/
├── applications/
│   ├── git.conf          # name = Git
│   ├── vim.conf          # name = Vim
│   └── tmux.conf         # name = Tmux
├── configuration_files/
│   ├── git.conf          # .gitconfig
│   ├── vim.conf          # .vimrc
│   └── tmux.conf         # .tmux.conf
└── xdg_configuration_files/
    ├── git.conf          # (empty - git doesn't use XDG)
    ├── vim.conf          # (empty - vim doesn't use XDG)
    └── tmux.conf         # tmux/tmux.conf
```

### Parsing Logic

The tool includes a custom parser for mackup's `.cfg` format:

**Input (mackup .cfg file):**
```ini
[application]
name = Tmux

[configuration_files]
.tmux.conf

[xdg_configuration_files]
tmux/tmux.conf
```

**Output (flat structure):**
- `applications/tmux.conf` → `name = Tmux`
- `configuration_files/tmux.conf` → `.tmux.conf`
- `xdg_configuration_files/tmux.conf` → `tmux/tmux.conf`

## Benefits

### 1. Zero Manual Setup
No need to manually create stub definitions - they're generated automatically from the actively maintained mackup repository.

### 2. Always Up-to-Date
Every time you run `dotfiles init`, you get the latest application definitions from mackup's master branch.

### 3. Comprehensive Coverage
Access to 500+ applications means you rarely need to create custom stubs.

### 4. No External Dependencies
The sync happens entirely in Rust - no Python, no manual git operations required.

### 5. Automatic Cleanup
The temporary mackup clone is automatically removed after sync, leaving no traces.

## Troubleshooting

### "Failed to clone mackup repository"

**Cause**: Network issues or git not installed.

**Solution**:
```bash
# Ensure git is installed
git --version

# Check internet connectivity
ping github.com

# Use --skip-mackup-sync and manually populate
dotfiles init --skip-mackup-sync
```

### Slow Sync

**Cause**: Large repository download.

**Note**: The first sync clones the entire mackup repository. This is normal and only happens once per init.

**Speed it up**: The tool uses `--depth=1` for shallow cloning, but it still takes a moment on slow connections.

### Partial Sync

Some `.cfg` files may be skipped if they:
- Have parsing errors
- Don't contain any useful configuration
- Have invalid section names

This is normal and doesn't affect the majority of applications.

## Manual Population (Alternative)

If you prefer not to use mackup sync:

```bash
# Initialize without sync
dotfiles init --skip-mackup-sync

# Create custom stubs manually
dotfiles create myapp ~/.myapprc ~/.config/myapp/config
dotfiles create work ~/.work/config

# Or manually create files in config_db/
echo "name = My App" > config_db/applications/myapp.conf
echo ".myapprc" > config_db/configuration_files/myapp.conf
```

## Implementation Details

### Code Location
- **Module**: `src/mackup.rs`
- **Integration**: `src/commands/init.rs`
- **Temp Directory**: `$TMPDIR/dotfiles_mackup_sync`

### Process
1. Check if `--skip-mackup-sync` flag is set
2. Create temp directory
3. Execute: `git clone --depth=1 --single-branch https://github.com/lra/mackup.git`
4. Find applications directory (`mackup/applications` or `src/mackup/applications`)
5. For each `.cfg` file:
   - Read and parse content
   - Extract application name, config files, XDG files
   - Write to flat structure in `config_db/`
6. Remove temp directory
7. Continue with rest of initialization

### Performance
- **Clone Time**: 5-15 seconds (depending on connection)
- **Parse Time**: < 1 second (500+ files)
- **Total Time**: ~10-20 seconds on average

## FAQ

**Q: Do I need to re-sync to get updates?**  
A: No, once initialized, your `config_db/` is static. You can manually update by deleting `config_db/` and running init again in a new directory, or by manually copying specific stubs you need.

**Q: Can I modify the synced stubs?**  
A: Yes! The synced files are yours to modify. Edit any `.conf` files in `config_db/` to customize behavior.

**Q: What if mackup changes?**  
A: Your existing setup continues working. New users get the latest version automatically on init.

**Q: Does this require Python?**  
A: No! Everything is pure Rust. No Python, no external dependencies beyond git.

**Q: Is my internet connection required?**  
A: Only during `dotfiles init` (unless you use `--skip-mackup-sync`). After initialization, everything is local.

## See Also

- [Mackup Repository](https://github.com/lra/mackup)
- [Creating Custom Stubs](../README.md#custom-configs)
- [Architecture Documentation](../ARCHITECTURE.md)
