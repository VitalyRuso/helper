use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
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
        let status = match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            AppError::Upstream(_) => StatusCode::BAD_GATEWAY,
            AppError::Sqlx(sqlx::Error::RowNotFound) => StatusCode::NOT_FOUND,
            AppError::Sqlx(_) | AppError::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = Json(ErrorBody {
            error: self.to_string(),
        });
        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
