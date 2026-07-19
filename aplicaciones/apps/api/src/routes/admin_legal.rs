use axum::{extract::State, routing::get, routing::post, Json, Router};
use serde_json::json;

use crate::{
    error::AppResult,
    legal::{fixtures, repository},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sources", get(list_sources))
        .route("/review-tasks", get(list_review_tasks))
        .route("/fixtures/run", post(run_fixture_ingestion))
}

async fn list_sources(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    let sources = repository::list_legal_sources(&state.db).await?;

    Ok(Json(json!({
        "items": sources
    })))
}

async fn list_review_tasks(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    let tasks = repository::list_pending_review_tasks(&state.db).await?;

    Ok(Json(json!({
        "items": tasks
    })))
}

async fn run_fixture_ingestion(
    State(state): State<AppState>,
) -> AppResult<Json<serde_json::Value>> {
    let result = fixtures::run_fixture_ingestion(&state.db).await?;

    Ok(Json(json!({
        "result": result
    })))
}
