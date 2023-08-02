use crate::interactive::{confirm, confirm_optional, user_input, user_input_optional};
use anyhow::{bail, Context, Result};
use autometrics_am::config::{AmConfig, Endpoint};
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tracing::info;
use url::Url;

#[derive(Parser, Clone)]
pub struct Arguments {
    /// Where the file should be outputted to. Defaults to current directory
    #[clap(long, env, default_value = "./am.toml")]
    output: PathBuf,

    /// Whenever to forcefully override an existing `am.toml` file, if it already exists
    #[clap(long, env)]
    force: bool,
}

pub async fn handle_command(args: Arguments) -> Result<()> {
    if args.output.exists() && !args.force {
        bail!("Output file already exists. Supply --force to override");
    }

    let mut endpoints = vec![];

    while confirm("Do you want to add (more) endpoints?")? {
        endpoints.push(prompt_endpoint()?);
    }

    let pushgateway_enabled =
        confirm_optional("Do you want to enable the Pushgateway (optional)?")?;
    let scrape_interval = prompt_scrape_interval()?;

    let cfg = AmConfig {
        endpoints: if endpoints.is_empty() {
            None
        } else {
            Some(endpoints)
        },
        pushgateway_enabled,
        prometheus_scrape_interval: scrape_interval,
    };

    let config = toml::to_string(&cfg)?;
    fs::write(&args.output, config).context("failed to write file to disk")?;

    info!("Successfully written config to {}", args.output.display());
    Ok(())
}

fn prompt_endpoint() -> Result<Endpoint> {
    let endpoint = user_input("Enter a metrics endpoint URL")?;
    let job_name = user_input_optional("Enter job name (optional)")?;
    let honor_labels = confirm_optional("honor_labels (optional)")?;
    let scrape_interval = prompt_scrape_interval()?;

    Ok(Endpoint {
        url: Url::parse(&endpoint)?,
        job_name,
        honor_labels,
        prometheus_scrape_interval: scrape_interval,
    })
}

fn prompt_scrape_interval() -> Result<Option<Duration>> {
    let scrape_interval: Option<u64> =
        user_input_optional("Scrape Interval in seconds (leave empty for default)")?
            .and_then(|i| i.parse().ok());

    Ok(scrape_interval.map(|input| Duration::from_secs(input)))
}
