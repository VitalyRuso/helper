use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("Доступ запрещен")]
    Unauthorized,
    #[error("{0}")]
    TooManyRequests(String),
    #[error("{0}")]
    Upstream(String),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, error) = match &self {
            AppError::BadRequest(message) => (StatusCode::BAD_REQUEST, message.clone()),
            AppError::NotFound(message) => (StatusCode::NOT_FOUND, message.clone()),
            AppError::Conflict(message) => (StatusCode::CONFLICT, message.clone()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::TooManyRequests(message) => (StatusCode::TOO_MANY_REQUESTS, message.clone()),
            AppError::Upstream(message) => (StatusCode::BAD_GATEWAY, message.clone()),
            AppError::Sqlx(sqlx::Error::RowNotFound) => {
                (StatusCode::NOT_FOUND, "resource not found".to_owned())
            }
            AppError::Sqlx(error) => {
                tracing::error!(?error, "database request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_owned(),
                )
            }
            AppError::Anyhow(error) => {
                tracing::error!(?error, "request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_owned(),
                )
            }
        };
        let body = Json(ErrorBody { error });
        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
