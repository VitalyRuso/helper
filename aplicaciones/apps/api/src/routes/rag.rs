use crate::{
    error::AppResult,
    rag::{indexer, qdrant_store},
    state::AppState,
};
use axum::{
    extract::State,
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct RagStatus {
    pub collection: String,
    pub vectors: u64,
    pub embedding_provider: String,
    pub embedding_model: String,
    pub embedding_dimensions: usize,
    pub embedding_fallback: String,
    pub qdrant_available: bool,
    pub error: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/status", get(status))
        .route("/reindex", post(reindex))
}

async fn status(State(state): State<AppState>) -> AppResult<Json<RagStatus>> {
    let (vectors, qdrant_available, error) = match qdrant_store::count_points(&state).await {
        Ok(vectors) => (vectors, true, None),
        Err(err) => (0, false, Some(err.to_string())),
    };
    Ok(Json(RagStatus {
        collection: state.config.qdrant_collection.clone(),
        vectors,
        embedding_provider: state.embeddings.provider_name().to_owned(),
        embedding_model: state.embeddings.model_name().to_owned(),
        embedding_dimensions: state.embeddings.dimensions(),
        embedding_fallback: state.embeddings.fallback().to_owned(),
        qdrant_available,
        error,
    }))
}

async fn reindex(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<indexer::IndexReport>> {
    crate::security::require_admin(&headers, &state)?;
    Ok(Json(indexer::reindex(&state).await?))
}
