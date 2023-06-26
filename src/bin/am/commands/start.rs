use crate::downloader::{download_github_release, unpack, verify_checksum};
use crate::interactive;
use anyhow::{bail, Context, Result};
use autometrics_am::prometheus;
use axum::body::{self, Body};
use axum::extract::Path as AxumPath;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{any, get};
use axum::Router;
use clap::Parser;
use directories::ProjectDirs;
use futures_util::FutureExt;
use http::{StatusCode, Uri};
use include_dir::{include_dir, Dir};
use indicatif::MultiProgress;
use once_cell::sync::Lazy;
use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use std::vec;
use tempfile::NamedTempFile;
use tokio::{process, select};
use tracing::{debug, error, info, trace, warn};
use url::Url;

// Create a reqwest client that will be used to make HTTP requests. This allows
// for keep-alives if we are making multiple requests to the same host.
pub(crate) static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .user_agent(concat!("am/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .expect("Unable to create reqwest client")
});

#[derive(Parser, Clone)]
pub struct Arguments {
    /// The endpoint(s) that Prometheus will scrape.
    ///
    /// Multiple endpoints can be specified by separating them with a space.
    /// The endpoint can be provided in the following formats:
    /// - `:3000`. Defaults to `http`, `localhost` and `/metrics`.
    /// - `localhost:3000`. Defaults to `http`, and `/metrics`.
    /// - `https://localhost:3000`. Defaults to `/metrics`.
    /// - `https://localhost:3000/api/metrics`. No defaults.
    #[clap(value_parser = endpoint_parser, verbatim_doc_comment)]
    metrics_endpoints: Vec<Url>,

    /// The Prometheus version to use.
    #[clap(long, env, default_value = "v2.44.0")]
    prometheus_version: String,

    /// The listen address for the web server of am.
    ///
    /// This includes am's HTTP API, the explorer and the proxy to the Prometheus, Gateway, etc.
    #[clap(
        short,
        long,
        env,
        default_value = "127.0.0.1:6789",
        alias = "explorer-address"
    )]
    listen_address: SocketAddr,

    /// Enable pushgateway.
    ///
    /// Pushgateway accepts metrics from other applications and exposes these to
    /// Prometheus. This is useful for applications that cannot be scraped,
    /// either cause they are short-lived (like functions), or Prometheus cannot
    /// reach them (like client-side applications).
    #[clap(short, long, env)]
    enable_pushgateway: bool,

    /// The pushgateway version to use.
    #[clap(long, env, default_value = "v1.6.0")]
    pushgateway_version: String,
}

pub async fn handle_command(mut args: Arguments, mp: MultiProgress) -> Result<()> {
    if args.metrics_endpoints.is_empty() && args.enable_pushgateway {
        let endpoint = interactive::user_input("Endpoint")?;
        args.metrics_endpoints.push(endpoint_parser(&endpoint)?);
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

    if args.enable_pushgateway {
        args.metrics_endpoints
            .push(Url::parse("http://localhost:9091/pushgateway/metrics").unwrap());
    }

    // Start Prometheus server
    let prometheus_args = args.clone();
    let prometheus_local_data = local_data.clone();
    let prometheus_multi_progress = mp.clone();
    let prometheus_task = async move {
        let prometheus_version = prometheus_args.prometheus_version.trim_start_matches('v');

        info!("Using Prometheus version: {}", prometheus_version);

        let prometheus_path =
            prometheus_local_data.join(format!("prometheus-{prometheus_version}"));

        // Check if prometheus is available
        if !prometheus_path.exists() {
            info!("Cached version of Prometheus not found, downloading Prometheus");
            install_prometheus(
                &prometheus_path,
                prometheus_version,
                prometheus_multi_progress,
            )
            .await?;
            debug!("Downloaded Prometheus to: {:?}", &prometheus_path);
        } else {
            debug!("Found prometheus in: {:?}", prometheus_path);
        }

        let prometheus_config = generate_prom_config(prometheus_args.metrics_endpoints)?;
        start_prometheus(&prometheus_path, &prometheus_config).await
    };

    let pushgateway_task = if args.enable_pushgateway {
        let pushgateway_args = args.clone();
        let pushgateway_local_data = local_data.clone();
        let pushgateway_multi_progress = mp.clone();
        async move {
            let pushgateway_version = pushgateway_args.pushgateway_version.trim_start_matches('v');

            info!("Using pushgateway version: {}", pushgateway_version);

            let pushgateway_path =
                pushgateway_local_data.join(format!("pushgateway-{pushgateway_version}"));

            // Check if pushgateway is available
            if !pushgateway_path.exists() {
                info!("Cached version of pushgateway not found, downloading pushgateway");
                install_pushgateway(
                    &pushgateway_path,
                    pushgateway_version,
                    pushgateway_multi_progress,
                )
                .await?;
                debug!("Downloaded pushgateway to: {:?}", &pushgateway_path);
            } else {
                debug!("Found pushgateway in: {:?}", &pushgateway_path);
            }

            start_pushgateway(&pushgateway_path).await
        }
        .boxed()
    } else {
        async move { anyhow::Ok(()) }.boxed()
    };

    // Start web server for hosting the explorer, am api and proxies to the enabled services.
    let listen_address = args.listen_address;
    let web_server_task = async move { start_web_server(&listen_address, args).await };

    select! {
        biased;

        _ = tokio::signal::ctrl_c() => {
            debug!("sigint received by user, exiting...");
            Ok(())
        }

        Err(err) = prometheus_task => {
            bail!("Prometheus exited with an error: {err:?}");
        }

        Err(err) = pushgateway_task => {
            bail!("Pushgateway exited with an error: {err:?}");
        }

        Err(err) = web_server_task => {
            bail!("Web server exited with an error: {err:?}");
        }

        else => {
            Ok(())
        }
    }
}

/// Install the specified version of Prometheus into `prometheus_path`.
///
/// This function will first create a temporary file to download the Prometheus
/// archive into. Then it will verify the downloaded archive against the
/// downloaded checksum. Finally it will unpack the archive into
/// `prometheus_path`.
async fn install_prometheus(
    prometheus_path: &Path,
    prometheus_version: &str,
    multi_progress: MultiProgress,
) -> Result<()> {
    let (os, arch) = determine_os_and_arch()?;
    let base = format!("prometheus-{prometheus_version}.{os}-{arch}");
    let package = format!("{base}.tar.gz");
    let prefix = format!("{base}/");

    let mut prometheus_archive = NamedTempFile::new()?;

    let calculated_checksum = download_github_release(
        prometheus_archive.as_file(),
        "prometheus",
        "prometheus",
        prometheus_version,
        &package,
        &multi_progress,
    )
    .await?;

    verify_checksum(
        &calculated_checksum,
        "prometheus",
        "prometheus",
        prometheus_version,
        &package,
    )
    .await?;

    // Make sure we set the position to the beginning of the file so that we can
    // unpack it.
    prometheus_archive.as_file_mut().seek(SeekFrom::Start(0))?;

    unpack(
        prometheus_archive.as_file(),
        "prometheus",
        prometheus_path,
        &prefix,
        &multi_progress,
    )
    .await
}

/// Install the specified version of Pushgateway into `pushgateway_path`.
///
/// This function will first create a temporary file to download the Pushgateway
/// archive into. Then it will verify the downloaded archive against the
/// downloaded checksum. Finally it will unpack the archive into
/// `pushgateway_path`.
async fn install_pushgateway(
    pushgateway_path: &Path,
    pushgateway_version: &str,
    multi_progress: MultiProgress,
) -> Result<()> {
    let (os, arch) = determine_os_and_arch()?;

    let base = format!("pushgateway-{pushgateway_version}.{os}-{arch}");
    let package = format!("{base}.tar.gz");
    let prefix = format!("{base}/");

    let mut pushgateway_archive = NamedTempFile::new()?;

    let calculated_checksum = download_github_release(
        pushgateway_archive.as_file(),
        "prometheus",
        "pushgateway",
        pushgateway_version,
        &package,
        &multi_progress,
    )
    .await?;

    verify_checksum(
        &calculated_checksum,
        "prometheus",
        "pushgateway",
        pushgateway_version,
        &package,
    )
    .await?;

    // Make sure we set the position to the beginning of the file so that we can
    // unpack it.
    pushgateway_archive.as_file_mut().seek(SeekFrom::Start(0))?;

    unpack(
        pushgateway_archive.as_file(),
        "pushgateway",
        pushgateway_path,
        &prefix,
        &multi_progress,
    )
    .await
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

    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let num = COUNTER.fetch_add(1, Ordering::SeqCst);

    prometheus::ScrapeConfig {
        job_name: format!("app_{num}"),
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
    prometheus_path: &Path,
    prometheus_config: &prometheus::Config,
) -> Result<()> {
    // First write the config to a temp file
    let config_file_path = std::env::temp_dir().join("prometheus.yml");
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

    #[cfg(not(target_os = "windows"))]
    let program = "prometheus";
    #[cfg(target_os = "windows")]
    let program = "prometheus.exe";

    let prometheus_path = prometheus_path.join(program);

    debug!("Invoking prometheus at {}", prometheus_path.display());

    let mut child = process::Command::new(prometheus_path)
        .arg(format!("--config.file={}", config_file_path.display()))
        .arg("--web.listen-address=:9090")
        .arg("--web.enable-lifecycle")
        .arg("--web.external-url=http://localhost:6789/prometheus") // TODO: Make sure this matches with that is actually running.
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("Unable to start Prometheus")?;

    let status = child.wait().await?;

    if !status.success() {
        bail!("Prometheus exited with status {}", status)
    }

    Ok(())
}

/// Start a prometheus process. This will block until the Prometheus process
/// stops.
async fn start_pushgateway(pushgateway_path: &Path) -> Result<()> {
    info!("Starting Pushgateway");
    let mut child = process::Command::new(pushgateway_path.join("pushgateway"))
        .arg("--web.listen-address=:9091")
        .arg("--web.external-url=http://localhost:6789/pushgateway") // TODO: Make sure this matches with that is actually running.
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("Unable to start Pushgateway")?;

    let status = child.wait().await?;
    if !status.success() {
        anyhow::bail!("Pushgateway exited with status {}", status)
    }

    Ok(())
}

async fn start_web_server(listen_address: &SocketAddr, args: Arguments) -> Result<()> {
    let mut app = Router::new()
        // Any calls to the root should be redirected to the explorer which is most likely what the user wants to use.
        .route("/", get(|| async { Redirect::temporary("/explorer/") }))
        .route(
            "/explorer",
            get(|| async { Redirect::permanent("/explorer/") }),
        )
        .route("/explorer/", get(explorer_root_handler))
        .route("/explorer/*path", get(explorer_handler))
        .route("/prometheus/*path", any(prometheus_handler))
        .route("/prometheus", any(prometheus_handler));

    if args.enable_pushgateway {
        app = app
            .route("/pushgateway/*path", any(pushgateway_handler))
            .route("/pushgateway", any(pushgateway_handler));
    }

    let server = axum::Server::try_bind(listen_address)
        .with_context(|| format!("failed to bind to {}", listen_address))?
        .serve(app.into_make_service());

    debug!("Web server listening on {}", server.local_addr());

    info!("Explorer endpoint: http://{}", server.local_addr());
    info!("Prometheus endpoint: http://127.0.0.1:9090/prometheus");
    if args.enable_pushgateway {
        info!("Pushgateway endpoint: http://127.0.0.1:9091/pushgateway");
    }

    if !args.metrics_endpoints.is_empty() {
        let endpoints = args
            .metrics_endpoints
            .iter()
            .map(|endpoint| endpoint.to_string())
            .collect::<Vec<String>>()
            .join(", ");
        info!("Now sampling the following {endpoints} for metrics");
    }

    // TODO: Add support for graceful shutdown
    // server.with_graceful_shutdown(shutdown_signal()).await?;
    server.await?;

    Ok(())
}

static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/files/explorer");

/// This will serve the "index.html" file from the explorer directory.
///
/// This needs to be a separate handler since otherwise the Path extractor will
/// fail since the root does not have a path.
async fn explorer_root_handler() -> impl IntoResponse {
    serve_explorer("index.html").await
}

/// This will look at the path of the request and serve the corresponding file.
async fn explorer_handler(AxumPath(path): AxumPath<String>) -> impl IntoResponse {
    serve_explorer(&path).await
}

/// Server a specific file from the explorer directory. Returns 404 if the file
/// was not found.
async fn serve_explorer(path: &str) -> impl IntoResponse {
    trace!(?path, "Serving static file");

    match STATIC_DIR.get_file(path) {
        None => {
            warn!(?path, "Request file was not found in the explorer assets");
            StatusCode::NOT_FOUND.into_response()
        }
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

async fn prometheus_handler(req: http::Request<Body>) -> impl IntoResponse {
    let upstream_base = Url::parse("http://localhost:9090").unwrap();
    proxy_handler(req, upstream_base).await
}

async fn pushgateway_handler(req: http::Request<Body>) -> impl IntoResponse {
    let upstream_base = Url::parse("http://localhost:9091").unwrap();
    proxy_handler(req, upstream_base).await
}

async fn proxy_handler(mut req: http::Request<Body>, upstream_base: Url) -> impl IntoResponse {
    trace!(req_uri=?req.uri(),method=?req.method(),"Proxying request");

    // NOTE: The username/password is not forwarded
    let mut url = upstream_base.join(req.uri().path()).unwrap();
    url.set_query(req.uri().query());
    *req.uri_mut() = Uri::try_from(url.as_str()).unwrap();

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
