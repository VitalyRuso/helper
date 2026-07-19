use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::{
    error::AppResult,
    legal::{fixtures, repository},
    state::AppState,
};

#[derive(Debug, Deserialize)]
struct ReviewDecisionRequest {
    reviewer: Option<String>,
    note: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sources", get(list_sources))
        .route("/review-tasks", get(list_review_tasks))
        .route("/review-tasks/:id/approve", post(approve_review_task))
        .route("/review-tasks/:id/reject", post(reject_review_task))
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

async fn approve_review_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    payload: Option<Json<ReviewDecisionRequest>>,
) -> AppResult<Json<serde_json::Value>> {
    let reviewer = payload.as_ref().and_then(|body| body.reviewer.as_deref());
    let note = payload.as_ref().and_then(|body| body.note.as_deref());

    let task = repository::approve_review_task(&state.db, task_id, reviewer, note).await?;

    Ok(Json(json!({
        "task": task
    })))
}

async fn reject_review_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    payload: Option<Json<ReviewDecisionRequest>>,
) -> AppResult<Json<serde_json::Value>> {
    let reviewer = payload.as_ref().and_then(|body| body.reviewer.as_deref());
    let note = payload.as_ref().and_then(|body| body.note.as_deref());

    let task = repository::reject_review_task(&state.db, task_id, reviewer, note).await?;

    Ok(Json(json!({
        "task": task
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
