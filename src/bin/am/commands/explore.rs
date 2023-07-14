use anyhow::Result;
use autometrics_am::config::AmConfig;
use clap::Parser;
use tracing::info;
use url::Url;

#[derive(Parser, Clone)]
pub struct CliArguments {
    /// The Prometheus endpoint that will be passed to Explorer
    prometheus_endpoint: Option<Url>,

    /// Which endpoint to open in the browser
    #[clap(long, env)]
    explorer_endpoint: Option<Url>,
}

#[derive(Debug, Clone)]
struct Arguments {
    prometheus_endpoint: Option<Url>,
    explorer_endpoint: Url,
}

impl Arguments {
    fn new(args: CliArguments, config: AmConfig) -> Self {
        Arguments {
            prometheus_endpoint: args.prometheus_endpoint.or(config.prometheus_endpoint),
            explorer_endpoint: args
                .explorer_endpoint
                .unwrap_or_else(|| Url::parse("http://localhost:6789/explorer").unwrap()), // .unwrap is safe because we control the input
        }
    }
}

pub async fn handle_command(args: CliArguments, config: AmConfig) -> Result<()> {
    let mut args = Arguments::new(args, config);

    let url = &mut args.explorer_endpoint;

    if let Some(prom_url) = args.prometheus_endpoint {
        let query = format!("prometheusUrl={}", prom_url.as_str());
        url.set_query(Some(&query));
    }

    if open::that(url.as_str()).is_err() {
        info!(
            "Unable to open browser, open the following URL in your browser: {}",
            url.as_str()
        );
    }

    Ok(())
}
