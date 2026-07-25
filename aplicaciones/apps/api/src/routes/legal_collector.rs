use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};

use crate::{
    error::{AppError, AppResult},
    legal::collector::{self, CollectorIntakeRequest},
    security,
    state::AppState,
};

const MAX_INTAKE_BODY_BYTES: usize = 1_200_000;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/intake", post(intake))
        .route("/report", get(report))
        .layer(DefaultBodyLimit::max(MAX_INTAKE_BODY_BYTES))
}

async fn intake(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<Json<collector::CollectorIntakeResponse>> {
    security::require_legal_collector(&headers, &state)?;
    let request: CollectorIntakeRequest = serde_json::from_slice(&body)
        .map_err(|_| AppError::BadRequest("request body must be valid JSON".to_owned()))?;
    Ok(Json(collector::ingest(&state.db, request).await?))
}

async fn report(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<collector::CollectorSqlReport>> {
    security::require_legal_collector(&headers, &state)?;
    Ok(Json(collector::sql_report(&state.db).await?))
}
