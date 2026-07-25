use axum::{extract::State, routing::post, Json, Router};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    error::AppResult, local_llm::ChatResponse, services::session_service, state::AppState,
};

use super::local_llm::{CreateJobRequest, SubmitResponse};

#[derive(Serialize)]
struct CompatibleChatResponse {
    #[serde(flatten)]
    response: ChatResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    job_id: Option<Uuid>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/", post(chat))
}

async fn chat(
    State(state): State<AppState>,
    Json(request): Json<CreateJobRequest>,
) -> AppResult<Json<CompatibleChatResponse>> {
    let session_id = request.session_id.clone();
    let (_, Json(submitted)) =
        super::local_llm::submit(State(state.clone()), Json(request)).await?;

    Ok(Json(match submitted {
        SubmitResponse {
            response: Some(response),
            ..
        } => CompatibleChatResponse {
            response,
            status: None,
            job_id: None,
        },
        SubmitResponse { status, job_id, .. } => {
            let session = session_service::get_or_create(&state.db, session_id.trim()).await?;
            CompatibleChatResponse {
                response: ChatResponse {
                    answer: "Запрос поставлен в очередь и будет обработан локальной моделью."
                        .to_owned(),
                    sources: vec![],
                    legal: None,
                    remaining_guest_questions: (state.config.guest_question_limit
                        - session.question_count)
                        .max(0),
                    unlocked: session.has_access,
                },
                status: Some(status),
                job_id,
            }
        }
    }))
}
