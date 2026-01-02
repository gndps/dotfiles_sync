# Scripts

## generate_config_db.py

One-time script to generate the `config_db/` directory from the [mackup](https://github.com/lra/mackup) repository.

### What it does

Converts mackup's `.cfg` files into the flat structure needed for this dotfiles manager:

**Input (mackup format):**
```ini
[application]
name = Tmux

[configuration_files]
.tmux.conf

[xdg_configuration_files]
tmux/tmux.conf
```

**Output (flat structure):**
```
config_db/
├── applications/tmux.conf
│   └── name = Tmux
├── configuration_files/tmux.conf
│   └── .tmux.conf
└── xdg_configuration_files/tmux.conf
    └── tmux/tmux.conf
```

### Usage

#### 1. Clone mackup repository

```bash
git clone https://github.com/lra/mackup.git ~/mackup
```

#### 2. Run the script

```bash
# From the dotfiles project root
python3 scripts/generate_config_db.py ~/mackup ./config_db
```

#### 3. Use the generated database

```bash
# The config_db directory is now populated with hundreds of app stubs
dotfiles list --all

# Use any stub
dotfiles add docker
dotfiles add vscode
dotfiles add aws
```

### Options

```bash
python3 scripts/generate_config_db.py [mackup_repo_path] [output_dir]

# Examples:
python3 scripts/generate_config_db.py ~/mackup ./config_db
python3 scripts/generate_config_db.py /path/to/mackup /output/path
```

### What gets generated

The script processes all `.cfg` files from mackup's `applications/` directory and creates:

- **applications/**: App names (e.g., `Git`, `Vim`, `Docker`)
- **configuration_files/**: Traditional dotfile paths (e.g., `.vimrc`, `.gitconfig`)
- **xdg_configuration_files/**: XDG-compliant paths (e.g., `nvim/init.vim`)

Currently, mackup has **500+ application configurations** including:

- Development tools (git, vim, vscode, intellij, etc.)
- Shells (bash, zsh, fish)
- Terminal multiplexers (tmux, screen)
- Cloud CLIs (aws, gcloud, azure)
- And many more...

### Updating

To update with latest mackup configs:

```bash
cd ~/mackup
git pull

cd /path/to/dotfiles
python3 scripts/generate_config_db.py ~/mackup ./config_db
```

### Notes

- The script creates empty files for apps without config/XDG files
- Invalid or unparseable `.cfg` files are skipped with warnings
- The script is idempotent - safe to run multiple times
- Existing files in `config_db/` will be overwritten
