use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

pub async fn complete(state: &AppState, prompt: &str) -> AppResult<String> {
    match state.config.llm_provider.as_str() {
        "ollama" => crate::rag::ollama::complete(state, prompt).await,
        "openai" => crate::rag::openai::complete(state, prompt).await,
        other => Err(AppError::BadRequest(format!(
            "unknown LLM_PROVIDER: {other}"
        ))),
    }
}
