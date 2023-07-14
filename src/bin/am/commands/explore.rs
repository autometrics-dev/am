use anyhow::Result;
use autometrics_am::config::{endpoints_from_first_input, AmConfig, Endpoint};
use autometrics_am::parser::endpoint_parser;
use clap::Parser;
use tracing::info;
use url::Url;

#[derive(Parser, Clone)]
pub struct CliArguments {
    /// The endpoint(s) that will be passed to Explorer
    ///
    /// Multiple endpoints can be specified by separating them with a space.
    /// The endpoint can be provided in the following formats:
    /// - `:3000`. Defaults to `http`, `localhost` and `/metrics`.
    /// - `localhost:3000`. Defaults to `http`, and `/metrics`.
    /// - `https://localhost:3000`. Defaults to `/metrics`.
    /// - `https://localhost:3000/api/metrics`. No defaults.
    #[clap(value_parser = endpoint_parser, verbatim_doc_comment)]
    metrics_endpoints: Vec<Url>,

    /// Which endpoint to open in the browser
    #[clap(long, env)]
    explorer_endpoint: Option<Url>,
}

#[derive(Debug, Clone)]
struct Arguments {
    metrics_endpoints: Vec<Endpoint>,
    explorer_endpoint: Url,
}

impl Arguments {
    fn new(args: CliArguments, config: AmConfig) -> Self {
        Arguments {
            metrics_endpoints: endpoints_from_first_input(args.metrics_endpoints, config.endpoints),
            explorer_endpoint: args
                .explorer_endpoint
                .or(config.explorer_endpoint)
                .unwrap_or_else(|| Url::parse("http://localhost:6789/explorer").unwrap()), // .unwrap is safe because we control the input
        }
    }
}

pub async fn handle_command(args: CliArguments, config: AmConfig) -> Result<()> {
    let mut args = Arguments::new(args, config);

    let query: String = args
        .metrics_endpoints
        .into_iter()
        .map(|e| format!("_prometheusUrl={}", e.url))
        .collect::<Vec<_>>()
        .join("&");

    let url = &mut args.explorer_endpoint;
    url.set_query(if !query.is_empty() {
        Some(query.as_str())
    } else {
        None
    });

    if open::that(url.as_str()).is_err() {
        info!(
            "Unable to open browser, open the following URL in your browser: {}",
            url.as_str()
        );
    }

    Ok(())
}
