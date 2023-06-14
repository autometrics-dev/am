use anyhow::{Context, Result};
use clap::Parser;
use commands::{handle_command, Application};
use interactive::IndicatifWriter;
use tracing::level_filters::LevelFilter;
use tracing::{debug, error};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};

mod commands;
mod interactive;

#[tokio::main]
async fn main() {
    let (writer, multi_progress) = IndicatifWriter::new();

    let app = Application::parse();

    if let Err(err) = init_logging(writer) {
        eprintln!("Unable to initialize logging: {:#}", err);
        std::process::exit(1);
    }

    let result = handle_command(app, multi_progress).await;
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
/// Currently, we check for the `RUST_LOG` environment variable and use that for
/// logging. If it isn't set or contains a invalid directive, we will show _all_
/// logs from INFO level.
///
/// For example: for local development it is convenient to set the environment
/// variable to `RUST_LOG=am=trace,info`. This will display all log messages
/// within the `am` module, but will only show info for other modules.
fn init_logging(writer: IndicatifWriter) -> Result<()> {
    // The filter layer controls which log levels to display.
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::default().add_directive(LevelFilter::INFO.into()));

    let log_layer = tracing_subscriber::fmt::layer().with_writer(writer);

    Registry::default()
        .with(filter)
        .with(log_layer)
        .try_init()
        .context("unable to initialize logger")?;

    Ok(())
}
