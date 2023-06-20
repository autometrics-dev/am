use anyhow::{Context, Result};
use clap::Parser;
use commands::{handle_command, Application};
use std::io;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};

mod commands;
mod interactive;

#[tokio::main]
async fn main() -> Result<()> {
    let app = Application::parse();

    if let Err(err) = init_logging() {
        eprintln!("Unable to initialize logging: {:#}", err);
        std::process::exit(1);
    }

    handle_command(app).await
}

/// Initialize logging for the application.
///
/// Currently, we have a straight forward logging setup that will log everything
/// that is level info and higher to stderr. Users are able to influence this by
/// exporting the `RUST_LOG` environment variable.
///
/// For example: for local development it is convenient to set the environment
/// variable to `RUST_LOG=am=trace,info`. This will display all log messages
/// within the `am` module, but will only show info for other modules.
fn init_logging() -> Result<()> {
    // The filter layer controls which log levels to display.
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::default().add_directive(LevelFilter::INFO.into()));

    let log_layer = tracing_subscriber::fmt::layer().with_writer(io::stderr);

    Registry::default()
        .with(filter)
        .with(log_layer)
        .try_init()
        .context("unable to initialize logger")?;

    Ok(())
}
