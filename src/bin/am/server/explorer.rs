use axum::body;
use axum::extract::Path;
use axum::response::{IntoResponse, Response};
use http::StatusCode;
use include_dir::{include_dir, Dir};
use tracing::{error, trace, warn};

static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/files/explorer");

/// This will serve the "index.html" file from the explorer directory.
///
/// This needs to be a separate handler since otherwise the Path extractor will
/// fail since the root does not have a path.
pub(crate) async fn root_handler() -> impl IntoResponse {
    serve_explorer("index.html").await
}

/// This will look at the path of the request and serve the corresponding file.
pub(crate) async fn handler(Path(path): Path<String>) -> impl IntoResponse {
    serve_explorer(&path).await
}

/// Server a specific file from the explorer directory. Returns 404 if the file
/// was not found.
async fn serve_explorer(path: &str) -> impl IntoResponse {
    trace!(?path, "Serving static file");

    match STATIC_DIR.get_file(path) {
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
