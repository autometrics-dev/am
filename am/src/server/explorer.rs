use axum::body;
use axum::extract::Path;
use axum::response::{IntoResponse, Response};
use http::StatusCode;
use include_dir::{include_dir, Dir};
use tracing::{error, trace, warn};

static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../files/explorer");

pub(crate) async fn handler(optional_path: Option<Path<String>>) -> impl IntoResponse {
    let path = optional_path.map_or_else(|| "index.html".to_string(), |path| path.0);

    trace!(?path, "Serving static file");

    match STATIC_DIR.get_file(&path) {
        None => {
            warn!(?path, "Request file was not found in the explorer assets");
            StatusCode::NOT_FOUND.into_response()
        }
        Some(file) => Response::builder()
            .status(StatusCode::OK)
            .body(body::boxed(body::Full::from(file.contents())))
            .map(|res| res.into_response())
            .unwrap_or_else(|err| {
                error!("Failed to build response: {}", err);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }),
    }
}
