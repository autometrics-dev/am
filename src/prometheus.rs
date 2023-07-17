use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Serialize)]
pub struct Config {
    pub global: GlobalConfig,
    pub scrape_configs: Vec<ScrapeConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_files: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct GlobalConfig {
    #[serde(with = "humantime_serde")]
    pub scrape_interval: Duration,
    pub evaluation_interval: String,
}

#[derive(Debug, Serialize)]
pub struct ScrapeConfig {
    pub job_name: String,
    pub static_configs: Vec<StaticScrapeConfig>,
    pub metrics_path: Option<String>,
    pub scheme: Option<Scheme>,
    pub honor_labels: Option<bool>,

    #[serde(
        default,
        with = "humantime_serde::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub scrape_interval: Option<Duration>,
}

#[derive(Debug, Serialize)]
pub struct StaticScrapeConfig {
    pub targets: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Scheme {
    Http,
    Https,
}
