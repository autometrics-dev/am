use crate::commands::start::CLIENT;
use axum::body;
use axum::body::Body;
use axum::response::{IntoResponse, Response};
use http::{StatusCode, Uri};
use tracing::{debug, error, trace, warn};
use url::Url;

pub(crate) async fn proxy_handler(
    mut req: http::Request<Body>,
    upstream_base: Url,
) -> impl IntoResponse {
    let req_uri = req.uri().to_string();
    let method = req.method().to_string();

    trace!(req_uri=%req_uri, method=%method, "Proxying request");

    // NOTE: The username/password is not forwarded
    let mut url = upstream_base.join(req.uri().path()).unwrap();
    url.set_query(req.uri().query());
    *req.uri_mut() = Uri::try_from(url.as_str()).unwrap();

    let res = CLIENT.execute(req.try_into().unwrap()).await;

    match res {
        Ok(res) => {
            if res.status().is_server_error() {
                warn!(
                    method=%method,
                    req_uri=%req_uri,
                    upstream_uri=%res.url(),
                    status_code=%res.status(),
                    "Response from the upstream source returned a server error status code",
                );
            } else if res.status().is_client_error() {
                debug!(
                    method=%method,
                    req_uri=%req_uri,
                    upstream_uri=%res.url(),
                    status_code=%res.status(),
                    "Response from the upstream source returned a client error status code",
                );
            } else {
                trace!(
                    method=%method,
                    req_uri=%req_uri,
                    upstream_uri=%res.url(),
                    status_code=%res.status(),
                    "Response from the upstream source",
                );
            }

            convert_response(res).into_response()
        }
        Err(err) => {
            warn!(
                method=%method,
                req_uri=%req_uri,
                err=%err,
                "Unable to proxy request to upstream server",
            );
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
pub(crate) fn convert_response(req: reqwest::Response) -> Response {
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
