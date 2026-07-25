use std::collections::HashSet;

use chrono::{NaiveDate, Utc};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

use super::{
    diff::{diff_sections, SectionChangeType, SectionSnapshot},
    models::{DocumentVersion, LegalDocument, LegalSource, ReviewTask},
};

const MAX_TEXT_CHARS: usize = 1_000_000;
const MAX_SECTIONS: usize = 200;
const PARSER_VERSION: &str = "collector-v1";

#[derive(Debug, Deserialize)]
pub struct CollectorIntakeRequest {
    pub source: CollectorSource,
    pub document: CollectorDocument,
    pub version: CollectorVersion,
    pub analysis: CollectorAnalysis,
}

#[derive(Debug, Deserialize)]
pub struct CollectorSource {
    pub source_key: Option<String>,
    pub title: String,
    pub authority: String,
    pub registry_id: Option<String>,
    pub jurisdiction: Option<String>,
    pub trust_level: Option<String>,
    pub ingestion_mode: Option<String>,
    pub terms_or_reuse_notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CollectorDocument {
    pub official_id: Option<String>,
    pub eli_id: Option<String>,
    pub title: String,
    pub document_type: String,
    pub legal_area: String,
    pub procedure_key: Option<String>,
    pub source_url: String,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CollectorVersion {
    pub version_label: Option<String>,
    pub publication_date: Option<NaiveDate>,
    pub effective_date: Option<NaiveDate>,
    pub version_date: Option<NaiveDate>,
    pub normalized_text: String,
    pub legal_status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CollectorAnalysis {
    pub summary: String,
    pub priority: Option<String>,
    pub change_type: Option<String>,
    pub sections: Vec<CollectorSection>,
    pub model_provider: Option<String>,
    pub model_name: Option<String>,
    pub model_location: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CollectorSection {
    pub stable_section_key: String,
    pub section_type: String,
    pub section_number: Option<String>,
    pub title: String,
    pub text_content: String,
}

#[derive(Debug, Serialize)]
pub struct CollectorIntakeResponse {
    pub status: String,
    pub source_id: Uuid,
    pub document_id: Uuid,
    pub version_id: Option<Uuid>,
    pub review_task_id: Option<Uuid>,
    pub content_hash: String,
    pub versions_created: u32,
    pub sections_created: u32,
    pub review_tasks_created: u32,
}

#[derive(Debug, Serialize)]
pub struct CollectorSqlReport {
    pub source_count: i64,
    pub document_count: i64,
    pub version_count: i64,
    pub section_count: i64,
    pub legal_change_count: i64,
    pub pending_review_task_count: i64,
    pub unchanged_count: i64,
    pub failed_count: i64,
}

pub async fn sql_report(pool: &PgPool) -> AppResult<CollectorSqlReport> {
    let row = sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64)>(
        r#"
        SELECT
          (SELECT count(*) FROM legal_sources),
          (SELECT count(*) FROM legal_documents),
          (SELECT count(*) FROM document_versions),
          (SELECT count(*) FROM document_sections),
          (SELECT count(*) FROM legal_changes),
          (SELECT count(*) FROM review_tasks WHERE status = 'pending')
        "#,
    )
    .fetch_one(pool)
    .await?;
    Ok(CollectorSqlReport {
        source_count: row.0,
        document_count: row.1,
        version_count: row.2,
        section_count: row.3,
        legal_change_count: row.4,
        pending_review_task_count: row.5,
        unchanged_count: 0,
        failed_count: 0,
    })
}

pub async fn ingest(
    pool: &PgPool,
    request: CollectorIntakeRequest,
) -> AppResult<CollectorIntakeResponse> {
    let prepared = PreparedIntake::new(request)?;
    let mut tx = pool.begin().await?;

    let source = upsert_source(&mut tx, &prepared).await?;
    let document = upsert_document(&mut tx, source.id, &prepared).await?;
    lock_document(&mut tx, document.id).await?;

    if let Some(current) = current_version(&mut tx, document.id).await? {
        if current.content_hash == prepared.content_hash {
            touch_document(&mut tx, document.id).await?;
            tx.commit().await?;
            return Ok(CollectorIntakeResponse {
                status: "unchanged".to_owned(),
                source_id: source.id,
                document_id: document.id,
                version_id: Some(current.id),
                review_task_id: None,
                content_hash: prepared.content_hash,
                versions_created: 0,
                sections_created: 0,
                review_tasks_created: 0,
            });
        }
    }

    let old_version = current_version(&mut tx, document.id).await?;
    let old_sections = match &old_version {
        Some(version) => load_sections(&mut tx, version.id).await?,
        None => Vec::new(),
    };

    sqlx::query("UPDATE document_versions SET is_current = false WHERE document_id = $1")
        .bind(document.id)
        .execute(&mut *tx)
        .await?;

    let version = insert_version(&mut tx, document.id, &prepared).await?;
    insert_sections(&mut tx, version.id, &prepared.sections).await?;
    touch_document(&mut tx, document.id).await?;

    let diff = diff_sections(&old_sections, &prepared.sections);
    let affected = diff
        .changes
        .iter()
        .filter(|change| change.change_type != SectionChangeType::Unchanged)
        .collect::<Vec<_>>();
    let diff_type = if old_version.is_some() {
        "version_diff"
    } else {
        "new_document"
    };
    let change_type = if old_version.is_some() {
        prepared.change_type.as_str()
    } else {
        "new_document"
    };
    let model_metadata = json!({
        "provider": prepared.model_provider.as_deref(),
        "model": prepared.model_name.as_deref(),
        "location": prepared.model_location.as_deref(),
        "parser_version": PARSER_VERSION
    });
    let document_diff_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO document_diffs (
          document_id, from_version_id, to_version_id, diff_type, summary, diff_json
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id
        "#,
    )
    .bind(document.id)
    .bind(old_version.as_ref().map(|version| version.id))
    .bind(version.id)
    .bind(diff_type)
    .bind(&prepared.summary)
    .bind(json!({ "section_diff": diff, "model": model_metadata }))
    .fetch_one(&mut *tx)
    .await?;

    let legal_change_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO legal_changes (
          document_diff_id, document_id, change_type, priority, affected_sections,
          detected_summary, requires_analysis, status
        )
        VALUES ($1, $2, $3, $4, $5, $6, true, 'pending_review')
        RETURNING id
        "#,
    )
    .bind(document_diff_id)
    .bind(document.id)
    .bind(change_type)
    .bind(&prepared.priority)
    .bind(json!(affected))
    .bind(&prepared.summary)
    .fetch_one(&mut *tx)
    .await?;

    let review_task = insert_review_task(&mut tx, legal_change_id, document.id, &prepared).await?;
    tx.commit().await?;

    Ok(CollectorIntakeResponse {
        status: "pending_review".to_owned(),
        source_id: source.id,
        document_id: document.id,
        version_id: Some(version.id),
        review_task_id: Some(review_task.id),
        content_hash: prepared.content_hash,
        versions_created: 1,
        sections_created: prepared.sections.len() as u32,
        review_tasks_created: 1,
    })
}

struct PreparedIntake {
    source_key: String,
    source_title: String,
    source_authority: String,
    source_jurisdiction: String,
    source_type: String,
    source_trust_level: String,
    ingestion_mode: String,
    base_url: String,
    terms_or_reuse_notes: String,
    official_id: Option<String>,
    eli_id: Option<String>,
    document_title: String,
    document_type: String,
    legal_area: String,
    procedure_key: Option<String>,
    source_url: String,
    document_status: String,
    version_label: String,
    publication_date: Option<NaiveDate>,
    effective_date: Option<NaiveDate>,
    version_date: Option<NaiveDate>,
    normalized_text: String,
    content_hash: String,
    legal_status: String,
    summary: String,
    priority: String,
    change_type: String,
    sections: Vec<SectionSnapshot>,
    model_provider: Option<String>,
    model_name: Option<String>,
    model_location: Option<String>,
}

impl PreparedIntake {
    fn new(request: CollectorIntakeRequest) -> AppResult<Self> {
        let url = official_url(&request.document.source_url)?;
        let host = url.host_str().unwrap_or_default();
        let registry = trusted_source(host)
            .ok_or_else(|| AppError::BadRequest("source_url domain is not allowed".to_owned()))?;
        if !registry.enabled {
            return Err(AppError::BadRequest(
                "source registry entry is disabled".to_owned(),
            ));
        }
        let registry_id = request.source.registry_id.as_deref().unwrap_or(registry.id);
        if registry_id != registry.id {
            return Err(AppError::BadRequest(
                "source.registry_id does not match source_url domain".to_owned(),
            ));
        }
        let source_type = match registry.id {
            "boe" => "boe",
            "inclusion" => "migraciones",
            "eur_lex" => "eurlex",
            "poder_judicial" | "curia" => "cendoj",
            _ => "manual",
        };
        let source_jurisdiction = request
            .source
            .jurisdiction
            .as_deref()
            .map(|value| bounded_key(value, "source.jurisdiction", 40))
            .transpose()?
            .unwrap_or_else(|| registry.jurisdiction.to_owned());
        let source_trust_level = match request.source.trust_level.as_deref() {
            Some(value) => enum_value(
                value,
                "source.trust_level",
                &[
                    "official",
                    "official_secondary",
                    "court",
                    "manual_import",
                    "unknown",
                ],
            )?,
            None => registry.trust_level.to_owned(),
        };
        let ingestion_mode = request
            .source
            .ingestion_mode
            .as_deref()
            .map(|value| bounded_key(value, "source.ingestion_mode", 80))
            .transpose()?
            .unwrap_or_else(|| registry.ingestion_mode.to_owned());
        let source_key = match request.source.source_key {
            Some(value) => bounded_key(&value, "source.source_key", 120)?,
            None => format!("collector-{source_type}-{}", short_hash(url.as_str())),
        };
        let normalized_text = bounded_text(
            &request.version.normalized_text,
            "version.normalized_text",
            MAX_TEXT_CHARS,
        )?;
        let content_hash = sha256_hex(&normalized_text);
        let sections = prepare_sections(&request.analysis.sections, &normalized_text)?;
        let document_type = enum_value(
            &request.document.document_type,
            "document.document_type",
            &[
                "law",
                "royal_decree",
                "instruction",
                "info_sheet",
                "form",
                "court_decision",
                "guide",
                "manual",
            ],
        )?;
        let legal_status = enum_value(
            request
                .version
                .legal_status
                .as_deref()
                .unwrap_or("effective"),
            "version.legal_status",
            &[
                "published",
                "not_yet_effective",
                "effective",
                "partially_effective",
                "amended",
                "repealed",
                "superseded",
                "pending_consolidation",
            ],
        )?;
        let priority = enum_value(
            request.analysis.priority.as_deref().unwrap_or("medium"),
            "analysis.priority",
            &["low", "medium", "high", "critical"],
        )?;
        let change_type = enum_value(
            request
                .analysis
                .change_type
                .as_deref()
                .unwrap_or("modified_wording"),
            "analysis.change_type",
            &[
                "added_section",
                "removed_section",
                "modified_wording",
                "renumbering",
                "effective_date_change",
                "repeal_notice",
                "procedure_requirement_change",
                "metadata_only",
            ],
        )?;

        Ok(Self {
            source_key,
            source_title: bounded_text(&request.source.title, "source.title", 300)?,
            source_authority: bounded_text(&request.source.authority, "source.authority", 200)?,
            source_jurisdiction,
            source_type: source_type.to_owned(),
            source_trust_level,
            ingestion_mode,
            base_url: format!("https://{host}"),
            terms_or_reuse_notes: request
                .source
                .terms_or_reuse_notes
                .as_deref()
                .map(|value| bounded_text(value, "source.terms_or_reuse_notes", 1_000))
                .transpose()?
                .unwrap_or_else(|| {
                    "Official Spanish public source collected by bounded job.".to_owned()
                }),
            official_id: optional_text(request.document.official_id, "document.official_id", 200)?,
            eli_id: optional_text(request.document.eli_id, "document.eli_id", 500)?,
            document_title: bounded_text(&request.document.title, "document.title", 500)?,
            document_type,
            legal_area: bounded_key(&request.document.legal_area, "document.legal_area", 80)?,
            procedure_key: request
                .document
                .procedure_key
                .map(|value| bounded_key(&value, "document.procedure_key", 120))
                .transpose()?,
            source_url: url.to_string(),
            document_status: enum_value(
                request.document.status.as_deref().unwrap_or("active"),
                "document.status",
                &[
                    "draft",
                    "active",
                    "amended",
                    "repealed",
                    "superseded",
                    "archived",
                ],
            )?,
            version_label: request.version.version_label.unwrap_or_else(|| {
                format!(
                    "collector-{}-{}",
                    Utc::now().format("%Y%m%d"),
                    &content_hash[..12]
                )
            }),
            publication_date: request.version.publication_date,
            effective_date: request.version.effective_date,
            version_date: request.version.version_date,
            normalized_text,
            content_hash,
            legal_status,
            summary: bounded_text(&request.analysis.summary, "analysis.summary", 4_000)?,
            priority,
            change_type,
            sections,
            model_provider: optional_text(
                request.analysis.model_provider,
                "analysis.model_provider",
                80,
            )?,
            model_name: optional_text(request.analysis.model_name, "analysis.model_name", 120)?,
            model_location: optional_text(
                request.analysis.model_location,
                "analysis.model_location",
                80,
            )?,
        })
    }
}

async fn upsert_source(
    tx: &mut Transaction<'_, Postgres>,
    intake: &PreparedIntake,
) -> Result<LegalSource, sqlx::Error> {
    sqlx::query_as::<_, LegalSource>(
        r#"
        INSERT INTO legal_sources (
          source_key, title, authority, jurisdiction, source_type, base_url,
          acquisition_method, trust_level, language, terms_or_reuse_notes, parser_version
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'es', $9, $10)
        ON CONFLICT (source_key) DO UPDATE SET
          title = EXCLUDED.title,
          authority = EXCLUDED.authority,
          jurisdiction = EXCLUDED.jurisdiction,
          base_url = EXCLUDED.base_url,
          acquisition_method = EXCLUDED.acquisition_method,
          trust_level = EXCLUDED.trust_level,
          terms_or_reuse_notes = EXCLUDED.terms_or_reuse_notes,
          parser_version = EXCLUDED.parser_version,
          updated_at = now()
        RETURNING
          id, source_key, title, authority, jurisdiction, source_type, base_url,
          acquisition_method, trust_level, language, enabled, terms_or_reuse_notes,
          parser_version, created_at, updated_at
        "#,
    )
    .bind(&intake.source_key)
    .bind(&intake.source_title)
    .bind(&intake.source_authority)
    .bind(&intake.source_jurisdiction)
    .bind(&intake.source_type)
    .bind(&intake.base_url)
    .bind(&intake.ingestion_mode)
    .bind(&intake.source_trust_level)
    .bind(&intake.terms_or_reuse_notes)
    .bind(PARSER_VERSION)
    .fetch_one(&mut **tx)
    .await
}

async fn upsert_document(
    tx: &mut Transaction<'_, Postgres>,
    source_id: Uuid,
    intake: &PreparedIntake,
) -> Result<LegalDocument, sqlx::Error> {
    if let Some(document) = sqlx::query_as::<_, LegalDocument>(
        r#"
        SELECT
          id, source_id, official_id, eli_id, title, document_type, legal_area, procedure_key,
          source_url, status, first_seen_at, last_checked_at, created_at, updated_at
        FROM legal_documents
        WHERE source_id = $1
          AND (($2::text IS NOT NULL AND official_id = $2) OR source_url = $3)
        ORDER BY created_at ASC
        LIMIT 1
        "#,
    )
    .bind(source_id)
    .bind(&intake.official_id)
    .bind(&intake.source_url)
    .fetch_optional(&mut **tx)
    .await?
    {
        return sqlx::query_as::<_, LegalDocument>(
            r#"
            UPDATE legal_documents
            SET official_id = $2,
                eli_id = $3,
                title = $4,
                document_type = $5,
                legal_area = $6,
                procedure_key = $7,
                source_url = $8,
                status = $9,
                updated_at = now()
            WHERE id = $1
            RETURNING
              id, source_id, official_id, eli_id, title, document_type, legal_area, procedure_key,
              source_url, status, first_seen_at, last_checked_at, created_at, updated_at
            "#,
        )
        .bind(document.id)
        .bind(&intake.official_id)
        .bind(&intake.eli_id)
        .bind(&intake.document_title)
        .bind(&intake.document_type)
        .bind(&intake.legal_area)
        .bind(&intake.procedure_key)
        .bind(&intake.source_url)
        .bind(&intake.document_status)
        .fetch_one(&mut **tx)
        .await;
    }

    sqlx::query_as::<_, LegalDocument>(
        r#"
        INSERT INTO legal_documents (
          source_id, official_id, eli_id, title, document_type, legal_area,
          procedure_key, source_url, status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING
          id, source_id, official_id, eli_id, title, document_type, legal_area, procedure_key,
          source_url, status, first_seen_at, last_checked_at, created_at, updated_at
        "#,
    )
    .bind(source_id)
    .bind(&intake.official_id)
    .bind(&intake.eli_id)
    .bind(&intake.document_title)
    .bind(&intake.document_type)
    .bind(&intake.legal_area)
    .bind(&intake.procedure_key)
    .bind(&intake.source_url)
    .bind(&intake.document_status)
    .fetch_one(&mut **tx)
    .await
}

async fn lock_document(
    tx: &mut Transaction<'_, Postgres>,
    document_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM legal_documents WHERE id = $1 FOR UPDATE")
        .bind(document_id)
        .fetch_one(&mut **tx)
        .await?;
    Ok(())
}

async fn current_version(
    tx: &mut Transaction<'_, Postgres>,
    document_id: Uuid,
) -> Result<Option<DocumentVersion>, sqlx::Error> {
    sqlx::query_as::<_, DocumentVersion>(
        r#"
        SELECT
          id, document_id, version_label, publication_date, effective_date, version_date,
          retrieved_at, source_url, raw_content_path, normalized_text, content_hash,
          parser_version, legal_status, is_current, created_at
        FROM document_versions
        WHERE document_id = $1 AND is_current = true
        ORDER BY retrieved_at DESC
        LIMIT 1
        "#,
    )
    .bind(document_id)
    .fetch_optional(&mut **tx)
    .await
}

async fn load_sections(
    tx: &mut Transaction<'_, Postgres>,
    version_id: Uuid,
) -> Result<Vec<SectionSnapshot>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String, Option<String>, String, String, String)>(
        r#"
        SELECT stable_section_key, section_type, section_number, title, text_content, text_hash
        FROM document_sections
        WHERE version_id = $1
        ORDER BY order_index ASC, stable_section_key ASC
        "#,
    )
    .bind(version_id)
    .fetch_all(&mut **tx)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(stable_section_key, section_type, section_number, title, text_content, text_hash)| {
                SectionSnapshot {
                    stable_section_key,
                    section_type,
                    section_number,
                    title,
                    text_content,
                    text_hash,
                }
            },
        )
        .collect())
}

async fn insert_version(
    tx: &mut Transaction<'_, Postgres>,
    document_id: Uuid,
    intake: &PreparedIntake,
) -> Result<DocumentVersion, sqlx::Error> {
    sqlx::query_as::<_, DocumentVersion>(
        r#"
        INSERT INTO document_versions (
          document_id, version_label, publication_date, effective_date, version_date,
          source_url, normalized_text, content_hash, parser_version, legal_status, is_current
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, true)
        RETURNING
          id, document_id, version_label, publication_date, effective_date, version_date,
          retrieved_at, source_url, raw_content_path, normalized_text, content_hash,
          parser_version, legal_status, is_current, created_at
        "#,
    )
    .bind(document_id)
    .bind(&intake.version_label)
    .bind(intake.publication_date)
    .bind(intake.effective_date)
    .bind(intake.version_date)
    .bind(&intake.source_url)
    .bind(&intake.normalized_text)
    .bind(&intake.content_hash)
    .bind(PARSER_VERSION)
    .bind(&intake.legal_status)
    .fetch_one(&mut **tx)
    .await
}

async fn insert_sections(
    tx: &mut Transaction<'_, Postgres>,
    version_id: Uuid,
    sections: &[SectionSnapshot],
) -> Result<(), sqlx::Error> {
    for (index, section) in sections.iter().enumerate() {
        sqlx::query(
            r#"
            INSERT INTO document_sections (
              version_id, stable_section_key, section_type, section_number,
              title, text_content, text_hash, order_index
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(version_id)
        .bind(&section.stable_section_key)
        .bind(&section.section_type)
        .bind(&section.section_number)
        .bind(&section.title)
        .bind(&section.text_content)
        .bind(&section.text_hash)
        .bind(index as i32)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn touch_document(
    tx: &mut Transaction<'_, Postgres>,
    document_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE legal_documents SET last_checked_at = now(), updated_at = now() WHERE id = $1",
    )
    .bind(document_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_review_task(
    tx: &mut Transaction<'_, Postgres>,
    legal_change_id: Uuid,
    document_id: Uuid,
    intake: &PreparedIntake,
) -> Result<ReviewTask, sqlx::Error> {
    sqlx::query_as::<_, ReviewTask>(
        r#"
        INSERT INTO review_tasks (
          legal_change_id, document_id, task_type, title, status, priority, ai_summary
        )
        VALUES ($1, $2, 'legal_change_review', $3, 'pending', $4, $5)
        RETURNING
          id, legal_change_id, document_id, task_type, title, status, priority,
          ai_summary, reviewer_note, reviewed_by, reviewed_at, created_at, updated_at
        "#,
    )
    .bind(legal_change_id)
    .bind(document_id)
    .bind(format!(
        "Review collected legal source: {}",
        intake.document_title
    ))
    .bind(&intake.priority)
    .bind(&intake.summary)
    .fetch_one(&mut **tx)
    .await
}

fn prepare_sections(
    sections: &[CollectorSection],
    normalized_text: &str,
) -> AppResult<Vec<SectionSnapshot>> {
    if sections.is_empty() || sections.len() > MAX_SECTIONS {
        return Err(AppError::BadRequest(format!(
            "analysis.sections must contain 1..={MAX_SECTIONS} items"
        )));
    }

    let mut seen = HashSet::new();
    sections
        .iter()
        .map(|section| {
            let stable_section_key = bounded_key(
                &section.stable_section_key,
                "section.stable_section_key",
                160,
            )?;
            if !seen.insert(stable_section_key.clone()) {
                return Err(AppError::BadRequest(
                    "section.stable_section_key must be unique".to_owned(),
                ));
            }
            let section_type = enum_value(
                &section.section_type,
                "section.section_type",
                &[
                    "title",
                    "chapter",
                    "article",
                    "section",
                    "paragraph",
                    "additional_provision",
                    "transitional_provision",
                    "repealing_provision",
                    "final_provision",
                    "annex",
                    "other",
                ],
            )?;
            let text_content = bounded_text(&section.text_content, "section.text_content", 80_000)?;
            if !normalized_text.contains(&text_content) {
                return Err(AppError::BadRequest(
                    "section.text_content must be an exact fragment of normalized_text".to_owned(),
                ));
            }
            Ok(SectionSnapshot {
                stable_section_key,
                section_type,
                section_number: section
                    .section_number
                    .as_deref()
                    .map(|value| bounded_text(value, "section.section_number", 80))
                    .transpose()?,
                title: bounded_text(&section.title, "section.title", 300)?,
                text_hash: sha256_hex(&text_content),
                text_content,
            })
        })
        .collect()
}

fn official_url(value: &str) -> AppResult<Url> {
    let url = Url::parse(value)
        .map_err(|_| AppError::BadRequest("document.source_url must be a valid URL".to_owned()))?;
    if url.scheme() != "https" {
        return Err(AppError::BadRequest(
            "document.source_url must use HTTPS".to_owned(),
        ));
    }
    match url.host_str().and_then(trusted_source) {
        Some(source) if source.enabled => Ok(url),
        _ => Err(AppError::BadRequest(
            "document.source_url domain is not allowed".to_owned(),
        )),
    }
}

struct TrustedSource {
    id: &'static str,
    jurisdiction: &'static str,
    trust_level: &'static str,
    ingestion_mode: &'static str,
    enabled: bool,
}

fn trusted_source(host: &str) -> Option<TrustedSource> {
    let source = match host {
        "boe.es" | "www.boe.es" => TrustedSource {
            id: "boe",
            jurisdiction: "ES",
            trust_level: "official",
            ingestion_mode: "html_text",
            enabled: true,
        },
        "administracion.gob.es" | "www.administracion.gob.es" => TrustedSource {
            id: "administracion",
            jurisdiction: "ES",
            trust_level: "official_secondary",
            ingestion_mode: "html_text",
            enabled: true,
        },
        "inclusion.gob.es" | "www.inclusion.gob.es" => TrustedSource {
            id: "inclusion",
            jurisdiction: "ES",
            trust_level: "official_secondary",
            ingestion_mode: "html_text",
            enabled: true,
        },
        "interior.gob.es" | "www.interior.gob.es" => TrustedSource {
            id: "interior",
            jurisdiction: "ES",
            trust_level: "official_secondary",
            ingestion_mode: "html_text",
            enabled: true,
        },
        "exteriores.gob.es" | "www.exteriores.gob.es" => TrustedSource {
            id: "exteriores",
            jurisdiction: "ES",
            trust_level: "official_secondary",
            ingestion_mode: "html_text",
            enabled: true,
        },
        "poderjudicial.es" | "www.poderjudicial.es" => TrustedSource {
            id: "poder_judicial",
            jurisdiction: "ES",
            trust_level: "court",
            ingestion_mode: "html_text",
            enabled: true,
        },
        "eur-lex.europa.eu" => TrustedSource {
            id: "eur_lex",
            jurisdiction: "EU",
            trust_level: "official",
            ingestion_mode: "html_text",
            enabled: true,
        },
        "curia.europa.eu" => TrustedSource {
            id: "curia",
            jurisdiction: "EU",
            trust_level: "court",
            ingestion_mode: "html_text",
            enabled: true,
        },
        _ => return None,
    };
    Some(source)
}

fn enum_value(value: &str, field: &str, allowed: &[&str]) -> AppResult<String> {
    let value = value.trim();
    if allowed.contains(&value) {
        Ok(value.to_owned())
    } else {
        Err(AppError::BadRequest(format!("{field} has invalid value")))
    }
}

fn bounded_key(value: &str, field: &str, max_chars: usize) -> AppResult<String> {
    let value = bounded_text(value, field, max_chars)?;
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'))
    {
        Ok(value)
    } else {
        Err(AppError::BadRequest(format!(
            "{field} contains invalid characters"
        )))
    }
}

fn optional_text(
    value: Option<String>,
    field: &str,
    max_chars: usize,
) -> AppResult<Option<String>> {
    value
        .map(|value| bounded_text(&value, field, max_chars))
        .transpose()
}

fn bounded_text(value: &str, field: &str, max_chars: usize) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::BadRequest(format!("{field} is required")));
    }
    if value.chars().count() > max_chars {
        return Err(AppError::BadRequest(format!("{field} is too long")));
    }
    if value
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(AppError::BadRequest(format!(
            "{field} contains invalid characters"
        )));
    }
    Ok(value.to_owned())
}

fn sha256_hex(input: &str) -> String {
    hex::encode(Sha256::digest(input.as_bytes()))
}

fn short_hash(input: &str) -> String {
    sha256_hex(input).chars().take(16).collect()
}
