use crate::{error::AppResult, services::search_service, state::AppState};
use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(search))
}

async fn search(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> AppResult<Json<Vec<search_service::SearchResult>>> {
    let q = query.q.unwrap_or_default();
    if q.trim().is_empty() {
        return Ok(Json(vec![]));
    }
    Ok(Json(search_service::search(&state.db, q.trim()).await?))
}
