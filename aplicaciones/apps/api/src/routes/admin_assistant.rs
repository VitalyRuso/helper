use crate::{
    error::{AppError, AppResult},
    security,
    state::AppState,
    tool_adapters::assistant_brain_adapter,
};
use assistant_brain_tool::{
    approve_candidate as approve_tool_candidate, create_change_candidate as tool_candidate,
    AssistantChangeCandidate, AssistantChangeType, ReviewStatus, RiskLevel,
};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/profiles", get(list_profiles).post(create_profile))
        .route("/profiles/:id", get(get_profile))
        .route(
            "/profiles/:id/prompts",
            get(list_prompts).post(create_prompt),
        )
        .route("/prompts/:id/activate", post(reject_prompt_activation))
        .route(
            "/profiles/:id/policies",
            get(list_policies).post(create_policy),
        )
        .route("/policies/:id/activate", post(reject_policy_activation))
        .route("/candidates", get(list_candidates).post(create_candidate))
        .route("/candidates/:id", get(get_candidate))
        .route("/candidates/:id/approve", post(approve_candidate))
        .route("/candidates/:id/reject", post(reject_candidate))
        .route("/notes", get(list_notes).post(create_note))
        .route("/notes/:id/convert-to-candidate", post(convert_note))
}

#[derive(Debug, Serialize, FromRow)]
pub struct AssistantProfileRow {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub active_prompt_version_id: Option<Uuid>,
    pub active_policy_version_id: Option<Uuid>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct AssistantPromptRow {
    pub id: Uuid,
    pub assistant_profile_id: Uuid,
    pub title: String,
    pub system_prompt: String,
    pub answer_format: String,
    pub safety_rules: Vec<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct AssistantPolicyRow {
    pub id: Uuid,
    pub assistant_profile_id: Uuid,
    pub title: String,
    pub retrieval_top_k: i32,
    pub require_sources: bool,
    pub allow_llm_without_sources: bool,
    pub allowed_collection: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct AssistantCandidateRow {
    pub id: Uuid,
    pub assistant_profile_id: Uuid,
    pub candidate_type: String,
    pub title: String,
    pub description: String,
    pub proposed_payload_json: Value,
    pub reason: String,
    pub risk_level: String,
    pub status: String,
    pub review_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct AssistantNoteRow {
    pub id: Uuid,
    pub note_type: String,
    pub title: String,
    pub body: String,
    pub status: String,
    pub candidate_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct ProfileInput {
    name: String,
    slug: String,
    description: Option<String>,
    is_active: Option<bool>,
}

#[derive(Deserialize)]
struct PromptInput {
    title: String,
    system_prompt: String,
    answer_format: Option<String>,
    safety_rules: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct PolicyInput {
    title: String,
    retrieval_top_k: Option<i32>,
    require_sources: Option<bool>,
    allow_llm_without_sources: Option<bool>,
    allowed_collection: Option<String>,
}

#[derive(Deserialize)]
struct CandidateInput {
    assistant_profile_id: Uuid,
    candidate_type: String,
    title: String,
    description: Option<String>,
    proposed_payload_json: Option<Value>,
    reason: Option<String>,
    risk_level: Option<String>,
}

#[derive(Deserialize)]
struct ReviewInput {
    note: Option<String>,
}

#[derive(Deserialize)]
struct NoteInput {
    note_type: String,
    title: String,
    body: Option<String>,
}

async fn list_profiles(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<AssistantProfileRow>>> {
    security::require_admin(&headers, &state)?;
    Ok(Json(
        sqlx::query_as::<_, AssistantProfileRow>(
            "SELECT * FROM assistant_profiles ORDER BY created_at ASC",
        )
        .fetch_all(&state.db)
        .await?,
    ))
}

async fn create_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ProfileInput>,
) -> AppResult<Json<AssistantProfileRow>> {
    security::require_admin(&headers, &state)?;
    Ok(Json(
        sqlx::query_as::<_, AssistantProfileRow>(
            r#"
            INSERT INTO assistant_profiles (name, slug, description, is_active)
            VALUES ($1,$2,$3,$4)
            RETURNING *
            "#,
        )
        .bind(input.name)
        .bind(input.slug)
        .bind(input.description.unwrap_or_default())
        .bind(input.is_active.unwrap_or(true))
        .fetch_one(&state.db)
        .await?,
    ))
}

async fn get_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> AppResult<Json<AssistantProfileRow>> {
    security::require_admin(&headers, &state)?;
    Ok(Json(profile_by_id(&state, id).await?))
}

async fn list_prompts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<AssistantPromptRow>>> {
    security::require_admin(&headers, &state)?;
    Ok(Json(
        sqlx::query_as::<_, AssistantPromptRow>(
            "SELECT * FROM assistant_prompt_versions WHERE assistant_profile_id = $1 ORDER BY created_at DESC",
        )
        .bind(id)
        .fetch_all(&state.db)
        .await?,
    ))
}

async fn create_prompt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<PromptInput>,
) -> AppResult<Json<AssistantPromptRow>> {
    security::require_admin(&headers, &state)?;
    if input.system_prompt.trim().is_empty() {
        return Err(AppError::BadRequest("system_prompt is required".to_owned()));
    }
    Ok(Json(
        sqlx::query_as::<_, AssistantPromptRow>(
            r#"
            INSERT INTO assistant_prompt_versions (assistant_profile_id, title, system_prompt, answer_format, safety_rules, status)
            VALUES ($1,$2,$3,$4,$5,'draft')
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(input.title)
        .bind(input.system_prompt)
        .bind(input.answer_format.unwrap_or_default())
        .bind(input.safety_rules.unwrap_or_default())
        .fetch_one(&state.db)
        .await?,
    ))
}

async fn reject_prompt_activation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    security::require_admin(&headers, &state)?;
    assistant_brain_adapter::reject_direct_prompt_or_policy_activation()
        .map_err(|error| AppError::BadRequest(error.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": false })))
}

async fn list_policies(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<AssistantPolicyRow>>> {
    security::require_admin(&headers, &state)?;
    Ok(Json(
        sqlx::query_as::<_, AssistantPolicyRow>(
            "SELECT * FROM assistant_policies WHERE assistant_profile_id = $1 ORDER BY created_at DESC",
        )
        .bind(id)
        .fetch_all(&state.db)
        .await?,
    ))
}

async fn create_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<PolicyInput>,
) -> AppResult<Json<AssistantPolicyRow>> {
    security::require_admin(&headers, &state)?;
    Ok(Json(
        sqlx::query_as::<_, AssistantPolicyRow>(
            r#"
            INSERT INTO assistant_policies (
              assistant_profile_id, title, retrieval_top_k, require_sources,
              allow_llm_without_sources, allowed_collection, status
            )
            VALUES ($1,$2,$3,$4,$5,$6,'draft')
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(input.title)
        .bind(
            input
                .retrieval_top_k
                .unwrap_or(state.config.top_k as i32)
                .max(1),
        )
        .bind(input.require_sources.unwrap_or(true))
        .bind(input.allow_llm_without_sources.unwrap_or(false))
        .bind(
            input
                .allowed_collection
                .unwrap_or_else(|| state.config.qdrant_collection.to_string()),
        )
        .fetch_one(&state.db)
        .await?,
    ))
}

async fn reject_policy_activation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    security::require_admin(&headers, &state)?;
    assistant_brain_adapter::reject_direct_prompt_or_policy_activation()
        .map_err(|error| AppError::BadRequest(error.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": false })))
}

async fn list_candidates(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<AssistantCandidateRow>>> {
    security::require_admin(&headers, &state)?;
    Ok(Json(
        sqlx::query_as::<_, AssistantCandidateRow>(
            "SELECT * FROM assistant_change_candidates ORDER BY updated_at DESC",
        )
        .fetch_all(&state.db)
        .await?,
    ))
}

async fn create_candidate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CandidateInput>,
) -> AppResult<Json<AssistantCandidateRow>> {
    security::require_admin(&headers, &state)?;
    let candidate = tool_candidate(
        &input.assistant_profile_id.to_string(),
        parse_candidate_type(&input.candidate_type),
        &input.title,
        input.description.as_deref().unwrap_or_default(),
        input
            .proposed_payload_json
            .clone()
            .unwrap_or_else(|| serde_json::json!({})),
        input.reason.as_deref().unwrap_or_default(),
        parse_risk(input.risk_level.as_deref().unwrap_or("medium")),
    );
    Ok(Json(
        insert_candidate(&state, input.assistant_profile_id, candidate).await?,
    ))
}

async fn get_candidate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> AppResult<Json<AssistantCandidateRow>> {
    security::require_admin(&headers, &state)?;
    Ok(Json(candidate_by_id(&state, id).await?))
}

async fn approve_candidate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<ReviewInput>,
) -> AppResult<Json<AssistantCandidateRow>> {
    security::require_admin(&headers, &state)?;
    let row = candidate_by_id(&state, id).await?;
    if row.status != "draft" {
        return Err(AppError::BadRequest(
            "candidate is already reviewed".to_owned(),
        ));
    }
    let _ = approve_tool_candidate(row_to_tool_candidate(&row)?)
        .map_err(|error| AppError::BadRequest(error.to_string()))?;
    match row.candidate_type.as_str() {
        "prompt_change" => approve_prompt_change(&state, &row).await?,
        "policy_change" => approve_policy_change(&state, &row).await?,
        _ => {}
    }
    Ok(Json(
        sqlx::query_as::<_, AssistantCandidateRow>(
            "UPDATE assistant_change_candidates SET status = 'approved', review_note = $1, updated_at = now() WHERE id = $2 RETURNING *",
        )
        .bind(input.note.unwrap_or_default())
        .bind(id)
        .fetch_one(&state.db)
        .await?,
    ))
}

async fn reject_candidate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<ReviewInput>,
) -> AppResult<Json<AssistantCandidateRow>> {
    security::require_admin(&headers, &state)?;
    Ok(Json(
        sqlx::query_as::<_, AssistantCandidateRow>(
            "UPDATE assistant_change_candidates SET status = 'rejected', review_note = $1, updated_at = now() WHERE id = $2 RETURNING *",
        )
        .bind(input.note.unwrap_or_default())
        .bind(id)
        .fetch_one(&state.db)
        .await?,
    ))
}

async fn list_notes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<AssistantNoteRow>>> {
    security::require_admin(&headers, &state)?;
    Ok(Json(
        sqlx::query_as::<_, AssistantNoteRow>(
            "SELECT * FROM assistant_notes ORDER BY updated_at DESC",
        )
        .fetch_all(&state.db)
        .await?,
    ))
}

async fn create_note(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<NoteInput>,
) -> AppResult<Json<AssistantNoteRow>> {
    security::require_admin(&headers, &state)?;
    Ok(Json(
        sqlx::query_as::<_, AssistantNoteRow>(
            "INSERT INTO assistant_notes (note_type, title, body) VALUES ($1,$2,$3) RETURNING *",
        )
        .bind(input.note_type)
        .bind(input.title)
        .bind(input.body.unwrap_or_default())
        .fetch_one(&state.db)
        .await?,
    ))
}

async fn convert_note(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> AppResult<Json<AssistantCandidateRow>> {
    security::require_admin(&headers, &state)?;
    let note = sqlx::query_as::<_, AssistantNoteRow>("SELECT * FROM assistant_notes WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await?;
    let profile_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM assistant_profiles WHERE is_active = true ORDER BY created_at ASC LIMIT 1",
    )
    .fetch_one(&state.db)
    .await?;
    let candidate = insert_candidate(
        &state,
        profile_id,
        tool_candidate(
            &profile_id.to_string(),
            AssistantChangeType::ArchitectureNote,
            &note.title,
            &note.body,
            serde_json::json!({ "note_id": note.id, "note_type": note.note_type, "body": note.body }),
            "Converted from assistant note.",
            RiskLevel::Low,
        ),
    )
    .await?;
    sqlx::query("UPDATE assistant_notes SET status = 'converted', candidate_id = $1, updated_at = now() WHERE id = $2")
        .bind(candidate.id)
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(candidate))
}

async fn approve_prompt_change(state: &AppState, row: &AssistantCandidateRow) -> AppResult<()> {
    let title =
        text_payload(&row.proposed_payload_json, "title").unwrap_or_else(|| row.title.clone());
    let system_prompt =
        text_payload(&row.proposed_payload_json, "system_prompt").ok_or_else(|| {
            AppError::BadRequest("proposed_payload_json.system_prompt is required".to_owned())
        })?;
    let answer_format =
        text_payload(&row.proposed_payload_json, "answer_format").unwrap_or_default();
    let safety_rules = row
        .proposed_payload_json
        .get("safety_rules")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_else(Vec::<String>::new);
    sqlx::query("UPDATE assistant_prompt_versions SET status = 'archived', updated_at = now() WHERE assistant_profile_id = $1 AND status = 'active'")
        .bind(row.assistant_profile_id)
        .execute(&state.db)
        .await?;
    let prompt = sqlx::query_as::<_, AssistantPromptRow>(
        r#"
        INSERT INTO assistant_prompt_versions (
          assistant_profile_id, title, system_prompt, answer_format, safety_rules, status
        )
        VALUES ($1,$2,$3,$4,$5,'active')
        RETURNING *
        "#,
    )
    .bind(row.assistant_profile_id)
    .bind(title)
    .bind(system_prompt)
    .bind(answer_format)
    .bind(safety_rules)
    .fetch_one(&state.db)
    .await?;
    sqlx::query("UPDATE assistant_profiles SET active_prompt_version_id = $1, updated_at = now() WHERE id = $2")
        .bind(prompt.id)
        .bind(row.assistant_profile_id)
        .execute(&state.db)
        .await?;
    Ok(())
}

async fn approve_policy_change(state: &AppState, row: &AssistantCandidateRow) -> AppResult<()> {
    sqlx::query("UPDATE assistant_policies SET status = 'archived', updated_at = now() WHERE assistant_profile_id = $1 AND status = 'active'")
        .bind(row.assistant_profile_id)
        .execute(&state.db)
        .await?;
    let policy = sqlx::query_as::<_, AssistantPolicyRow>(
        r#"
        INSERT INTO assistant_policies (
          assistant_profile_id, title, retrieval_top_k, require_sources,
          allow_llm_without_sources, allowed_collection, status
        )
        VALUES ($1,$2,$3,$4,$5,$6,'active')
        RETURNING *
        "#,
    )
    .bind(row.assistant_profile_id)
    .bind(text_payload(&row.proposed_payload_json, "title").unwrap_or_else(|| row.title.clone()))
    .bind(
        int_payload(&row.proposed_payload_json, "retrieval_top_k")
            .unwrap_or(state.config.top_k as i32)
            .max(1),
    )
    .bind(bool_payload(&row.proposed_payload_json, "require_sources").unwrap_or(true))
    .bind(bool_payload(&row.proposed_payload_json, "allow_llm_without_sources").unwrap_or(false))
    .bind(
        text_payload(&row.proposed_payload_json, "allowed_collection")
            .unwrap_or_else(|| state.config.qdrant_collection.to_string()),
    )
    .fetch_one(&state.db)
    .await?;
    sqlx::query("UPDATE assistant_profiles SET active_policy_version_id = $1, updated_at = now() WHERE id = $2")
        .bind(policy.id)
        .bind(row.assistant_profile_id)
        .execute(&state.db)
        .await?;
    Ok(())
}

async fn profile_by_id(state: &AppState, id: Uuid) -> Result<AssistantProfileRow, sqlx::Error> {
    sqlx::query_as::<_, AssistantProfileRow>("SELECT * FROM assistant_profiles WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await
}

async fn candidate_by_id(state: &AppState, id: Uuid) -> Result<AssistantCandidateRow, sqlx::Error> {
    sqlx::query_as::<_, AssistantCandidateRow>(
        "SELECT * FROM assistant_change_candidates WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
}

async fn insert_candidate(
    state: &AppState,
    assistant_profile_id: Uuid,
    candidate: AssistantChangeCandidate,
) -> Result<AssistantCandidateRow, sqlx::Error> {
    sqlx::query_as::<_, AssistantCandidateRow>(
        r#"
        INSERT INTO assistant_change_candidates (
          assistant_profile_id, candidate_type, title, description,
          proposed_payload_json, reason, risk_level, status
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
        RETURNING *
        "#,
    )
    .bind(assistant_profile_id)
    .bind(candidate_type(&candidate.candidate_type))
    .bind(candidate.title)
    .bind(candidate.description)
    .bind(candidate.proposed_payload_json)
    .bind(candidate.reason)
    .bind(risk_level(&candidate.risk_level))
    .bind(review_status(&candidate.status))
    .fetch_one(&state.db)
    .await
}

fn row_to_tool_candidate(row: &AssistantCandidateRow) -> AppResult<AssistantChangeCandidate> {
    Ok(AssistantChangeCandidate {
        id: row.id.to_string(),
        assistant_profile_id: row.assistant_profile_id.to_string(),
        candidate_type: parse_candidate_type(&row.candidate_type),
        title: row.title.clone(),
        description: row.description.clone(),
        proposed_payload_json: row.proposed_payload_json.clone(),
        reason: row.reason.clone(),
        risk_level: parse_risk(&row.risk_level),
        status: match row.status.as_str() {
            "draft" => ReviewStatus::Draft,
            "approved" => ReviewStatus::Approved,
            "rejected" => ReviewStatus::Rejected,
            other => {
                return Err(AppError::BadRequest(format!(
                    "unknown candidate status: {other}"
                )))
            }
        },
    })
}

fn parse_candidate_type(value: &str) -> AssistantChangeType {
    match value {
        "prompt_change" => AssistantChangeType::PromptChange,
        "policy_change" => AssistantChangeType::PolicyChange,
        "tool_change" => AssistantChangeType::ToolChange,
        "answer_format_change" => AssistantChangeType::AnswerFormatChange,
        "data_source_change" => AssistantChangeType::DataSourceChange,
        _ => AssistantChangeType::ArchitectureNote,
    }
}

fn candidate_type(value: &AssistantChangeType) -> &'static str {
    match value {
        AssistantChangeType::PromptChange => "prompt_change",
        AssistantChangeType::PolicyChange => "policy_change",
        AssistantChangeType::ToolChange => "tool_change",
        AssistantChangeType::AnswerFormatChange => "answer_format_change",
        AssistantChangeType::DataSourceChange => "data_source_change",
        AssistantChangeType::ArchitectureNote => "architecture_note",
    }
}

fn parse_risk(value: &str) -> RiskLevel {
    match value {
        "low" => RiskLevel::Low,
        "high" => RiskLevel::High,
        _ => RiskLevel::Medium,
    }
}

fn risk_level(value: &RiskLevel) -> &'static str {
    match value {
        RiskLevel::Low => "low",
        RiskLevel::Medium => "medium",
        RiskLevel::High => "high",
    }
}

fn review_status(value: &ReviewStatus) -> &'static str {
    match value {
        ReviewStatus::Draft => "draft",
        ReviewStatus::Approved => "approved",
        ReviewStatus::Rejected => "rejected",
    }
}

fn text_payload(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn bool_payload(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn int_payload(value: &Value, key: &str) -> Option<i32> {
    value.get(key).and_then(Value::as_i64).map(|v| v as i32)
}

#[cfg(test)]
mod tests {
    use super::{candidate_type, parse_candidate_type};
    use assistant_brain_tool::AssistantChangeType;

    #[test]
    fn candidate_type_round_trips_prompt_change() {
        let parsed = parse_candidate_type("prompt_change");
        assert_eq!(candidate_type(&parsed), "prompt_change");
        assert!(matches!(parsed, AssistantChangeType::PromptChange));
    }
}
