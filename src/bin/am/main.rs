use anyhow::{bail, Context, Result};
use autometrics_am::config::AmConfig;
use clap::Parser;
use commands::{handle_command, Application};
use interactive::IndicatifWriter;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::timeout;
use tracing::level_filters::LevelFilter;
use tracing::{debug, error, warn};
use tracing_subscriber::fmt::format;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, Registry};

mod commands;
mod dir;
mod downloader;
mod interactive;
mod server;
mod updater;

#[tokio::main]
async fn main() {
    let app = Application::parse();

    let (writer, multi_progress) = IndicatifWriter::new();

    if let Err(err) = init_logging(&app, writer) {
        eprintln!("Unable to initialize logging: {:#}", err);
        std::process::exit(1);
    }

    let task = if std::env::var_os("AM_NO_UPDATE").is_none() {
        tokio::task::spawn(updater::update_check())
    } else {
        tokio::task::spawn(async { /* intentionally left empty */ })
    };

    let config = match load_config(app.config_file.clone()).await {
        Ok(config) => config,
        Err(err) => {
            error!("Unable to load config: {:?}", err);
            std::process::exit(1);
        }
    };

    let result = handle_command(app, config, multi_progress).await;

    match result {
        Ok(_) => debug!("Command completed successfully"),
        Err(err) => {
            error!("Command failed: {:?}", err);
            std::process::exit(1);
        }
    }

    if let Err(err) = timeout(Duration::from_secs(1), task).await {
        warn!(?err, "background update check timed out");
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
fn init_logging(app: &Application, writer: IndicatifWriter) -> Result<()> {
    let (filter_layer, log_layer) = if app.verbose {
        let filter_layer = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::try_new("am=debug,info").unwrap());

        let log_layer = tracing_subscriber::fmt::layer().with_writer(writer).boxed();

        (filter_layer, log_layer)
    } else {
        let filter_layer = EnvFilter::default().add_directive(LevelFilter::INFO.into());

        // Create a custom field formatter, which only outputs the `message`
        // field, all other fields are ignored.
        let field_formatter = format::debug_fn(|writer, field, value| {
            if field.name() == "message" {
                write!(writer, "{value:?}")
            } else {
                Ok(())
            }
        });

        let log_layer = tracing_subscriber::fmt::layer()
            .fmt_fields(field_formatter)
            .without_time()
            .with_level(false)
            .with_span_events(format::FmtSpan::NONE)
            .with_target(false)
            .with_writer(writer)
            .boxed();

        (filter_layer, log_layer)
    };

    Registry::default()
        .with(filter_layer)
        .with(log_layer)
        .try_init()
        .context("unable to initialize logger")?;

    Ok(())
}

/// Try to load the config from the specified path. If the file doesn't exist it
/// will return a AmConfig with all its defaults set. If it is invalid toml file
/// it will return an error.
async fn load_config(config_file: Option<PathBuf>) -> Result<AmConfig> {
    let (path, is_default) = match config_file {
        Some(path) => (path, false),
        None => (PathBuf::from("./am.toml"), true),
    };

    debug!(?path, "Loading config");

    match tokio::fs::read_to_string(path).await {
        Ok(contents) => {
            debug!("Found config file, parsing");
            let config =
                toml::from_str(&contents).context("config file contains invalid toml contents")?;
            Ok(config)
        }
        Err(err) => {
            if is_default {
                debug!(?err, "No config file found, using defaults");
                Ok(AmConfig::default())
            } else {
                bail!("Unable to read config file: {}", err);
            }
        }
    }
}
