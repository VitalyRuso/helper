use axum::{
    body::Bytes,
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    legal::{fixtures, repository},
    state::AppState,
};

#[derive(Debug, Default, Deserialize)]
struct ReviewDecisionRequest {
    reviewer: Option<String>,
    #[serde(alias = "note")]
    reviewer_note: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sources", get(list_sources))
        .route("/review-tasks", get(list_review_tasks))
        .route("/review-tasks/:id", get(get_review_task))
        .route("/review-tasks/:id/approve", post(approve_review_task))
        .route("/review-tasks/:id/reject", post(reject_review_task))
        .route("/changes/:id", get(get_legal_change))
        .route("/documents/:id/versions", get(list_document_versions))
        .route("/documents/:id/sections", get(list_document_sections))
        .route("/knowledge", get(list_knowledge_items))
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

async fn get_review_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let context = repository::get_review_task_context(&state.db, task_id).await?;
    Ok(Json(json!(context)))
}

async fn get_legal_change(
    State(state): State<AppState>,
    Path(change_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let context = repository::get_legal_change_context(&state.db, change_id).await?;
    Ok(Json(json!(context)))
}

async fn list_document_versions(
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let document = repository::get_legal_document(&state.db, document_id).await?;
    let versions = repository::list_document_versions(&state.db, document_id).await?;
    let current_version_ids = versions
        .iter()
        .filter(|version| version.is_current)
        .map(|version| version.id)
        .collect::<Vec<_>>();
    let current_version_id = (current_version_ids.len() == 1).then(|| current_version_ids[0]);

    Ok(Json(json!({
        "document": document,
        "items": versions,
        "current_version_id": current_version_id,
        "current_version_count": current_version_ids.len()
    })))
}

async fn list_document_sections(
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let document = repository::get_legal_document(&state.db, document_id).await?;
    let versions = repository::list_document_versions(&state.db, document_id).await?;
    let current_versions = versions
        .into_iter()
        .filter(|version| version.is_current)
        .collect::<Vec<_>>();
    let current_version = (current_versions.len() == 1).then(|| current_versions[0].clone());
    let sections = repository::list_current_document_sections(&state.db, document_id).await?;

    Ok(Json(json!({
        "document": document,
        "version": current_version,
        "current_version_count": current_versions.len(),
        "items": sections
    })))
}

async fn list_knowledge_items(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    let items = repository::list_knowledge_items(&state.db).await?;
    Ok(Json(json!({ "items": items })))
}

async fn approve_review_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    body: Bytes,
) -> AppResult<Json<serde_json::Value>> {
    let payload = parse_review_decision(&body)?;
    let reviewer = payload
        .reviewer
        .as_deref()
        .filter(|reviewer| !reviewer.trim().is_empty())
        .unwrap_or("Legal Reviewer");
    let reviewer_note = payload.reviewer_note.as_deref().unwrap_or("");

    let result =
        repository::approve_review_task(&state.db, task_id, Some(reviewer), Some(reviewer_note))
            .await?;

    Ok(Json(json!(result)))
}

async fn reject_review_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    body: Bytes,
) -> AppResult<Json<serde_json::Value>> {
    let payload = parse_review_decision(&body)?;
    let reviewer = payload
        .reviewer
        .as_deref()
        .filter(|reviewer| !reviewer.trim().is_empty())
        .unwrap_or("Legal Reviewer");
    let reviewer_note = payload.reviewer_note.as_deref().unwrap_or("");

    let result =
        repository::reject_review_task(&state.db, task_id, Some(reviewer), Some(reviewer_note))
            .await?;

    Ok(Json(json!(result)))
}

fn parse_review_decision(body: &[u8]) -> AppResult<ReviewDecisionRequest> {
    if body.iter().all(|byte| byte.is_ascii_whitespace()) {
        return Ok(ReviewDecisionRequest::default());
    }

    serde_json::from_slice(body)
        .map_err(|_| AppError::BadRequest("request body must be valid JSON".to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_body_is_optional_but_must_be_json() {
        assert!(parse_review_decision(b" \n").unwrap().reviewer.is_none());

        let decision = parse_review_decision(br#"{"reviewer":"Ana","note":"checked"}"#).unwrap();
        assert_eq!(decision.reviewer.as_deref(), Some("Ana"));
        assert_eq!(decision.reviewer_note.as_deref(), Some("checked"));
        assert!(matches!(
            parse_review_decision(b"not json"),
            Err(AppError::BadRequest(_))
        ));
    }
}

async fn run_fixture_ingestion(
    State(state): State<AppState>,
) -> AppResult<Json<serde_json::Value>> {
    let result = fixtures::run_fixture_ingestion(&state.db).await?;

    Ok(Json(json!({
        "result": result
    })))
}
