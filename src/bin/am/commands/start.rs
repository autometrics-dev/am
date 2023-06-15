use anyhow::{anyhow, bail, Context, Result};
use autometrics_am::prometheus;
use axum::body::{self, Body};
use axum::extract::Path;
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use axum::Router;
use clap::Parser;
use dialoguer::theme::SimpleTheme;
use dialoguer::Input;
use directories::ProjectDirs;
use flate2::read::GzDecoder;
use futures_util::future::join_all;
use http::{StatusCode, Uri};
use include_dir::{include_dir, Dir};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use once_cell::sync::Lazy;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use std::vec;
use tokio::process;
use tracing::{debug, error, info, trace, warn};
use url::Url;

// Create a reqwest client that will be used to make HTTP requests. This allows
// for keep-alives if we are making multiple requests to the same host.
static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .user_agent(concat!("am/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("Unable to create reqwest client")
});

#[derive(Parser, Clone)]
pub struct Arguments {
    /// The endpoint(s) that Prometheus will scrape.
    #[clap(value_parser = endpoint_parser)]
    metrics_endpoints: Vec<Url>,

    /// The Prometheus version to use.
    #[clap(long, env, default_value = "v2.44.0")]
    prometheus_version: String,

    /// The listen address for the web server of am.
    ///
    /// This includes am's HTTP API, the explorer and the proxy to the Prometheus, Gateway, etc.
    #[clap(short, long, env, default_value = "127.0.0.1:6789")]
    listen_address: SocketAddr,

    /// Startup the gateway as well.
    // TODO: Actually implement that we use this
    #[clap(short, long, env)]
    enable_gateway: bool,
}

pub async fn handle_command(mut args: Arguments) -> Result<()> {
    if args.metrics_endpoints.is_empty() && args.enable_gateway {
        let endpoint: String = Input::with_theme(&SimpleTheme)
            .with_prompt("Endpoint")
            .interact()?;

        args.metrics_endpoints.push(Url::parse(&endpoint)?);
    }

    // First let's retrieve the directory for our application to store data in.
    let project_dirs =
        ProjectDirs::from("", "autometrics", "am").context("Unable to determine home directory")?;
    let local_data = project_dirs.data_local_dir().to_owned();

    // Make sure that the local data directory exists for our application.
    std::fs::create_dir_all(&local_data)
        .with_context(|| format!("Unable to create data directory: {:?}", local_data))?;

    info!("Checking if provided metrics endpoints work...");

    // check if the provided endpoint works
    for endpoint in &args.metrics_endpoints {
        if let Err(err) = check_endpoint(endpoint).await {
            warn!(?endpoint, "Failed to contact endpoint: {err:?}");
        }
    }

    let mut handles = vec![];

    // Start Prometheus server
    let prometheus_args = args.clone();
    let prometheus_local_data = local_data.clone();
    let prometheus_handle = tokio::spawn(async move {
        let prometheus_version = args.prometheus_version.trim_start_matches('v');

        info!("Using Prometheus version: {}", prometheus_version);

        let prometheus_path =
            prometheus_local_data.join(format!("prometheus-{}", prometheus_version));

        // Check if prom is available at "some" location
        if !prometheus_path.exists() {
            info!("Downloading prometheus");
            download_prometheus(&prometheus_path, prometheus_version).await?;
            info!("Downloaded to: {:?}", &prometheus_path);
        }

        let prometheus_config = generate_prom_config(prometheus_args.metrics_endpoints)?;
        start_prometheus(&prometheus_path, &prometheus_config).await
    });
    handles.push(prometheus_handle);

    // Start web server for hosting the explorer, am api and proxies to the enabled services.
    let listen_address = args.listen_address;
    let web_server_handle = tokio::spawn(async move { start_web_server(&listen_address).await });
    handles.push(web_server_handle);

    join_all(handles).await;

    Ok(())
}

/// Download the specified Prometheus version from GitHub and extract the
/// archive into `prometheus_path`.
async fn download_prometheus(prometheus_path: &PathBuf, prometheus_version: &str) -> Result<()> {
    let (os, arch) = determine_os_and_arch()?;

    // TODO: Grab the checksum file and retrieve the checksum for the archive
    let archive_path = {
        let tmp_file = tempfile::NamedTempFile::new()?;
        let mut res = CLIENT
            .get(format!("https://github.com/prometheus/prometheus/releases/download/v{prometheus_version}/prometheus-{prometheus_version}.{os}-{arch}.tar.gz"))
            .send()
            .await?
            .error_for_status()?;

        let total_size = res
            .content_length()
            .ok_or_else(|| anyhow!("didn't receive content length"))?;
        let mut downloaded = 0;

        let pb = ProgressBar::new(total_size);

        // https://github.com/console-rs/indicatif/blob/HEAD/examples/download.rs#L12
        pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
            .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
            .progress_chars("=> "));

        let file = File::create(&tmp_file)?;
        let mut buffer = BufWriter::new(file);

        while let Some(ref chunk) = res.chunk().await? {
            buffer.write_all(chunk)?;

            let new_size = (downloaded + chunk.len() as u64).min(total_size);
            downloaded = new_size;

            pb.set_position(downloaded);
        }

        pb.finish_and_clear();
        info!("Successfully downloaded Prometheus");
        tmp_file
    };

    let file = File::open(archive_path)?;

    let tar_file = GzDecoder::new(file);
    let mut ar = tar::Archive::new(tar_file);

    // This prefix will be removed from the files in the archive.
    let prefix = format!("prometheus-{prometheus_version}.{os}-{arch}/");

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner());
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_message("Unpacking...");

    for entry in ar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;

        pb.set_message(format!("Unpacking {}", path.display()));
        debug!("Unpacking {}", path.display());

        // Remove the prefix and join it with the base directory.
        let path = path.strip_prefix(&prefix)?.to_owned();
        let path = prometheus_path.join(path);

        entry.unpack(&path)?;
    }

    pb.finish_and_clear();
    Ok(())
}

/// Translates the OS and arch provided by Rust to the convention used by
/// Prometheus.
fn determine_os_and_arch() -> Result<(&'static str, &'static str)> {
    use std::env::consts::{ARCH, OS};

    let os = match OS {
        "linux" => "linux",
        "macos" => "darwin",
        "windows" => "windows",
        "freebsd" => "freebsd",
        "netbsd" => "netbsd",
        "openbsd" => "openbsd",
        "dragonfly" => "dragonfly",
        _ => bail!(format!("Unsupported OS: {}", ARCH)),
    };

    let arch = match ARCH {
        "x86" => "386",
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "s390x" => "s390x",
        "powerpc64" => "powerpc64", // NOTE: Do we use this one, or the le one?
        // "mips" => "mips", // NOTE: Not sure which mips to pick in this situation
        // "arm" => "arm", // NOTE: Not sure which arm to pick in this situation
        _ => bail!(format!("Unsupported architecture: {}", ARCH)),
    };

    Ok((os, arch))
}

/// Generate a Prometheus configuration file.
///
/// For now this will expand a simple template and only has support for a single
/// endpoint.
fn generate_prom_config(metric_endpoints: Vec<Url>) -> Result<prometheus::Config> {
    let scrape_configs = metric_endpoints.iter().map(to_scrape_config).collect();

    let config = prometheus::Config {
        global: prometheus::GlobalConfig {
            scrape_interval: "15s".to_string(),
            evaluation_interval: "15s".to_string(),
        },
        scrape_configs,
    };

    Ok(config)
}

/// Convert an URL to a metric endpoint.
///
/// Scrape config only supports http and https atm.
fn to_scrape_config(metric_endpoint: &Url) -> prometheus::ScrapeConfig {
    let scheme = match metric_endpoint.scheme() {
        "http" => Some(prometheus::Scheme::Http),
        "https" => Some(prometheus::Scheme::Https),
        _ => None,
    };

    let mut metrics_path = metric_endpoint.path();
    if metrics_path.is_empty() {
        metrics_path = "/metrics";
    }

    let host = match metric_endpoint.port() {
        Some(port) => format!("{}:{}", metric_endpoint.host_str().unwrap(), port),
        None => metric_endpoint.host_str().unwrap().to_string(),
    };

    prometheus::ScrapeConfig {
        job_name: "app".to_string(),
        static_configs: vec![prometheus::StaticScrapeConfig {
            targets: vec![host],
        }],
        metrics_path: Some(metrics_path.to_string()),
        scheme,
    }
}

/// Checks whenever the endpoint works
async fn check_endpoint(url: &Url) -> Result<()> {
    let response = CLIENT
        .get(url.as_str())
        .timeout(Duration::from_secs(5))
        .send()
        .await?;

    if !response.status().is_success() {
        bail!("endpoint did not return 2xx status code");
    }

    Ok(())
}

/// Start a prometheus process. This will block until the Prometheus process
/// stops.
async fn start_prometheus(
    prometheus_binary_path: &PathBuf,
    prometheus_config: &prometheus::Config,
) -> Result<()> {
    // First write the config to a temp file

    let config_file_path = PathBuf::from("/tmp/prometheus.yml");
    let config_file = File::create(&config_file_path)?;
    debug!(
        path = ?config_file_path,
        "Created temporary file for Prometheus config serialization"
    );
    serde_yaml::to_writer(&config_file, &prometheus_config)?;

    // TODO: Capture prometheus output into a internal buffer and expose it
    // through an api.
    // TODO: Change the working directory, maybe make it configurable?

    info!("Starting prometheus");
    let mut child = process::Command::new(prometheus_binary_path.join("prometheus"))
        .arg(format!("--config.file={}", config_file_path.display()))
        .arg("--web.listen-address=:9090")
        .arg("--web.enable-lifecycle")
        .arg("--web.external-url=http://localhost:6789/prometheus") // TODO: Make sure this matches with that is actually running.
        .spawn()
        .context("Unable to start Prometheus")?;

    let status = child.wait().await?;
    if !status.success() {
        anyhow::bail!("Prometheus exited with status {}", status)
    }

    Ok(())
}

async fn start_web_server(listen_address: &SocketAddr) -> Result<()> {
    let app = Router::new()
        // .route("/api/ ... ") // This can expose endpoints that the ui app can call
        .route("/explorer/*path", get(explorer_handler))
        .route("/prometheus/*path", any(prometheus_handler));

    let server = axum::Server::try_bind(listen_address)
        .with_context(|| format!("failed to bind to {}", listen_address))?
        .serve(app.into_make_service());

    debug!("Web server listening on {}", server.local_addr());

    // TODO: Add support for graceful shutdown
    // server.with_graceful_shutdown(shutdown_signal()).await?;
    server.await?;

    Ok(())
}

static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/files/explorer");

async fn explorer_handler(Path(path): Path<String>) -> impl IntoResponse {
    let path = path.trim_start_matches('/');

    trace!("Serving static file {}", path);

    match STATIC_DIR.get_file(path) {
        None => StatusCode::NOT_FOUND.into_response(),
        Some(file) => Response::builder()
            .status(StatusCode::OK)
            .body(body::boxed(body::Full::from(file.contents())))
            .map(|res| res.into_response())
            .unwrap_or_else(|err| {
                error!("Failed to build response: {}", err);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }),
    }
}

async fn prometheus_handler(mut req: http::Request<Body>) -> impl IntoResponse {
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or_else(|| req.uri().path());

    // TODO hardcoded for now
    let uri = format!("http://127.0.0.1:9090{}", path_query);

    trace!("Proxying request to {}", uri);

    *req.uri_mut() = Uri::try_from(uri).unwrap();

    let res = CLIENT.execute(req.try_into().unwrap()).await;

    match res {
        Ok(res) => {
            if !res.status().is_success() {
                debug!(
                    "Response from the upstream source returned a non-success status code for {}: {:?}",
                    res.url(), res.status()
                );
            }

            convert_response(res).into_response()
        }
        Err(err) => {
            error!("Error proxying request: {:?}", err);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Convert a reqwest::Response into a axum_core::Response.
///
/// If the Response builder is unable to create a Response, then it will log the
/// error and return a http status code 500.
///
/// We cannot implement this as an Into or From trait since both types are
/// foreign to this code.
fn convert_response(req: reqwest::Response) -> Response {
    let mut builder = http::Response::builder().status(req.status());

    // Calling `headers_mut` is safe here because we're constructing a new
    // Response from scratch and it will only return `None` if the builder is in
    // a Error state.
    let headers = builder.headers_mut().unwrap();
    for (name, value) in req.headers() {
        // Insert all the headers that were in the response from the upstream.
        headers.insert(name, value.clone());
    }

    // TODO: Do we need to rewrite some headers, such as host?

    match builder.body(body::StreamBody::from(req.bytes_stream())) {
        Ok(res) => res.into_response(),
        Err(err) => {
            error!("Error converting response: {:?}", err);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Parses the input string into a Url. This uses a custom parser to allow for
/// some more flexible input.
///
/// Parsing adheres to the following rules:
/// - The protocol should only allow for http and https, where http is the
///   default.
/// - The port should follow the default for the protocol, 80 for http and 443
///   for https.
/// - The path should default to /metrics if the path is empty. It should not be
///   appended if a path is already there.
fn endpoint_parser(input: &str) -> Result<Url> {
    let mut input = input.to_owned();

    if input.starts_with(':') {
        // Prepend http://localhost if the input starts with a colon.
        input = format!("http://localhost{}", input);
    }

    // Prepend http:// if the input does not contain ://. This is a rather naive
    // check, but it should suffice for our purposes.
    if !input.contains("://") {
        input = format!("http://{}", input);
    }

    let mut url =
        Url::parse(&input).with_context(|| format!("Unable to parse endpoint {}", input))?;

    //  Note that this should never be Err(_) since we're always adding http://
    // in front of the input and thus making sure it is not a "cannot-be-a-base"
    // URL.
    if url.path() == "" || url.path() == "/" {
        url.set_path("/metrics");
    }

    if url.scheme() != "http" && url.scheme() != "https" {
        bail!("unsupported protocol {}", url.scheme());
    }

    Ok(url)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    #[rstest]
    #[case("127.0.0.1", "http://127.0.0.1:80/metrics")]
    #[case("https://127.0.0.1", "https://127.0.0.1:443/metrics")]
    #[case("localhost:3030", "http://localhost:3030/metrics")]
    #[case("localhost:3030/api/metrics", "http://localhost:3030/api/metrics")]
    #[case(
        "localhost:3030/api/observability",
        "http://localhost:3030/api/observability"
    )]
    #[case(":3000", "http://localhost:3000/metrics")]
    #[case(":3030/api/observability", "http://localhost:3030/api/observability")]
    fn endpoint_parser_ok(#[case] input: &str, #[case] expected: url::Url) {
        let result = super::endpoint_parser(input).expect("expected no error");
        assert_eq!(expected, result);
    }

    #[rstest]
    #[case("ftp://localhost")]
    #[case("not a valid url at all")]
    fn endpoint_parser_error(#[case] input: &str) {
        let _ = super::endpoint_parser(input).expect_err("expected a error");
        // We're not checking which specific error occurred, just that a error
        // occurred.
    }
}
