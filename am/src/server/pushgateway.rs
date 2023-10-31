use crate::server::util::proxy_handler;
use autometrics::autometrics;
use axum::body::Body;
use axum::response::IntoResponse;
use url::Url;

#[autometrics]
pub(crate) async fn handler(req: http::Request<Body>) -> impl IntoResponse {
    let upstream_base = Url::parse("http://localhost:9091").unwrap();
    proxy_handler(req, upstream_base).await
}

#[autometrics]
pub(crate) async fn metrics_proxy_handler(req: http::Request<Body>) -> impl IntoResponse {
    let upstream_base = Url::parse("http://localhost:9091/pushgateway/metrics").unwrap();
    proxy_handler(req, upstream_base).await
}
