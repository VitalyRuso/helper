use crate::{db::models::Article, error::AppResult, services::article_service, state::AppState};
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ArticleQuery {
    category: Option<String>,
}

#[derive(Deserialize)]
pub struct ArticleInput {
    category_id: Option<Uuid>,
    title_ru: String,
    slug: String,
    summary_ru: String,
    body_ru_markdown: String,
    tags: Option<Vec<String>>,
    source_references: Option<Value>,
    legal_risk_level: Option<String>,
    is_published: Option<bool>,
    include_in_ai: Option<bool>,
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

async fn list(
    State(state): State<AppState>,
    Query(query): Query<ArticleQuery>,
) -> AppResult<Json<Vec<Article>>> {
    if let Some(category) = query.category {
        let rows = sqlx::query_as::<_, Article>(
            r#"
            SELECT a.* FROM articles a
            JOIN categories c ON c.id = a.category_id
            WHERE a.is_published = true AND c.slug = $1
            ORDER BY a.updated_at DESC
            "#,
        )
        .bind(category)
        .fetch_all(&state.db)
        .await?;
        Ok(Json(rows))
    } else {
        Ok(Json(article_service::list_published(&state.db).await?))
    }
}

async fn get_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<Json<Article>> {
    let row = sqlx::query_as::<_, Article>(
        "SELECT * FROM articles WHERE slug = $1 AND is_published = true",
    )
    .bind(slug)
    .fetch_one(&state.db)
    .await?;
    Ok(Json(row))
}

async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ArticleInput>,
) -> AppResult<Json<Article>> {
    crate::security::require_admin(&headers, &state)?;
    let row = save_article(&state, None, input).await?;
    Ok(Json(row))
}

async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<ArticleInput>,
) -> AppResult<Json<Article>> {
    crate::security::require_admin(&headers, &state)?;
    let row = save_article(&state, Some(id), input).await?;
    Ok(Json(row))
}

async fn save_article(
    state: &AppState,
    id: Option<Uuid>,
    input: ArticleInput,
) -> Result<Article, sqlx::Error> {
    if let Some(id) = id {
        sqlx::query_as::<_, Article>(
            r#"
            UPDATE articles SET
              category_id = $1, title_ru = $2, slug = $3, summary_ru = $4,
              body_ru_markdown = $5, tags = $6, source_references = $7,
              legal_risk_level = $8, is_published = $9, include_in_ai = $10, updated_at = now()
            WHERE id = $11
            RETURNING *
            "#,
        )
        .bind(input.category_id)
        .bind(input.title_ru)
        .bind(input.slug)
        .bind(input.summary_ru)
        .bind(input.body_ru_markdown)
        .bind(input.tags.unwrap_or_default())
        .bind(
            input
                .source_references
                .unwrap_or_else(|| serde_json::json!([])),
        )
        .bind(
            input
                .legal_risk_level
                .unwrap_or_else(|| "medium".to_owned()),
        )
        .bind(input.is_published.unwrap_or(true))
        .bind(input.include_in_ai.unwrap_or(true))
        .bind(id)
        .fetch_one(&state.db)
        .await
    } else {
        sqlx::query_as::<_, Article>(
            r#"
            INSERT INTO articles (
              category_id, title_ru, slug, summary_ru, body_ru_markdown, tags,
              source_references, legal_risk_level, is_published, include_in_ai
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            RETURNING *
            "#,
        )
        .bind(input.category_id)
        .bind(input.title_ru)
        .bind(input.slug)
        .bind(input.summary_ru)
        .bind(input.body_ru_markdown)
        .bind(input.tags.unwrap_or_default())
        .bind(
            input
                .source_references
                .unwrap_or_else(|| serde_json::json!([])),
        )
        .bind(
            input
                .legal_risk_level
                .unwrap_or_else(|| "medium".to_owned()),
        )
        .bind(input.is_published.unwrap_or(true))
        .bind(input.include_in_ai.unwrap_or(true))
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
    sqlx::query("DELETE FROM articles WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
