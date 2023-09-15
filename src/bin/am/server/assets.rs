use crate::server::util::proxy_handler;
use axum::body::Body;
use axum::response::IntoResponse;
use url::Url;

pub(crate) async fn handler(mut req: http::Request<Body>) -> impl IntoResponse {
    let upstream_base = Url::parse("https://explorer.autometrics.dev/static").unwrap();
    *req.uri_mut() = req
        .uri()
        .path_and_query()
        .unwrap()
        .as_str()
        .replace("/explorer/static", "/static")
        .parse()
        .unwrap();
    proxy_handler(req, upstream_base).await
}
