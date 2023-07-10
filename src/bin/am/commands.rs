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

    /// Enable verbose logging. By enabling this you are also able to use
    /// RUST_LOG environment variable to change the log levels of other
    /// modules.
    ///
    /// By default we will only log INFO level messages of all modules. If this
    /// flag is enabled, then we will log the message from `am` with DEBUG
    /// level, other modules still use the INFO level.
    #[clap(long, short)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum SubCommands {
    /// Start scraping the specified endpoint(s), while also providing a web
    /// interface to inspect the autometrics data.
    Start(start::Arguments),

    /// Manage am related system settings. Such as cleaning up downloaded
    /// Prometheus, Pushgateway installs.
    System(system::Arguments),

    /// Open the Fiberplane discord to receive help, send suggestions or
    /// discuss various things related to Autometrics and the `am` CLI
    Discord,

    #[clap(hide = true)]
    MarkdownHelp,
}

pub async fn handle_command(app: Application, mp: MultiProgress) -> Result<()> {
    match app.command {
        SubCommands::Start(args) => start::handle_command(args, mp).await,
        SubCommands::System(args) => system::handle_command(args, mp).await,
        SubCommands::Discord => Ok(open::that("https://discord.gg/kHtwcH8As9")?),
        SubCommands::MarkdownHelp => {
            clap_markdown::print_help_markdown::<Application>();
            Ok(())
        }
    }
}
