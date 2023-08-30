use anyhow::{Context, Result};
use axum::body::Body;
use axum::response::Redirect;
use axum::routing::{any, get};
use axum::{Router, Server};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::watch::Sender;
use tracing::{debug, info};
use url::Url;

mod explorer;
mod prometheus;
mod pushgateway;
mod util;

pub(crate) async fn start_web_server(
    listen_address: &SocketAddr,
    enable_prometheus: bool,
    enable_pushgateway: bool,
    prometheus_proxy_url: Option<Url>,
    tx: Sender<Option<SocketAddr>>,
) -> Result<()> {
    let is_proxying_prometheus = prometheus_proxy_url.is_some();
    let should_enable_prometheus = enable_prometheus && !is_proxying_prometheus;
    let mut app = Router::new()
        // Any calls to the root should be redirected to the explorer which is most likely what the user wants to use.
        .route("/", get(|| async { Redirect::temporary("/explorer/") }))
        .route(
            "/explorer",
            get(|| async { Redirect::permanent("/explorer/") }),
        )
        .route(
            "/graph",
            get(|req: http::Request<Body>| async move {
                let query = req.uri().query().unwrap_or_default();
                Redirect::temporary(&format!("/explorer/graph.html?{query}"))
            }),
        )
        .route("/explorer/", get(explorer::handler))
        .route("/explorer/*path", get(explorer::handler));

    // Proxy `/prometheus` to the upstream (local) prometheus instance
    if should_enable_prometheus {
        app = app
            .route("/prometheus/*path", any(prometheus::handler))
            .route("/prometheus", any(prometheus::handler));
    }

    // NOTE - this will override local prometheus routes if specified
    if is_proxying_prometheus {
        let prometheus_upstream_base = Arc::new(prometheus_proxy_url.clone().unwrap());

        // Define a handler that will proxy to an external Prometheus instance
        let handler = move |mut req: http::Request<Body>| {
            let upstream_base = prometheus_upstream_base.clone();
            // 1. Get the path and query from the request, since we need to strip out `/prometheus`
            let path_and_query = req
                .uri()
                .path_and_query()
                .map(|pq| pq.as_str())
                .unwrap_or("");
            if let Some(stripped_path) = path_and_query.strip_prefix("/prometheus") {
                let stripped_path_str = stripped_path.to_string();
                // 2. Remove the `/prometheus` prefix.
                let new_path_and_query =
                    http::uri::PathAndQuery::from_maybe_shared(stripped_path_str)
                        .expect("Invalid path");

                // 3. Create a new URI with the modified path.
                let mut new_uri_parts = req.uri().clone().into_parts();
                new_uri_parts.path_and_query = Some(new_path_and_query);

                let new_uri = http::Uri::from_parts(new_uri_parts).expect("Invalid URI");

                // 4. Replace the request's URI with the modified URI.
                *req.uri_mut() = new_uri;
            }
            async move { prometheus::handler_with_url(req, &upstream_base).await }
        };

        app = app
            .route("/prometheus/*path", any(handler.clone()))
            .route("/prometheus", any(handler));
    }

    if enable_pushgateway {
        app = app
            .route("/metrics", any(pushgateway::metrics_proxy_handler))
            .route("/pushgateway/*path", any(pushgateway::handler))
            .route("/pushgateway", any(pushgateway::handler));
    }

    let server = Server::try_bind(listen_address)
        .with_context(|| format!("failed to bind to {}", listen_address))?
        .serve(app.into_make_service());

    tx.send_replace(Some(server.local_addr()));

    debug!("Web server listening on {}", server.local_addr());

    info!("Explorer endpoint: http://{}", server.local_addr());

    if should_enable_prometheus {
        info!("Prometheus endpoint: http://127.0.0.1:9090/prometheus");
    }

    if is_proxying_prometheus {
        info!("Proxying to prometheus: {}", prometheus_proxy_url.unwrap());
    }

    if enable_pushgateway {
        info!("Pushgateway endpoint: http://127.0.0.1:9091/pushgateway");
    }

    // TODO: Add support for graceful shutdown
    // server.with_graceful_shutdown(shutdown_signal()).await?;
    server.await?;

    Ok(())
}
