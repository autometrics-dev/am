use serde::Deserialize;

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
    pub url: url::Url,

    pub job_name: Option<String>,
}
