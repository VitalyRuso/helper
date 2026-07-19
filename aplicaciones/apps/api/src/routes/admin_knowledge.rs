use crate::{
    db::models::Article,
    error::{AppError, AppResult},
    security,
    state::AppState,
    tool_adapters::knowledge_intake_adapter,
};
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared_contracts::{
    CandidateType, ExtractedFact, FactType, KnowledgeSourceInput, ReviewStatus, RiskLevel,
    SourceType, TrustLevel,
};
use sqlx::FromRow;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sources/scan-docs", post(scan_docs))
        .route("/sources", get(list_sources).post(create_source))
        .route("/sources/:id", get(get_source))
        .route("/sources/:id/analyze-now", post(analyze_now))
        .route("/sources/:id/index", post(queue_index))
        .route("/sources/:id/facts", get(list_facts))
        .route("/candidates", get(list_candidates))
        .route("/candidates/:id", get(get_candidate))
        .route("/candidates/:id/approve", post(approve_candidate))
        .route("/candidates/:id/reject", post(reject_candidate))
}

#[derive(Debug, Serialize, FromRow)]
pub struct DataSourceRow {
    pub id: Uuid,
    pub title: String,
    pub source_type: String,
    pub original_path: Option<String>,
    pub source_url: Option<String>,
    pub raw_text: Option<String>,
    pub trust_level: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ExtractedFactRow {
    pub id: Uuid,
    pub source_id: Uuid,
    pub fact_type: String,
    pub title_ru: String,
    pub text_ru: String,
    pub original_text: String,
    pub confidence: f32,
    pub source_location: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ContentCandidateRow {
    pub id: Uuid,
    pub source_id: Option<Uuid>,
    pub candidate_type: String,
    pub title_ru: String,
    pub summary_ru: String,
    pub body_ru_markdown: String,
    pub category_slug: Option<String>,
    pub risk_level: String,
    pub status: String,
    pub review_note: Option<String>,
    pub article_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct ScanDocsResponse {
    sources: u64,
}

#[derive(Serialize)]
struct AnalyzeResponse {
    facts: Vec<ExtractedFactRow>,
    candidate: ContentCandidateRow,
}

#[derive(Serialize)]
struct CandidateDetails {
    candidate: ContentCandidateRow,
    source: Option<DataSourceRow>,
    facts: Vec<ExtractedFactRow>,
}

#[derive(Deserialize)]
struct CandidateQuery {
    status: Option<String>,
}

#[derive(Deserialize)]
struct ReviewRequest {
    note: Option<String>,
}

#[derive(Serialize)]
struct ApproveResponse {
    candidate: ContentCandidateRow,
    article: Article,
}

async fn scan_docs(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<ScanDocsResponse>> {
    security::require_admin(&headers, &state)?;
    let paths = knowledge_intake_adapter::scan_docs(&state.config.docs_dir)?;
    let mut sources = 0;
    for path in paths {
        let title = path
            .file_stem()
            .and_then(|v| v.to_str())
            .unwrap_or("Document")
            .to_owned();
        let result = sqlx::query(
            r#"
            INSERT INTO data_sources (title, source_type, original_path, trust_level, status)
            VALUES ($1, 'file', $2, 'unknown', 'new')
            ON CONFLICT (original_path) DO NOTHING
            "#,
        )
        .bind(title)
        .bind(path.to_string_lossy().to_string())
        .execute(&state.db)
        .await?;
        sources += result.rows_affected();
    }
    Ok(Json(ScanDocsResponse { sources }))
}

async fn list_sources(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<DataSourceRow>>> {
    security::require_admin(&headers, &state)?;
    Ok(Json(
        sqlx::query_as::<_, DataSourceRow>("SELECT * FROM data_sources ORDER BY updated_at DESC")
            .fetch_all(&state.db)
            .await?,
    ))
}

async fn create_source(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<KnowledgeSourceInput>,
) -> AppResult<Json<DataSourceRow>> {
    security::require_admin(&headers, &state)?;
    Ok(Json(insert_source(&state, input).await?))
}

async fn get_source(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> AppResult<Json<DataSourceRow>> {
    security::require_admin(&headers, &state)?;
    Ok(Json(source_by_id(&state, id).await?))
}

async fn analyze_now(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> AppResult<Json<AnalyzeResponse>> {
    security::require_admin(&headers, &state)?;
    let source = source_by_id(&state, id).await?;
    let output = knowledge_intake_adapter::analyze(row_to_input(&source)?)
        .map_err(|error| AppError::BadRequest(error.to_string()))?;
    let output_json = serde_json::to_value(&output).unwrap_or_else(|_| serde_json::json!({}));
    sqlx::query(
        "INSERT INTO analysis_jobs (source_id, status, output_json) VALUES ($1, 'completed', $2)",
    )
    .bind(id)
    .bind(output_json)
    .execute(&state.db)
    .await?;

    let mut facts = Vec::new();
    for fact in output.facts {
        facts.push(insert_fact(&state, id, fact).await?);
    }
    let candidate = insert_candidate(&state, id, output.candidate).await?;
    sqlx::query("UPDATE data_sources SET status = 'analyzed', updated_at = now() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(AnalyzeResponse { facts, candidate }))
}

async fn queue_index(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    security::require_admin(&headers, &state)?;
    source_by_id(&state, id).await?;
    sqlx::query(
        "INSERT INTO indexing_jobs (source_id, status, message) VALUES ($1, 'queued', 'Review approval required before indexing')",
    )
    .bind(id)
    .execute(&state.db)
    .await?;
    Ok(Json(serde_json::json!({ "queued": true })))
}

async fn list_facts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<ExtractedFactRow>>> {
    security::require_admin(&headers, &state)?;
    Ok(Json(facts_for_source(&state, id).await?))
}

async fn list_candidates(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CandidateQuery>,
) -> AppResult<Json<Vec<ContentCandidateRow>>> {
    security::require_admin(&headers, &state)?;
    let rows = if let Some(status) = query.status {
        sqlx::query_as::<_, ContentCandidateRow>(
            "SELECT * FROM content_candidates WHERE status = $1 ORDER BY updated_at DESC",
        )
        .bind(status)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, ContentCandidateRow>(
            "SELECT * FROM content_candidates ORDER BY updated_at DESC",
        )
        .fetch_all(&state.db)
        .await?
    };
    Ok(Json(rows))
}

async fn get_candidate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> AppResult<Json<CandidateDetails>> {
    security::require_admin(&headers, &state)?;
    let candidate = candidate_by_id(&state, id).await?;
    let source = match candidate.source_id {
        Some(source_id) => Some(source_by_id(&state, source_id).await?),
        None => None,
    };
    let facts = match candidate.source_id {
        Some(source_id) => facts_for_source(&state, source_id).await?,
        None => vec![],
    };
    Ok(Json(CandidateDetails {
        candidate,
        source,
        facts,
    }))
}

async fn approve_candidate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ApproveResponse>> {
    security::require_admin(&headers, &state)?;
    let candidate = candidate_by_id(&state, id).await?;
    if candidate.status != "draft" {
        return Err(AppError::BadRequest(
            "candidate is already reviewed".to_owned(),
        ));
    }
    let category_id = match candidate.category_slug.as_deref() {
        Some(slug) => {
            sqlx::query_scalar::<_, Uuid>("SELECT id FROM categories WHERE slug = $1")
                .bind(slug)
                .fetch_optional(&state.db)
                .await?
        }
        None => None,
    };
    let article = sqlx::query_as::<_, Article>(
        r#"
        INSERT INTO articles (
          category_id, title_ru, slug, summary_ru, body_ru_markdown, tags,
          source_references, legal_risk_level, is_published, include_in_ai
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,false,false)
        RETURNING *
        "#,
    )
    .bind(category_id)
    .bind(&candidate.title_ru)
    .bind(slug_for(&candidate.title_ru, candidate.id))
    .bind(&candidate.summary_ru)
    .bind(&candidate.body_ru_markdown)
    .bind(Vec::<String>::new())
    .bind(serde_json::json!({
        "content_candidate_id": candidate.id,
        "source_id": candidate.source_id,
    }))
    .bind(&candidate.risk_level)
    .fetch_one(&state.db)
    .await?;
    let candidate = sqlx::query_as::<_, ContentCandidateRow>(
        "UPDATE content_candidates SET status = 'approved', article_id = $1, updated_at = now() WHERE id = $2 RETURNING *",
    )
    .bind(article.id)
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    Ok(Json(ApproveResponse { candidate, article }))
}

async fn reject_candidate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<ReviewRequest>,
) -> AppResult<Json<ContentCandidateRow>> {
    security::require_admin(&headers, &state)?;
    Ok(Json(
        sqlx::query_as::<_, ContentCandidateRow>(
            "UPDATE content_candidates SET status = 'rejected', review_note = $1, updated_at = now() WHERE id = $2 RETURNING *",
        )
        .bind(input.note.unwrap_or_default())
        .bind(id)
        .fetch_one(&state.db)
        .await?,
    ))
}

async fn insert_source(
    state: &AppState,
    input: KnowledgeSourceInput,
) -> Result<DataSourceRow, sqlx::Error> {
    sqlx::query_as::<_, DataSourceRow>(
        r#"
        INSERT INTO data_sources (title, source_type, original_path, source_url, raw_text, trust_level, status)
        VALUES ($1,$2,$3,$4,$5,$6,'new')
        RETURNING *
        "#,
    )
    .bind(input.title)
    .bind(source_type(&input.source_type))
    .bind(input.original_path)
    .bind(input.source_url)
    .bind(input.raw_text)
    .bind(trust_level(&input.trust_level))
    .fetch_one(&state.db)
    .await
}

async fn insert_fact(
    state: &AppState,
    source_id: Uuid,
    fact: ExtractedFact,
) -> Result<ExtractedFactRow, sqlx::Error> {
    sqlx::query_as::<_, ExtractedFactRow>(
        r#"
        INSERT INTO extracted_facts (source_id, fact_type, title_ru, text_ru, original_text, confidence, source_location)
        VALUES ($1,$2,$3,$4,$5,$6,$7)
        RETURNING *
        "#,
    )
    .bind(source_id)
    .bind(fact_type(&fact.fact_type))
    .bind(fact.title_ru)
    .bind(fact.text_ru)
    .bind(fact.original_text)
    .bind(fact.confidence)
    .bind(fact.source_location)
    .fetch_one(&state.db)
    .await
}

async fn insert_candidate(
    state: &AppState,
    source_id: Uuid,
    candidate: shared_contracts::ContentCandidate,
) -> Result<ContentCandidateRow, sqlx::Error> {
    sqlx::query_as::<_, ContentCandidateRow>(
        r#"
        INSERT INTO content_candidates (
          source_id, candidate_type, title_ru, summary_ru, body_ru_markdown,
          category_slug, risk_level, status
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
        RETURNING *
        "#,
    )
    .bind(source_id)
    .bind(candidate_type(&candidate.candidate_type))
    .bind(candidate.title_ru)
    .bind(candidate.summary_ru)
    .bind(candidate.body_ru_markdown)
    .bind(candidate.category_slug)
    .bind(risk_level(&candidate.risk_level))
    .bind(review_status(&candidate.status))
    .fetch_one(&state.db)
    .await
}

async fn source_by_id(state: &AppState, id: Uuid) -> Result<DataSourceRow, sqlx::Error> {
    sqlx::query_as::<_, DataSourceRow>("SELECT * FROM data_sources WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await
}

async fn candidate_by_id(state: &AppState, id: Uuid) -> Result<ContentCandidateRow, sqlx::Error> {
    sqlx::query_as::<_, ContentCandidateRow>("SELECT * FROM content_candidates WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await
}

async fn facts_for_source(
    state: &AppState,
    source_id: Uuid,
) -> Result<Vec<ExtractedFactRow>, sqlx::Error> {
    sqlx::query_as::<_, ExtractedFactRow>(
        "SELECT * FROM extracted_facts WHERE source_id = $1 ORDER BY created_at DESC",
    )
    .bind(source_id)
    .fetch_all(&state.db)
    .await
}

fn row_to_input(row: &DataSourceRow) -> AppResult<KnowledgeSourceInput> {
    Ok(KnowledgeSourceInput {
        title: row.title.clone(),
        source_type: parse_source_type(&row.source_type)?,
        original_path: row.original_path.clone(),
        source_url: row.source_url.clone(),
        raw_text: row.raw_text.clone(),
        trust_level: parse_trust_level(&row.trust_level),
    })
}

fn parse_source_type(value: &str) -> AppResult<SourceType> {
    match value {
        "file" => Ok(SourceType::File),
        "url" => Ok(SourceType::Url),
        "pasted_text" => Ok(SourceType::PastedText),
        "manual_note" => Ok(SourceType::ManualNote),
        other => Err(AppError::BadRequest(format!(
            "unknown source_type: {other}"
        ))),
    }
}

fn parse_trust_level(value: &str) -> TrustLevel {
    match value {
        "official" => TrustLevel::Official,
        "semi_official" => TrustLevel::SemiOfficial,
        "user_provided" => TrustLevel::UserProvided,
        _ => TrustLevel::Unknown,
    }
}

fn source_type(value: &SourceType) -> &'static str {
    match value {
        SourceType::File => "file",
        SourceType::Url => "url",
        SourceType::PastedText => "pasted_text",
        SourceType::ManualNote => "manual_note",
    }
}

fn trust_level(value: &TrustLevel) -> &'static str {
    match value {
        TrustLevel::Official => "official",
        TrustLevel::SemiOfficial => "semi_official",
        TrustLevel::UserProvided => "user_provided",
        TrustLevel::Unknown => "unknown",
    }
}

fn fact_type(value: &FactType) -> &'static str {
    match value {
        FactType::Requirement => "requirement",
        FactType::Deadline => "deadline",
        FactType::Fee => "fee",
        FactType::Document => "document",
        FactType::ProcedureStep => "procedure_step",
        FactType::Warning => "warning",
        FactType::Definition => "definition",
        FactType::Contact => "contact",
        FactType::SourceReference => "source_reference",
        FactType::Unknown => "unknown",
    }
}

fn candidate_type(value: &CandidateType) -> &'static str {
    match value {
        CandidateType::Article => "article",
        CandidateType::Guide => "guide",
        CandidateType::Checklist => "checklist",
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

fn slug_for(title: &str, id: Uuid) -> String {
    let slug = title
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let suffix = id.to_string()[..8].to_owned();
    if slug.is_empty() {
        format!("candidate-{suffix}")
    } else {
        format!("{slug}-{suffix}")
    }
}

#[cfg(test)]
mod tests {
    use super::slug_for;
    use uuid::Uuid;

    #[test]
    fn approval_slug_has_uuid_suffix() {
        let id = Uuid::nil();
        assert_eq!(slug_for("Test title", id), "test-title-00000000");
    }
}
