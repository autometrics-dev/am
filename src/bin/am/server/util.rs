use crate::commands::start::CLIENT;
use axum::body;
use axum::body::Body;
use axum::response::{IntoResponse, Response};
use http::{StatusCode, Uri};
use tracing::{debug, error, trace};
use url::Url;

pub(crate) async fn proxy_handler(
    mut req: http::Request<Body>,
    upstream_base: Url,
) -> impl IntoResponse {
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
