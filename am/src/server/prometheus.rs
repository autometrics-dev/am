use crate::server::util::proxy_handler;
use autometrics::autometrics;
use axum::body::Body;
use axum::response::IntoResponse;
use url::Url;

#[autometrics]
pub(crate) async fn handler(req: http::Request<Body>) -> impl IntoResponse {
    let upstream_base = url::Url::parse("http://localhost:9090").unwrap();
    proxy_handler(req, upstream_base).await
}

pub(crate) async fn handler_with_url(
    req: http::Request<Body>,
    upstream_base: &Url,
) -> impl IntoResponse {
    proxy_handler(req, upstream_base.clone()).await
}
