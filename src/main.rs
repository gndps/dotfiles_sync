mod cli;
mod commands;
mod config;
mod db;
mod encryption;
mod git;
mod sync;
mod utils;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path, tag, encryption_key_path } => commands::init::execute(path, tag, encryption_key_path)?,
        Commands::Add { stubs, encrypt, password } => commands::add::execute(stubs, encrypt, password)?,
        Commands::Remove { stub_or_path } => commands::remove::execute(stub_or_path)?,
        Commands::List { all, stubs } => commands::list::execute(all, stubs)?,
        Commands::Status => commands::status::execute()?,
        Commands::Sync { dir, r#continue, encryption_key_path, password } => {
            if r#continue {
                commands::sync_continue::execute()?
            } else {
                commands::sync::execute(dir, encryption_key_path, password)?
            }
        },
        Commands::SyncLocal => commands::sync_local::execute()?,
        Commands::Pull => commands::pull::execute()?,
        Commands::Push => commands::push::execute()?,
        Commands::Create { stub, paths, tag } => commands::create::execute(stub, paths, tag)?,
        Commands::Scan => commands::scan::execute()?,
        Commands::Cd => commands::cd::execute()?,
        Commands::Config { key, value } => commands::config::execute(key, value)?,
    }

    Ok(())
}
