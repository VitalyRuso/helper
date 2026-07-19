use crate::{
    error::{AppError, AppResult},
    services::{access_key_service, session_service},
    state::AppState,
};
use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct UnlockRequest {
    pub session_id: String,
    pub access_key: String,
}

#[derive(Serialize)]
pub struct UnlockResponse {
    pub unlocked: bool,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/unlock", post(unlock))
}

async fn unlock(
    State(state): State<AppState>,
    Json(req): Json<UnlockRequest>,
) -> AppResult<Json<UnlockResponse>> {
    if access_key_service::is_valid(&state.db, &req.access_key).await? {
        session_service::unlock(&state.db, &req.session_id).await?;
        Ok(Json(UnlockResponse { unlocked: true }))
    } else {
        Err(AppError::Unauthorized)
    }
}
