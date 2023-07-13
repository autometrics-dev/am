use crate::parser::endpoint_parser;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
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
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Endpoint {
    #[serde(deserialize_with = "parse_maybe_shorthand")]
    pub url: Url,
    pub job_name: Option<String>,
    pub honor_labels: Option<bool>,
}

fn parse_maybe_shorthand<'de, D: Deserializer<'de>>(input: D) -> Result<Url, D::Error> {
    let input_str: String = Deserialize::deserialize(input)?;
    endpoint_parser(&input_str).map_err(Error::custom)
}