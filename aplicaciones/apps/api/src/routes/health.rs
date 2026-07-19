use crate::state::AppState;
use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
struct Health {
    ok: bool,
    app: String,
    env: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/health", get(health))
}

async fn health(State(state): State<AppState>) -> Json<Health> {
    Json(Health {
        ok: true,
        app: state.config.app_name.clone(),
        env: state.config.app_env.clone(),
    })
}
