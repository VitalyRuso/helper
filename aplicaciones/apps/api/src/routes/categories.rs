use crate::{db::models::Category, error::AppResult, services::category_service, state::AppState};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CategoryInput {
    pub title_ru: String,
    pub slug: String,
    pub description_ru: String,
    pub icon: Option<String>,
    pub sort_order: Option<i32>,
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

async fn list(State(state): State<AppState>) -> AppResult<Json<Vec<Category>>> {
    Ok(Json(category_service::list(&state.db).await?))
}

async fn get_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<Json<Category>> {
    let row = sqlx::query_as::<_, Category>("SELECT * FROM categories WHERE slug = $1")
        .bind(slug)
        .fetch_one(&state.db)
        .await?;
    Ok(Json(row))
}

async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CategoryInput>,
) -> AppResult<Json<Category>> {
    crate::security::require_admin(&headers, &state)?;
    let row = sqlx::query_as::<_, Category>(
        r#"
        INSERT INTO categories (title_ru, slug, description_ru, icon, sort_order)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(input.title_ru)
    .bind(input.slug)
    .bind(input.description_ru)
    .bind(input.icon.unwrap_or_else(|| "file-text".to_owned()))
    .bind(input.sort_order.unwrap_or(0))
    .fetch_one(&state.db)
    .await?;
    Ok(Json(row))
}

async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<CategoryInput>,
) -> AppResult<Json<Category>> {
    crate::security::require_admin(&headers, &state)?;
    let row = sqlx::query_as::<_, Category>(
        r#"
        UPDATE categories
        SET title_ru = $1, slug = $2, description_ru = $3, icon = $4, sort_order = $5, updated_at = now()
        WHERE id = $6
        RETURNING *
        "#,
    )
    .bind(input.title_ru)
    .bind(input.slug)
    .bind(input.description_ru)
    .bind(input.icon.unwrap_or_else(|| "file-text".to_owned()))
    .bind(input.sort_order.unwrap_or(0))
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    Ok(Json(row))
}

async fn remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    crate::security::require_admin(&headers, &state)?;
    sqlx::query("DELETE FROM categories WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
