use anyhow::{Context, Result};
use axum::body::Body;
use axum::response::Redirect;
use axum::routing::{any, get};
use axum::{Router, Server};
use std::net::SocketAddr;
use tracing::{debug, info};

mod explorer;
mod prometheus;
mod pushgateway;
mod util;

pub(crate) async fn start_web_server(
    listen_address: &SocketAddr,
    enable_pushgateway: bool,
) -> Result<()> {
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
                let query = req.uri().query().unwrap_or("");
                Redirect::temporary(&format!("/explorer/graph.html?{query}"))
            }),
        )
        .route("/explorer/", get(explorer::handler))
        .route("/explorer/*path", get(explorer::handler))
        .route("/prometheus/*path", any(prometheus::handler))
        .route("/prometheus", any(prometheus::handler));

    if enable_pushgateway {
        app = app
            .route("/metrics", any(pushgateway::metrics_proxy_handler))
            .route("/pushgateway/*path", any(pushgateway::handler))
            .route("/pushgateway", any(pushgateway::handler));
    }

    let server = Server::try_bind(listen_address)
        .with_context(|| format!("failed to bind to {}", listen_address))?
        .serve(app.into_make_service());

    debug!("Web server listening on {}", server.local_addr());

    info!("Explorer endpoint: http://{}", server.local_addr());
    info!("Prometheus endpoint: http://127.0.0.1:9090/prometheus");

    if enable_pushgateway {
        info!("Pushgateway endpoint: http://127.0.0.1:9091/pushgateway");
    }

    // TODO: Add support for graceful shutdown
    // server.with_graceful_shutdown(shutdown_signal()).await?;
    server.await?;

    Ok(())
}
