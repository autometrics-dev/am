use anyhow::Result;
use clap::{Parser, Subcommand};
use indicatif::MultiProgress;

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

    #[clap(hide = true)]
    MarkdownHelp,
}

pub async fn handle_command(app: Application, mp: MultiProgress) -> Result<()> {
    match app.command {
        SubCommands::Start(args) => start::handle_command(args, mp).await,
        SubCommands::System(args) => system::handle_command(args, mp).await,
        SubCommands::MarkdownHelp => {
            clap_markdown::print_help_markdown::<Application>();
            Ok(())
        }
    }
}
