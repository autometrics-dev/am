use crate::server::start_web_server;
use anyhow::{bail, Context, Result};
use clap::Parser;
use directories::ProjectDirs;
use std::net::SocketAddr;
use tokio::select;
use tokio::sync::watch;
use tracing::info;
use url::Url;

#[derive(Parser, Clone)]
pub struct CliArguments {
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

    /// The upstream Prometheus URL
    #[clap(long, env, alias = "prometheus-address")]
    prometheus_url: Option<Url>,

    #[clap(
        long,
        env,
        default_value = "https://explorer.autometrics.dev/static",
        help_heading = "Location for static assets used by the explorer"
    )]
    static_assets_url: Url,
}

#[derive(Debug, Clone)]
struct Arguments {
    listen_address: SocketAddr,
    prometheus_url: Option<Url>,
    static_assets_url: Url,
}

impl Arguments {
    fn new(args: CliArguments) -> Self {
        Arguments {
            listen_address: args.listen_address,
            prometheus_url: args.prometheus_url,
            static_assets_url: args.static_assets_url,
        }
    }
}

pub async fn handle_command(args: CliArguments) -> Result<()> {
    let args = Arguments::new(args);

    // First let's retrieve the directory for our application to store data in.
    let project_dirs =
        ProjectDirs::from("", "autometrics", "am").context("Unable to determine home directory")?;
    let local_data = project_dirs.data_local_dir().to_owned();

    // Make sure that the local data directory exists for our application.
    std::fs::create_dir_all(&local_data)
        .with_context(|| format!("Unable to create data directory: {:?}", local_data))?;

    let (tx, _) = watch::channel(None);

    // Start web server for hosting the explorer, am api and proxies to the enabled services.
    let web_server_task = async move {
        start_web_server(
            &args.listen_address,
            false,
            false,
            args.prometheus_url,
            args.static_assets_url,
            tx,
        )
        .await
    };

    select! {
        biased;

        _ = tokio::signal::ctrl_c() => {
            info!("SIGINT signal received, exiting...");
            Ok(())
        }

        Err(err) = web_server_task => {
            bail!("Web server exited with an error: {err:?}");
        }

        else => {
            Ok(())
        }
    }
}
