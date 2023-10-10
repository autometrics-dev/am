use anyhow::Result;
use clap::Parser;
use tracing::info;
use url::Url;

#[derive(Parser, Clone)]
pub struct Arguments {
    /// The Prometheus endpoint that will be passed to Explorer
    #[clap(long, env)]
    prometheus_endpoint: Option<Url>,

    /// Which endpoint to open in the browser
    #[clap(long, env, default_value = "https://explorer.autometrics.dev/")]
    explorer_endpoint: Url,
}

pub async fn handle_command(mut args: Arguments) -> Result<()> {
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
