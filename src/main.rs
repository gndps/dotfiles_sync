mod cli;
mod commands;
mod config;
mod db;
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
        Commands::Add { stubs } => commands::add::execute(stubs)?,
        Commands::Remove { stub_or_path } => commands::remove::execute(stub_or_path)?,
        Commands::List { all, stubs } => commands::list::execute(all, stubs)?,
        Commands::Status => commands::status::execute()?,
        Commands::Sync { dir } => commands::sync::execute(dir)?,
        Commands::SyncLocal => commands::sync_local::execute()?,
        Commands::Pull => commands::pull::execute()?,
        Commands::Push => commands::push::execute()?,
        Commands::Create { stub, paths, tag } => commands::create::execute(stub, paths, tag)?,
        Commands::Scan => commands::scan::execute()?,
        Commands::Cd => commands::cd::execute()?,
    }

    Ok(())
}
