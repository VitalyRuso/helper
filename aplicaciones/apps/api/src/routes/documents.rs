use crate::state::AppState;
use axum::{http::StatusCode, routing::post, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
struct NotReady {
    error: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/analyze", post(analyze))
}

async fn analyze() -> (StatusCode, Json<NotReady>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(NotReady {
            error: "Модуль анализа документов готовится. Сейчас используйте ИИ-помощника и базу инструкций.",
        }),
    )
}
