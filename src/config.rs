use crate::parser::endpoint_parser;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use url::Url;

/// This struct represents the am.toml configuration. Most properties in here
/// are optional so that the user only specifies the ones that they want in that
/// file.
#[derive(Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct AmConfig {
    /// The endpoints that will be scraped by the Prometheus server.
    #[serde(rename = "endpoint")]
    pub endpoints: Option<Vec<Endpoint>>,

    /// Prometheus endpoint which should get passed to Dora when invoking `am explore`
    pub prometheus_endpoint: Option<Url>,

    /// Startup the pushgateway.
    pub pushgateway_enabled: Option<bool>,

    /// The default scrape interval for all Prometheus endpoints.
    #[serde(default, with = "humantime_serde::option")]
    pub prometheus_scrape_interval: Option<Duration>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Endpoint {
    /// The URL of the endpoint that will be scraped by the Prometheus server.
    /// Can use shorthand notation for the URL, e.g. `:3000`.
    #[serde(deserialize_with = "parse_maybe_shorthand")]
    pub url: Url,

    /// The job name as it appears in Prometheus. This value will be added to
    /// the scraped metrics as a label.
    pub job_name: Option<String>,

    pub honor_labels: Option<bool>,

    /// The scrape interval for this endpoint.
    #[serde(default, with = "humantime_serde::option")]
    pub prometheus_scrape_interval: Option<Duration>,
}

fn parse_maybe_shorthand<'de, D: Deserializer<'de>>(input: D) -> Result<Url, D::Error> {
    let input_str: String = Deserialize::deserialize(input)?;
    endpoint_parser(&input_str).map_err(Error::custom)
}

/// If the user specified an endpoint using args, then use those.
/// Otherwise, use the endpoint configured in the config file. And
/// fallback to an empty list if neither are configured.
pub fn endpoints_from_first_input(args: Vec<Url>, config: Option<Vec<Endpoint>>) -> Vec<Endpoint> {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    if !args.is_empty() {
        args.into_iter()
            .map(|url| {
                let num = COUNTER.fetch_add(1, Ordering::SeqCst);
                Endpoint {
                    url,
                    job_name: Some(format!("am_{num}")),
                    honor_labels: Some(false),
                    prometheus_scrape_interval: None,
                }
            })
            .collect()
    } else if let Some(endpoints) = config {
        endpoints
            .into_iter()
            .map(|endpoint| {
                let job_name = endpoint.job_name.unwrap_or_else(|| {
                    format!("am_{num}", num = COUNTER.fetch_add(1, Ordering::SeqCst))
                });

                Endpoint {
                    url: endpoint.url,
                    job_name: Some(job_name),
                    honor_labels: endpoint.honor_labels,
                    prometheus_scrape_interval: endpoint.prometheus_scrape_interval,
                }
            })
            .collect()
    } else {
        Vec::new()
    }
}
