use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Config {
    pub global: GlobalConfig,
    pub scrape_configs: Vec<ScrapeConfig>,
}

#[derive(Debug, Serialize)]
pub struct GlobalConfig {
    pub scrape_interval: String,
    pub evaluation_interval: String,
}

#[derive(Debug, Serialize)]
pub struct ScrapeConfig {
    pub job_name: String,
    pub static_configs: Vec<StaticScrapeConfig>,
    pub metrics_path: Option<String>,
    pub scheme: Option<Scheme>,
    pub honor_labels: Option<bool>,
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
