use crate::{
    error::{AppError, AppResult},
    state::AppState,
};
use serde_json::{json, Value};

pub async fn complete(state: &AppState, prompt: &str) -> AppResult<String> {
    let url = format!("{}/api/generate", state.config.ollama_base_url);
    let response = state
        .http
        .post(url)
        .json(&json!({
            "model": state.config.ollama_model,
            "prompt": prompt,
            "stream": false
        }))
        .send()
        .await
        .map_err(|_| {
            AppError::Upstream(
                "ИИ-провайдер Ollama недоступен. Проверьте OLLAMA_BASE_URL и запустите Ollama."
                    .to_owned(),
            )
        })?;
    if !response.status().is_success() {
        return Err(AppError::Upstream(
            "ИИ-провайдер Ollama вернул ошибку. Проверьте модель и настройки Ollama.".to_owned(),
        ));
    }
    let body: Value = response
        .json()
        .await
        .map_err(|e| AppError::Upstream(e.to_string()))?;
    Ok(body["response"].as_str().unwrap_or_default().to_owned())
}
