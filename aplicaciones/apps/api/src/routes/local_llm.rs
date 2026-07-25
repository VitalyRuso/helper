use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    legal::answer::{self as legal_answer, QuestionRoute},
    local_llm::{
        self as queue, ChatResponse, ClaimRequest, ClaimResponse, CompleteJobRequest,
        FailJobRequest, HeartbeatRequest, LocalLlmStatus, PublicJob,
    },
    rag::agent::{self, PreparedAnswer},
    security,
    services::{access_key_service, session_service},
    state::AppState,
};

const LIMIT_MESSAGE: &str =
    "Бесплатный лимит закончился. Введите ключ доступа или оформите подписку.";
const MAX_MESSAGE_CHARS: usize = 10_000;
const MAX_PAGE_CONTEXT_CHARS: usize = 20_000;
const MAX_SESSION_ID_CHARS: usize = 200;

#[derive(Debug, Deserialize)]
pub struct CreateJobRequest {
    pub message: String,
    pub session_id: String,
    pub page_context: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SubmitResponse {
    pub status: String,
    pub job_id: Option<Uuid>,
    pub response: Option<ChatResponse>,
}

#[derive(Debug, Deserialize)]
struct JobQuery {
    session_id: String,
}

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/status", get(status))
        .route("/jobs", post(submit))
        .route("/jobs/:id", get(get_job))
}

pub fn worker_router() -> Router<AppState> {
    Router::new()
        .route("/check", get(check_worker))
        .route("/heartbeat", post(heartbeat))
        .route("/jobs/claim", post(claim_job))
        .route("/jobs/:id/complete", post(complete_job))
        .route("/jobs/:id/fail", post(fail_job))
}

pub(crate) async fn submit(
    State(state): State<AppState>,
    Json(request): Json<CreateJobRequest>,
) -> AppResult<(StatusCode, Json<SubmitResponse>)> {
    let message = bounded_text(&request.message, "message", MAX_MESSAGE_CHARS, true)?;
    let session_id = bounded_text(
        &request.session_id,
        "session_id",
        MAX_SESSION_ID_CHARS,
        false,
    )?;
    let page_context = request
        .page_context
        .as_deref()
        .map(|value| bounded_text(value, "page_context", MAX_PAGE_CONTEXT_CHARS, true))
        .transpose()?;

    let session = session_service::get_or_create(&state.db, &session_id).await?;
    if message == "/help" {
        return Ok(completed(command_response(
            "Команды: /help, /status, /key ACCESS_KEY. Задавайте вопросы по документам после индексации базы.",
            &session,
            state.config.guest_question_limit,
        )));
    }
    if message == "/status" {
        return Ok(completed(command_response(
            &format!(
                "Использовано вопросов: {}. Доступ: {}.",
                session.question_count,
                if session.has_access {
                    "открыт"
                } else {
                    "гостевой"
                }
            ),
            &session,
            state.config.guest_question_limit,
        )));
    }
    if let Some(key) = message.strip_prefix("/key ") {
        if access_key_service::is_valid(&state.db, key.trim()).await? {
            session_service::unlock(&state.db, &session_id).await?;
            return Ok(completed(ChatResponse {
                answer: "Ключ принят. Доступ открыт.".to_owned(),
                sources: vec![],
                legal: None,
                remaining_guest_questions: state.config.guest_question_limit,
                unlocked: true,
            }));
        }
        return Err(AppError::BadRequest(
            "Ключ доступа не найден или отключен.".to_owned(),
        ));
    }

    if !session.has_access && session.question_count >= state.config.guest_question_limit {
        return Err(AppError::TooManyRequests(LIMIT_MESSAGE.to_owned()));
    }

    let is_legal = matches!(
        legal_answer::route_question(&message),
        QuestionRoute::Legal(_)
    );
    if !is_legal {
        queue::require_available(&state.db).await?;
    }

    match agent::prepare(&state, &message, page_context.as_deref()).await? {
        PreparedAnswer::Immediate(answer) => {
            let response = queue::store_immediate_answer(
                &state.db,
                &session_id,
                &message,
                answer,
                state.config.guest_question_limit,
            )
            .await?;
            Ok(completed(response))
        }
        PreparedAnswer::Generate(plan) => {
            let job_id = queue::create_job(&state.db, &session_id, &message, plan).await?;
            Ok((
                StatusCode::ACCEPTED,
                Json(SubmitResponse {
                    status: "queued".to_owned(),
                    job_id: Some(job_id),
                    response: None,
                }),
            ))
        }
    }
}

async fn status(State(state): State<AppState>) -> AppResult<Json<LocalLlmStatus>> {
    Ok(Json(queue::get_status(&state.db).await?))
}

async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<JobQuery>,
) -> AppResult<Json<PublicJob>> {
    let session_id = bounded_text(&query.session_id, "session_id", MAX_SESSION_ID_CHARS, false)?;
    Ok(Json(
        queue::get_public_job(&state.db, id, &session_id).await?,
    ))
}

async fn check_worker(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    security::require_worker(&headers, &state)?;
    Ok(Json(json!({ "authenticated": true })))
}

async fn heartbeat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<HeartbeatRequest>,
) -> AppResult<Json<LocalLlmStatus>> {
    security::require_worker(&headers, &state)?;
    Ok(Json(queue::heartbeat(&state.db, request).await?))
}

async fn claim_job(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ClaimRequest>,
) -> AppResult<Json<ClaimResponse>> {
    security::require_worker(&headers, &state)?;
    Ok(Json(queue::claim_job(&state.db, request).await?))
}

async fn complete_job(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(request): Json<CompleteJobRequest>,
) -> AppResult<Json<serde_json::Value>> {
    security::require_worker(&headers, &state)?;
    queue::complete_job(&state.db, id, request, state.config.guest_question_limit).await?;
    Ok(Json(json!({ "status": "completed" })))
}

async fn fail_job(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(request): Json<FailJobRequest>,
) -> AppResult<Json<serde_json::Value>> {
    security::require_worker(&headers, &state)?;
    queue::fail_job(&state.db, id, request).await?;
    Ok(Json(json!({ "status": "failed" })))
}

fn completed(response: ChatResponse) -> (StatusCode, Json<SubmitResponse>) {
    (
        StatusCode::OK,
        Json(SubmitResponse {
            status: "completed".to_owned(),
            job_id: None,
            response: Some(response),
        }),
    )
}

fn command_response(
    answer: &str,
    session: &session_service::SessionUsage,
    limit: i32,
) -> ChatResponse {
    ChatResponse {
        answer: answer.to_owned(),
        sources: vec![],
        legal: None,
        remaining_guest_questions: (limit - session.question_count).max(0),
        unlocked: session.has_access,
    }
}

fn bounded_text(
    value: &str,
    field: &str,
    max_chars: usize,
    allow_layout_whitespace: bool,
) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::BadRequest(format!("{field} is required")));
    }
    if value.chars().count() > max_chars {
        return Err(AppError::BadRequest(format!("{field} is too long")));
    }
    if value.chars().any(|character| {
        character.is_control()
            && !(allow_layout_whitespace && matches!(character, '\n' | '\r' | '\t'))
    }) {
        return Err(AppError::BadRequest(format!(
            "{field} contains invalid characters"
        )));
    }
    Ok(value.to_owned())
}
