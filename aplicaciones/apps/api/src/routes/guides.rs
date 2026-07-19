use crate::{db::models::Guide, error::AppResult, services::guide_service, state::AppState};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct GuideInput {
    title_ru: String,
    slug: String,
    summary_ru: String,
    target_audience: Option<String>,
    required_documents: Option<Value>,
    steps: Option<Value>,
    deadlines: Option<Value>,
    fees: Option<Value>,
    where_to_submit: Option<String>,
    common_mistakes: Option<Value>,
    risks: Option<Value>,
    official_sources: Option<Value>,
    related_article_ids: Option<Vec<Uuid>>,
    is_published: Option<bool>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list))
        .route("/:slug", get(get_by_slug))
}

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/", post(create))
        .route("/:id", put(update).delete(remove))
}

async fn list(State(state): State<AppState>) -> AppResult<Json<Vec<Guide>>> {
    Ok(Json(guide_service::list_published(&state.db).await?))
}

async fn get_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<Json<Guide>> {
    let row =
        sqlx::query_as::<_, Guide>("SELECT * FROM guides WHERE slug = $1 AND is_published = true")
            .bind(slug)
            .fetch_one(&state.db)
            .await?;
    Ok(Json(row))
}

async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<GuideInput>,
) -> AppResult<Json<Guide>> {
    crate::security::require_admin(&headers, &state)?;
    Ok(Json(save_guide(&state, None, input).await?))
}

async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<GuideInput>,
) -> AppResult<Json<Guide>> {
    crate::security::require_admin(&headers, &state)?;
    Ok(Json(save_guide(&state, Some(id), input).await?))
}

async fn save_guide(
    state: &AppState,
    id: Option<Uuid>,
    input: GuideInput,
) -> Result<Guide, sqlx::Error> {
    let empty = serde_json::json!([]);
    if let Some(id) = id {
        sqlx::query_as::<_, Guide>(
            r#"
            UPDATE guides SET
              title_ru=$1, slug=$2, summary_ru=$3, target_audience=$4,
              required_documents=$5, steps=$6, deadlines=$7, fees=$8,
              where_to_submit=$9, common_mistakes=$10, risks=$11,
              official_sources=$12, related_article_ids=$13, is_published=$14, updated_at=now()
            WHERE id=$15
            RETURNING *
            "#,
        )
        .bind(input.title_ru)
        .bind(input.slug)
        .bind(input.summary_ru)
        .bind(input.target_audience.unwrap_or_default())
        .bind(input.required_documents.unwrap_or_else(|| empty.clone()))
        .bind(input.steps.unwrap_or_else(|| empty.clone()))
        .bind(input.deadlines.unwrap_or_else(|| empty.clone()))
        .bind(input.fees.unwrap_or_else(|| empty.clone()))
        .bind(input.where_to_submit.unwrap_or_default())
        .bind(input.common_mistakes.unwrap_or_else(|| empty.clone()))
        .bind(input.risks.unwrap_or_else(|| empty.clone()))
        .bind(input.official_sources.unwrap_or_else(|| empty.clone()))
        .bind(input.related_article_ids.unwrap_or_default())
        .bind(input.is_published.unwrap_or(true))
        .bind(id)
        .fetch_one(&state.db)
        .await
    } else {
        sqlx::query_as::<_, Guide>(
            r#"
            INSERT INTO guides (
              title_ru, slug, summary_ru, target_audience, required_documents,
              steps, deadlines, fees, where_to_submit, common_mistakes, risks,
              official_sources, related_article_ids, is_published
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
            RETURNING *
            "#,
        )
        .bind(input.title_ru)
        .bind(input.slug)
        .bind(input.summary_ru)
        .bind(input.target_audience.unwrap_or_default())
        .bind(input.required_documents.unwrap_or_else(|| empty.clone()))
        .bind(input.steps.unwrap_or_else(|| empty.clone()))
        .bind(input.deadlines.unwrap_or_else(|| empty.clone()))
        .bind(input.fees.unwrap_or_else(|| empty.clone()))
        .bind(input.where_to_submit.unwrap_or_default())
        .bind(input.common_mistakes.unwrap_or_else(|| empty.clone()))
        .bind(input.risks.unwrap_or_else(|| empty.clone()))
        .bind(input.official_sources.unwrap_or_else(|| empty.clone()))
        .bind(input.related_article_ids.unwrap_or_default())
        .bind(input.is_published.unwrap_or(true))
        .fetch_one(&state.db)
        .await
    }
}

async fn remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    crate::security::require_admin(&headers, &state)?;
    sqlx::query("DELETE FROM guides WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
