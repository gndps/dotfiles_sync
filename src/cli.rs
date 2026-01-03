use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "dotfiles")]
#[command(about = "A clean, hassle-free dotfiles manager with git integration", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Initialize dotfiles repository")]
    Init {
        #[arg(help = "Path to initialize (defaults to current directory)")]
        path: Option<PathBuf>,
        
        #[arg(long, help = "Tag for organizing custom configurations")]
        tag: Option<String>,
    },

    #[command(about = "Add a config file using stub name or direct path")]
    Add {
        #[arg(help = "Stub names or paths (e.g., 'git', 'tmux', '~/.zshrc')")]
        stubs: Vec<String>,
        
        #[arg(long, help = "Encrypt the files (uses BIP39 seed phrase)")]
        encrypt: bool,
        
        #[arg(long, hide = true)]
        password: Option<String>,
    },

    #[command(visible_aliases = ["rm"])]
    #[command(about = "Remove a config file from tracking")]
    Remove {
        #[arg(help = "Stub name or path to remove (e.g., 'git', '~/.zshrc')")]
        stub_or_path: String,
    },

    #[command(visible_aliases = ["ls"])]
    #[command(about = "Show status of tracked files (or list all available stubs with --all)")]
    List {
        #[arg(short, long, help = "Show all available stubs from database")]
        all: bool,
        
        #[arg(help = "Filter by specific stub names (only works without --all)")]
        stubs: Vec<String>,
    },

    #[command(about = "Show status of tracked files")]
    Status,

    #[command(about = "Full bidirectional sync (pull + sync_local + push)")]
    Sync {
        #[arg(long, help = "Sync all files including encrypted ones")]
        all: bool,
        
        #[arg(long, help = "Sync only encrypted files")]
        encrypted: bool,
        
        #[arg(long, help = "Set dotfiles directory and save to local config")]
        dir: Option<PathBuf>,
        
        #[arg(long, hide = true)]
        password: Option<String>,
    },

    #[command(about = "Sync from repository to home directory only")]
    SyncLocal,

    #[command(about = "Pull changes from remote repository")]
    Pull,

    #[command(about = "Push changes to remote repository")]
    Push,

    #[command(about = "Create a new custom stub entry")]
    Create {
        #[arg(help = "Stub name for the new entry")]
        stub: String,
        #[arg(help = "File paths to track (relative to home directory)")]
        paths: Vec<String>,
        
        #[arg(long, help = "Tag for organizing this custom stub")]
        tag: Option<String>,
    },

    #[command(about = "Scan system for available dotfiles and show their status")]
    Scan,

    #[command(about = "Change to dotfiles repository directory")]
    Cd,
}
