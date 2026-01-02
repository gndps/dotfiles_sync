# Changelog - Major Refactoring

## Version 0.2.0 - Major Architecture Update

### 🎯 Key Changes

#### 1. **Restructured Database System**
- **Old**: Single `config_db/` with `applications/`, `configuration_files/`, `xdg_configuration_files/`
- **New**: 
  - `default_db/` - Pre-generated from mackup (packaged with releases)
    - `applications/` - App names
    - `default_configs/` - Configuration file paths
  - `custom_db/` - User-created stubs
    - `applications/` - Custom app names
    - `default_configs/` - Custom config paths
  - Custom stubs take precedence over default stubs

#### 2. **Removed Auto-Mackup Sync**
- **Old**: `dotfiles init` automatically cloned mackup repo
- **New**: 
  - Run `python3 scripts/generate_default_db.py` once to populate database
  - `default_db/` is committed to repo and packaged with releases
  - No runtime dependency on mackup repository

#### 3. **Added Encryption Support**
- Encrypt sensitive config files with AES-256-GCM
- Password-based encryption using Argon2 key derivation
- Commands:
  - `dotfiles add <stub> --encrypt [--password <pwd>]`
  - `dotfiles sync` - Syncs only unencrypted (default)
  - `dotfiles sync --all` - Syncs everything (prompts for password)
  - `dotfiles sync --encrypted` - Syncs only encrypted files

#### 4. **Added Tagging System**
- Organize custom configurations with tags
- Commands:
  - `dotfiles init --tag work` - Initialize with tag
  - `dotfiles create <stub> <paths...> --tag personal` - Tag custom stub
- Tag saved in `.dotfiles.config`
- Custom stubs organized in `custom_db/<tag>/`

#### 5. **Simplified Path Resolution**
- **Old**: Chose between XDG and traditional based on `use_xdg` config
- **New**: Always prioritizes traditional config files
  - Checks: `~/`, `~/.config/`, `/` in that order
  - Uses first existing location
  - Simpler and more predictable

### 📋 Migration Guide

#### For Existing Users

1. **Backup your current setup**:
   ```bash
   cp -r config_db config_db.backup
   ```

2. **Generate new default_db**:
   ```bash
   python3 scripts/generate_default_db.py
   ```

3. **Migrate custom stubs**:
   ```bash
   # Move your custom configs to custom_db
   mkdir -p custom_db/applications custom_db/default_configs
   mv config_db/applications/mycustom.conf custom_db/applications/
   mv config_db/configuration_files/mycustom.conf custom_db/default_configs/
   ```

4. **Update tracked files** (if needed):
   - Tracked files with encryption now have format: `stub: path: encrypted`
   - Regular files: `stub: path`

#### For New Users

1. **Clone and build**:
   ```bash
   git clone <repo>
   cd dotfiles
   python3 scripts/generate_default_db.py  # One-time
   cargo build --release
   ```

2. **Initialize**:
   ```bash
   dotfiles init
   ```

3. **Add configs**:
   ```bash
   dotfiles add git
   dotfiles add vim --encrypt  # Encrypted
   ```

### 🆕 New Commands & Flags

#### Init
```bash
dotfiles init [--tag <name>]
```

#### Add
```bash
dotfiles add <stub> [--encrypt] [--password <pwd>]
```

#### Sync
```bash
dotfiles sync           # Unencrypted only (default)
dotfiles sync --all     # Everything (prompts for password)
dotfiles sync --encrypted  # Encrypted only
```

#### Create
```bash
dotfiles create <stub> <paths...> [--tag <name>]
```

### 🔒 Encryption Details

- **Algorithm**: AES-256-GCM
- **Key Derivation**: Argon2
- **Storage**: Encrypted files saved as `.enc` in repository
- **Format**: `dotfiles.conf` marks encrypted files with `: encrypted` suffix

Example `dotfiles.conf`:
```
git: ~/.gitconfig
ssh: ~/.ssh/config: encrypted
aws: ~/.aws/credentials: encrypted
vim: ~/.vimrc
```

### 📊 Database Structure Comparison

#### Old Structure
```
config_db/
├── applications/
│   └── git.conf (name = Git)
├── configuration_files/
│   └── git.conf (.gitconfig)
└── xdg_configuration_files/
    └── git.conf (empty or xdg path)
```

#### New Structure
```
default_db/               # Generated once, committed
├── applications/
│   └── git.conf (name = Git)
└── default_configs/
    └── git.conf (.gitconfig)

custom_db/                # User-created
├── applications/
│   └── myapp.conf
└── default_configs/
    └── myapp.conf

custom_db/work/           # Tagged custom configs
├── applications/
│   └── worktool.conf
└── default_configs/
    └── worktool.conf
```

### 🗑️ Removed

- Mackup sync module (`src/mackup.rs`)
- Auto-sync on init (`--skip-mackup-sync` flag removed)
- XDG preference logic (now always checks traditional first)
- `xdg_configuration_files/` directory

### ⚠️ Breaking Changes

1. Database folder structure changed
2. Config format slightly changed (encryption flag)
3. Init no longer auto-syncs mackup
4. Path resolution behavior changed

### 🐛 Bug Fixes

- Fixed path resolution for espanso and other apps with non-standard paths
- Improved error messages for encryption failures
- Better handling of missing files during sync

### 📝 Notes

- The `default_db/` should be generated once and committed to git
- All encryption uses password-based encryption (no key files)
- Tags are optional and mainly for organizing large custom stub collections
- Custom stubs always override default stubs with same name

---

For detailed usage, see README.md and docs/
