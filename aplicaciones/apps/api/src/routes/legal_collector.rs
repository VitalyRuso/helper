use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::HeaderMap,
    routing::post,
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
