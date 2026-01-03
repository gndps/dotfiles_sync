# Architecture Documentation

## Overview

The dotfiles manager is built around a **git-based stash strategy** that ensures safe synchronization between your home directory and a git repository.

## Core Concepts

### Three Locations

1. **Home Directory (DIR1)** - Where your actual config files live (`~/.vimrc`, `~/.gitconfig`, etc.)
2. **Repository (DIR2)** - Git-backed storage for versioning and syncing
3. **Remote** - Optional remote git repository for backup and sharing

### The Stash Strategy

Instead of naive copying, we leverage git's merge capabilities:

```
1. Snapshot: Copy HOME → REPO (makes repo look like your work)
2. Stash: Save those changes safely (git stash)
3. Pull: Get remote updates (git pull)
4. Merge: Apply your changes on top (git stash pop)
5. Sync Back: Copy resolved state REPO → HOME
6. Push: Share with remote (git push)
```

This approach ensures:
- Git handles the complex merge logic
- Conflicts are detected and require manual resolution
- You never lose data (stash + git history)
- Standard git tools work for conflict resolution

## Project Structure

```
dotfiles/
├── src/
│   ├── main.rs              # Entry point, CLI routing
│   ├── cli.rs               # Command-line argument parsing (clap)
│   ├── config.rs            # Configuration management
│   ├── db.rs                # Stub database operations
│   ├── git.rs               # Git operations wrapper
│   ├── sync.rs              # File synchronization logic
│   ├── utils.rs             # Output formatting utilities
│   └── commands/            # Command implementations
│       ├── mod.rs
│       ├── init.rs          # Initialize repository
│       ├── add.rs           # Add files to tracking
│       ├── remove.rs        # Remove files from tracking
│       ├── list.rs          # List tracked/available files
│       ├── status.rs        # Show sync status
│       ├── sync.rs          # Full bidirectional sync
│       ├── sync_local.rs    # Repo → Home only
│       ├── pull.rs          # Git pull
│       ├── push.rs          # Git push
│       └── create.rs        # Create custom stubs
├── config_db/               # Stub definitions
│   ├── applications/        # App names
│   ├── configuration_files/ # Traditional dotfile paths
│   └── xdg_configuration_files/ # XDG-compliant paths
├── dotfiles.conf            # Tracked files (stub: path)
├── .dotfiles.config         # Tool configuration (TOML)
└── Cargo.toml               # Rust dependencies

```

## Data Flow

### Adding a File

```
User: dotfiles add vim

1. Load stub from config_db/applications/vim.conf
2. Read file paths from config_db/configuration_files/vim.conf
3. Copy ~/.vimrc → repo/.vimrc
4. Add "vim: ~/.vimrc" to dotfiles.conf
```

### Sync Operation

```
User: dotfiles sync

1. sync_home_to_repo()
   - Read dotfiles.conf
   - For each tracked file:
     - Copy HOME/file → REPO/file

2. git.stash()
   - git add .
   - git stash push -m "dotfiles-sync-<timestamp>"

3. git.pull()
   - git pull origin main

4. git.stash_pop()
   - git stash pop
   - If conflicts: STOP, ask user to resolve
   - If success: continue

5. sync_repo_to_home()
   - For each tracked file:
     - Copy REPO/file → HOME/file

6. git.push()
   - git push origin main
```

## Key Design Decisions

### Why Not Just Copy Files?

**Problem**: Blindly copying files loses git's conflict detection.

**Solution**: Use git stash to let git do the merging.

### Why Stub-Based?

**Problem**: Different apps store configs in different places.

**Solution**: Pre-configured stubs map app names to file paths.

### Why Flat Config File?

**Problem**: Complex config formats are hard to edit manually.

**Solution**: Simple `stub: path` format in `dotfiles.conf`.

### Why Two Sync Directions?

- **sync**: Bidirectional, handles conflicts, pushes to remote
- **sync_local**: One-way REPO → HOME, for applying pulled changes

## Error Handling

### Merge Conflicts

When `git stash pop` fails:
1. Tool stops immediately
2. Shows conflict resolution steps
3. User resolves in repo using standard git tools
4. User runs `dotfiles sync` again to complete

### Missing Files

- **Missing in HOME**: Skipped during sync, shown in status
- **Missing in REPO**: Created when syncing from HOME

### Git States

- **No remote**: Sync works locally, skips push/pull
- **In merge**: Tool refuses to operate until resolved
- **Dirty working tree**: Handled by stash strategy

## Module Responsibilities

### `config.rs`
- Load/save `.dotfiles.config` (TOML)
- Load/save `dotfiles.conf` (flat list)
- Determine repository path

### `db.rs`
- Read stub definitions from `config_db/`
- List available stubs
- Create new stub entries

### `git.rs`
- Wrap git commands with error handling
- Check repository state (is_in_merge, has_changes)
- Execute git operations (stash, pull, push)

### `sync.rs`
- File/directory copying operations
- Path expansion (`~/` → `/Users/you/`)
- Cross-platform file operations

### `commands/`
Each command is independent and follows this pattern:
1. Check preconditions (initialized? git repo?)
2. Load necessary config/data
3. Perform operation
4. Save state if needed
5. Show user-friendly output

## Testing Strategy

### Unit Tests
- Path expansion
- Config parsing
- Stub loading

### Integration Tests
- Initialize repo
- Add/remove files
- Sync operations (mocked git)

### Manual Tests
- Real git operations
- Conflict resolution
- Cross-platform compatibility

## Future Enhancements

### Planned Features
- [ ] Watch mode (auto-sync on file changes)
- [ ] Machine-specific configs
- [ ] Pre/post sync hooks
- [ ] Dry-run mode for all operations

### Considerations
- Keep it simple - don't over-engineer
- Git is the source of truth
- Manual resolution is better than wrong auto-resolution
- Cross-platform compatibility is key

## Dependencies

### Core
- `clap` - CLI argument parsing
- `anyhow` - Error handling
- `walkdir` - Directory traversal
- `serde` / `serde_json` / `toml` - Serialization
- `dirs` - Cross-platform paths

### Utilities
- `colored` - Terminal colors
- `regex` - Pattern matching (future use)

## Build & Distribution

### Local Development
```bash
cargo build
cargo test
cargo run -- init
```

### Release Build
```bash
cargo build --release
strip target/release/dotfiles  # Reduce size
```

### Distribution (Future)
- GitHub Actions for multi-platform builds
- `cargo-dist` for release automation
- Homebrew tap for macOS/Linux
- Cargo registry for `cargo install`

## Security Considerations

- Never auto-commit sensitive files (use .gitignore)
- SSH keys should be added carefully
- Repository permissions should be restricted

## Performance

- File operations are fast (Rust performance)
- Git operations depend on repository size
- Stash strategy adds minimal overhead
- No background daemons or watchers (by design)

---

This architecture prioritizes **safety, simplicity, and git compatibility** over automation complexity.
