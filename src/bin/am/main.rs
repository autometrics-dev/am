use anyhow::{Context, Result};
use clap::Parser;
use commands::{handle_command, Application};
use std::io;
use tracing::metadata::LevelFilter;
use tracing::{debug, error};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};

mod commands;

#[tokio::main]
async fn main() {
    let app = Application::parse();

    if let Err(err) = init_logging() {
        eprintln!("Unable to initialize logging: {:#}", err);
        std::process::exit(1);
    }

    let result = handle_command(app).await;

    match result {
        Ok(_) => debug!("Command completed successfully"),
        Err(err) => {
            error!("Command failed: {:?}", err);
            std::process::exit(1);
        }
    }
}

/// Initialize logging for the application.
///
/// Currently we have a straight forward logging setup that will log everything
/// that is level info and higher to stderr. Users are able to influence this by
/// exporting the `RUST_LOG` environment variable.
///
/// For example: for local development it is convenient to set the environment
/// variable to `RUST_LOG=am=trace,info`. This will display all log messages
/// within the `am` module, but will only show info for other modules.
fn init_logging() -> Result<()> {
    // The filter layer controls which log levels to display.
    let filter_layer = EnvFilter::from_default_env(); //.add_directive(LevelFilter::INFO.into());

    let log_layer = tracing_subscriber::fmt::layer().with_writer(io::stderr);

    Registry::default()
        .with(filter_layer)
        .with(log_layer)
        .try_init()
        .context("unable to initialize logger")?;

    Ok(())
}
