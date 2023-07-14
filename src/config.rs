use crate::parser::endpoint_parser;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
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
