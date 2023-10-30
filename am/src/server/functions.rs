use autometrics::autometrics;
use axum::response::{IntoResponse, Response};
use axum::Json;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[autometrics]
pub(crate) async fn all_functions() -> Result<impl IntoResponse, AllFunctionError> {
    let functions = am_list::list_all_project_functions(
        std::env::current_dir()
            .map_err(|_| AllFunctionError::DirNotFound)?
            .as_path(),
    )
    .map_err(|err| AllFunctionError::AmListError(format!("{err:?}")))?;

    let mut output = vec![];

    for (path, (language, language_functions)) in functions {
        for func in language_functions {
            let mut value =
                serde_json::to_value(&func).map_err(|_| AllFunctionError::SerdeError)?;

            // this is in a separate block so the mutable reference gets dropped before we try to move the value in the last line
            {
                let obj = value.as_object_mut().ok_or(AllFunctionError::NonObject)?;

                obj.insert(
                    "language".to_string(),
                    serde_json::to_value(language).map_err(|_| AllFunctionError::SerdeError)?,
                );

                obj.insert(
                    "path".to_string(),
                    serde_json::to_value(path.to_string_lossy())
                        .map_err(|_| AllFunctionError::SerdeError)?,
                );
            }

            output.push(value);
        }
    }

    Ok(Json(output))
}

#[derive(Deserialize, Serialize, Debug, Error)]
#[serde(tag = "error", content = "details", rename_all = "snake_case")]
pub(crate) enum AllFunctionError {
    #[error("`FunctionInfo` needs to serialize to a `Value::Object`")]
    NonObject,

    #[error("unable to determinate current working directory")]
    DirNotFound,

    #[error("{0}")]
    AmListError(String),

    #[error("serde error")]
    SerdeError,
}

impl IntoResponse for AllFunctionError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(self)).into_response()
    }
}
