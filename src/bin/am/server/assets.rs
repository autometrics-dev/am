use crate::server::util::proxy_handler;
use axum::body::Body;
use axum::response::IntoResponse;
use url::Url;

pub async fn handler(mut req: http::Request<Body>, upstream_base: Url) -> impl IntoResponse {
    *req.uri_mut() = req
        .uri()
        .path_and_query()
        .unwrap()
        .as_str()
        .replace("/explorer/static", "/static")
        .parse()
        .unwrap();
    proxy_handler(req, upstream_base.clone()).await
}
