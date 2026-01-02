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
        Commands::Init { path, tag } => commands::init::execute(path, tag)?,
        Commands::Add { stub, encrypt, password } => commands::add::execute(stub, encrypt, password)?,
        Commands::Remove { stub } => commands::remove::execute(stub)?,
        Commands::List { all } => commands::list::execute(all)?,
        Commands::Status => commands::status::execute()?,
        Commands::Sync { all, encrypted, password } => commands::sync::execute(all, encrypted, password)?,
        Commands::SyncLocal => commands::sync_local::execute()?,
        Commands::Pull => commands::pull::execute()?,
        Commands::Push => commands::push::execute()?,
        Commands::Create { stub, paths, tag } => commands::create::execute(stub, paths, tag)?,
        Commands::Scan => commands::scan::execute()?,
    }

    Ok(())
}
