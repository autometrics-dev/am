use anyhow::Result;
use clap::{Parser, Subcommand};

pub mod start;
pub mod system;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Application {
    #[command(subcommand)]
    pub command: Option<SubCommands>,
    #[clap(long, hide = true)]
    pub markdown_help: bool,
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
    if app.markdown_help {
        clap_markdown::print_help_markdown::<Application>();
        return Ok(());
    }

    match app.command {
        Some(SubCommands::Start(args)) => start::handle_command(args).await,
        Some(SubCommands::System(args)) => system::handle_command(args).await,
        None => return Ok(()),
    }
}
