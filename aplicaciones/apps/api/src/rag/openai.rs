use crate::{
    error::{AppError, AppResult},
    state::AppState,
};
use serde_json::{json, Value};

pub async fn complete(state: &AppState, prompt: &str) -> AppResult<String> {
    let Some(key) = &state.config.openai_api_key else {
        return Err(AppError::BadRequest(
            "LLM_PROVIDER=openai requires OPENAI_API_KEY".to_owned(),
        ));
    };
    let response = state
        .http
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(key)
        .json(&json!({
            "model": state.config.openai_model,
            "messages": [{ "role": "user", "content": prompt }]
        }))
        .send()
        .await
        .map_err(|e| AppError::Upstream(e.to_string()))?;
    if !response.status().is_success() {
        return Err(AppError::Upstream(
            response.text().await.unwrap_or_default(),
        ));
    }
    let body: Value = response
        .json()
        .await
        .map_err(|e| AppError::Upstream(e.to_string()))?;
    Ok(body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or_default()
        .to_owned())
}
