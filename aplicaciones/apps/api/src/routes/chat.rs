use crate::{
    error::{AppError, AppResult},
    rag::agent,
    services::{access_key_service, session_service},
    state::AppState,
};
use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

const LIMIT_MESSAGE: &str =
    "Бесплатный лимит закончился. Введите ключ доступа или оформите подписку.";

#[derive(Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub session_id: String,
    pub page_context: Option<String>,
}

#[derive(Serialize)]
pub struct ChatResponse {
    pub answer: String,
    pub sources: Vec<agent::Citation>,
    pub remaining_guest_questions: i32,
    pub unlocked: bool,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/", post(chat))
}

async fn chat(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> AppResult<Json<ChatResponse>> {
    let message = req.message.trim();
    if message.is_empty() {
        return Err(AppError::BadRequest("message is required".to_owned()));
    }

    let session = session_service::get_or_create(&state.db, &req.session_id).await?;

    if message == "/help" {
        return Ok(Json(command_response(
            "Команды: /help, /status, /key ACCESS_KEY. Задавайте вопросы по документам после индексации базы.",
            &session,
            state.config.guest_question_limit,
        )));
    }

    if message == "/status" {
        return Ok(Json(command_response(
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
            session_service::unlock(&state.db, &req.session_id).await?;
            return Ok(Json(ChatResponse {
                answer: "Ключ принят. Доступ открыт.".to_owned(),
                sources: vec![],
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

    let answer = agent::answer(&state, message, req.page_context.as_deref()).await?;
    session_service::increment_questions(&state.db, &req.session_id).await?;

    sqlx::query("INSERT INTO chat_messages (session_id, role, content, sources) VALUES ($1, 'user', $2, '[]')")
        .bind(&req.session_id)
        .bind(message)
        .execute(&state.db)
        .await?;
    sqlx::query("INSERT INTO chat_messages (session_id, role, content, sources) VALUES ($1, 'assistant', $2, $3)")
        .bind(&req.session_id)
        .bind(&answer.answer)
        .bind(serde_json::to_value(&answer.sources).unwrap_or_else(|_| serde_json::json!([])))
        .execute(&state.db)
        .await?;

    let used = session.question_count + 1;
    Ok(Json(ChatResponse {
        answer: answer.answer,
        sources: answer.sources,
        remaining_guest_questions: (state.config.guest_question_limit - used).max(0),
        unlocked: session.has_access,
    }))
}

fn command_response(
    answer: &str,
    session: &session_service::SessionUsage,
    limit: i32,
) -> ChatResponse {
    ChatResponse {
        answer: answer.to_owned(),
        sources: vec![],
        remaining_guest_questions: (limit - session.question_count).max(0),
        unlocked: session.has_access,
    }
}
