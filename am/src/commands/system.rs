use anyhow::Result;
use clap::{Parser, Subcommand};
use indicatif::MultiProgress;

pub mod prune;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Arguments {
    #[command(subcommand)]
    pub command: SubCommands,
}

#[derive(Subcommand)]
pub enum SubCommands {
    /// Delete all locally downloaded binaries.
    Prune(prune::Arguments),
}

pub async fn handle_command(args: Arguments, mp: MultiProgress) -> Result<()> {
    match args.command {
        SubCommands::Prune(args) => prune::handle_command(args, mp).await,
    }
}
