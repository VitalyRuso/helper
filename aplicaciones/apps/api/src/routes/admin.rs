use crate::{
    error::{AppError, AppResult},
    security,
    services::admin_service,
    state::AppState,
};
use axum::{
    extract::State,
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    token: String,
}

#[derive(Serialize)]
pub struct AdminStats {
    pub content: admin_service::Stats,
    pub rag_vectors: u64,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/stats", get(stats))
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    if req.username == state.config.admin_username && req.password == state.config.admin_password {
        Ok(Json(LoginResponse {
            token: state.admin_token.to_string(),
        }))
    } else {
        Err(AppError::Unauthorized)
    }
}

async fn stats(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<AdminStats>> {
    security::require_admin(&headers, &state)?;
    let content = admin_service::stats(&state.db).await?;
    let rag_vectors = crate::rag::qdrant_store::count_points(&state)
        .await
        .unwrap_or(0);
    Ok(Json(AdminStats {
        content,
        rag_vectors,
    }))
}
