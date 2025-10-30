use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use toml_edit::TomlError;
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Not Found: {0}")]
    NotFound(String),

    #[error("Bad Request: {0}")]
    BadRequest(String),

    #[error("Io Error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Database Error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Python Error: {0}")]
    Python(#[from] pyo3::PyErr),

    #[error("Strategy Error: {0}")]
    Strategy(#[from] libloading::Error),

    #[error("Internal Error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match &self {
            AppError::NotFound(msg) => {
                tracing::warn!(
                    error_type = %"NotFound",
                    status_code = %StatusCode::NOT_FOUND,
                    message = %msg,
                    "Resource not found"
                );
                (StatusCode::NOT_FOUND, "NotFound", msg.clone())
            }
            AppError::BadRequest(msg) => {
                tracing::warn!(
                    error_type = %"BadRequest",
                    status_code = %StatusCode::BAD_REQUEST,
                    message = %msg,
                    "Bad Request"
                );
                (StatusCode::BAD_REQUEST, "BadRequest", msg.clone())
            }
            AppError::IO(err) => {
                let msg = err.to_string();
                tracing::error!(
                    error_type = %"IO",
                    status_code = %StatusCode::INTERNAL_SERVER_ERROR,
                    message = %msg,
                    "IO operation failed"
                );
                (StatusCode::INTERNAL_SERVER_ERROR, "IO", msg)
            }
            AppError::Database(err) => {
                let msg = err.to_string();
                tracing::error!(
                    error_type = %"Database",
                    status_code = %StatusCode::INTERNAL_SERVER_ERROR,
                    message = %msg,
                    "Database operation failed"
                );
                (StatusCode::INTERNAL_SERVER_ERROR, "Database", msg)
            }
            AppError::Python(err) => {
                let msg = err.to_string();
                tracing::error!(
                    error_type = %"Python",
                    status_code = %StatusCode::INTERNAL_SERVER_ERROR,
                    message = %msg,
                    "Python operation failed"
                );
                (StatusCode::INTERNAL_SERVER_ERROR, "Python", msg)
            }
            AppError::Strategy(err) => {
                let msg = err.to_string();
                tracing::error!(
                    error_type = %"Strategy",
                    status_code = %StatusCode::INTERNAL_SERVER_ERROR,
                    message = %msg,
                    "Strategy operation failed"
                );
                (StatusCode::INTERNAL_SERVER_ERROR, "Strategy", msg)
            }
            AppError::Internal(msg) => {
                tracing::error!(
                    error_type = %"Internal",
                    status_code = %StatusCode::INTERNAL_SERVER_ERROR,
                    message = %msg,
                    "Internal server error"
                );
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal", msg.clone())
            }
        };

        let body = Json(ErrorResponse {
            error: error_type.to_string(),
            message,
        });
        (status, body).into_response()
    }
}

impl From<&str> for AppError {
    fn from(msg: &str) -> Self {
        AppError::Internal(msg.to_string())
    }
}

impl From<String> for AppError {
    fn from(msg: String) -> Self {
        AppError::Internal(msg)
    }
}

impl From<pyo3::CastError<'_, '_>> for AppError {
    fn from(err: pyo3::CastError<'_, '_>) -> Self {
        AppError::Python(pyo3::PyErr::from(err))
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::BadRequest(format!("JSON parse error: {}", err))
    }
}

impl From<TomlError> for AppError {
    fn from(err: TomlError) -> Self {
        AppError::Internal(err.to_string())
    }
}

impl From<cargo_metadata::Error> for AppError {
    fn from(err: cargo_metadata::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}

pub type AppResult<T> = std::result::Result<T, AppError>;
pub type ApiResult<T> = std::result::Result<Json<T>, AppError>;
