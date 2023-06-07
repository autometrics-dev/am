use anyhow::Result;
use clap::{Parser, Subcommand};

pub mod start;
pub mod system;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Application {
    #[command(subcommand)]
    pub command: SubCommands,
}

#[derive(Subcommand)]
pub enum SubCommands {
    /// Start scraping the specified endpoint, while also providing a web
    /// interface to inspect the autometrics data.
    Start(start::Arguments),

    /// Manage am related system settings.
    System(system::Arguments),
}

pub async fn handle_command(app: Application) -> Result<()> {
    match app.command {
        SubCommands::Start(args) => start::handle_command(args).await,
        SubCommands::System(args) => system::handle_command(args).await,
    }
}
