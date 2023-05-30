use serde::Serialize;

//     let yml = format!(
// r#"
// global:
//     scrape_interval: 15s # Set the scrape interval to every 15 seconds. Default is every 1 minute.
//     evaluation_interval: 15s # Evaluate rules every 15 seconds. The default is every 1 minute.
// scrape_configs:
//   - job_name: "prometheus"
//     static_configs:
//     - targets: ["localhost:9090"] # this should be the address of the prom server that we will use

//   - job_name: "app"
//     static_configs:
//     - targets: ["{}"]
// "#,
//         endpoint
//     );

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
