use crate::server::util::proxy_handler;
use axum::body::Body;
use axum::response::IntoResponse;
use url::Url;

pub(crate) async fn handler(req: http::Request<Body>) -> impl IntoResponse {
    let upstream_base = Url::parse("http://localhost:9091").unwrap();
    proxy_handler(req, upstream_base).await
}

pub(crate) async fn metrics_proxy_handler(req: http::Request<Body>) -> impl IntoResponse {
    let upstream_base = Url::parse("http://localhost:9091/pushgateway/metrics").unwrap();
    proxy_handler(req, upstream_base).await
}
