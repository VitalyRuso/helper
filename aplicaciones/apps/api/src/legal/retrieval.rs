use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

use super::models::{
    DocumentCurrentness, DocumentDiff, DocumentSection, DocumentVersion, KnowledgeItemView,
    LegalChange, LegalDocument, LegalSource, ReviewContext, ReviewTask,
};

pub async fn get_review_task_context(pool: &PgPool, task_id: Uuid) -> AppResult<ReviewContext> {
    let task = get_review_task(pool, task_id).await?;
    let change_id = task.legal_change_id.ok_or_else(|| {
        AppError::Conflict(format!(
            "review task {} is not linked to a legal change",
            task.id
        ))
    })?;
    let change = get_legal_change(pool, change_id).await?;
    build_review_context(pool, Some(task), change).await
}

pub async fn get_legal_change_context(pool: &PgPool, change_id: Uuid) -> AppResult<ReviewContext> {
    let change = get_legal_change(pool, change_id).await?;
    let task = sqlx::query_as::<_, ReviewTask>(
        r#"
        SELECT
          id,
          legal_change_id,
          document_id,
          task_type,
          title,
          status,
          priority,
          ai_summary,
          reviewer_note,
          reviewed_by,
          reviewed_at,
          created_at,
          updated_at
        FROM review_tasks
        WHERE legal_change_id = $1
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(change_id)
    .fetch_optional(pool)
    .await?;

    build_review_context(pool, task, change).await
}

async fn get_review_task(pool: &PgPool, task_id: Uuid) -> AppResult<ReviewTask> {
    sqlx::query_as::<_, ReviewTask>(
        r#"
        SELECT
          id,
          legal_change_id,
          document_id,
          task_type,
          title,
          status,
          priority,
          ai_summary,
          reviewer_note,
          reviewed_by,
          reviewed_at,
          created_at,
          updated_at
        FROM review_tasks
        WHERE id = $1
        "#,
    )
    .bind(task_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("review task {task_id} not found")))
}

async fn get_legal_change(pool: &PgPool, change_id: Uuid) -> AppResult<LegalChange> {
    sqlx::query_as::<_, LegalChange>(
        r#"
        SELECT
          id,
          document_diff_id,
          document_id,
          change_type,
          priority,
          affected_sections,
          detected_summary,
          requires_analysis,
          status,
          created_at,
          updated_at
        FROM legal_changes
        WHERE id = $1
        "#,
    )
    .bind(change_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("legal change {change_id} not found")))
}

pub async fn get_legal_document(pool: &PgPool, document_id: Uuid) -> AppResult<LegalDocument> {
    sqlx::query_as::<_, LegalDocument>(
        r#"
        SELECT
          id,
          source_id,
          official_id,
          eli_id,
          title,
          document_type,
          legal_area,
          procedure_key,
          source_url,
          status,
          first_seen_at,
          last_checked_at,
          created_at,
          updated_at
        FROM legal_documents
        WHERE id = $1
        "#,
    )
    .bind(document_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("legal document {document_id} not found")))
}

async fn get_document_diff(pool: &PgPool, change: &LegalChange) -> AppResult<DocumentDiff> {
    sqlx::query_as::<_, DocumentDiff>(
        r#"
        SELECT
          id,
          document_id,
          from_version_id,
          to_version_id,
          diff_type,
          summary,
          diff_json,
          created_at
        FROM document_diffs
        WHERE id = $1
          AND document_id = $2
        "#,
    )
    .bind(change.document_diff_id)
    .bind(change.document_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        AppError::Conflict(format!(
            "legal change {} has inconsistent diff linkage",
            change.id
        ))
    })
}

async fn get_legal_source(pool: &PgPool, source_id: Uuid) -> AppResult<LegalSource> {
    sqlx::query_as::<_, LegalSource>(
        r#"
        SELECT
          id,
          source_key,
          title,
          authority,
          jurisdiction,
          source_type,
          base_url,
          acquisition_method,
          trust_level,
          language,
          enabled,
          terms_or_reuse_notes,
          parser_version,
          created_at,
          updated_at
        FROM legal_sources
        WHERE id = $1
        "#,
    )
    .bind(source_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("legal source {source_id} not found")))
}

pub async fn list_document_versions(
    pool: &PgPool,
    document_id: Uuid,
) -> Result<Vec<DocumentVersion>, sqlx::Error> {
    sqlx::query_as::<_, DocumentVersion>(
        r#"
        SELECT
          id,
          document_id,
          version_label,
          publication_date,
          effective_date,
          version_date,
          retrieved_at,
          source_url,
          raw_content_path,
          normalized_text,
          content_hash,
          parser_version,
          legal_status,
          is_current,
          created_at
        FROM document_versions
        WHERE document_id = $1
        ORDER BY is_current DESC, version_date DESC NULLS LAST, retrieved_at DESC
        "#,
    )
    .bind(document_id)
    .fetch_all(pool)
    .await
}

pub async fn list_current_document_sections(
    pool: &PgPool,
    document_id: Uuid,
) -> Result<Vec<DocumentSection>, sqlx::Error> {
    sqlx::query_as::<_, DocumentSection>(
        r#"
        SELECT
          s.id,
          s.version_id,
          s.stable_section_key,
          s.section_type,
          s.section_number,
          s.title,
          s.text_content,
          s.text_hash,
          s.order_index,
          s.parent_section_id,
          s.created_at
        FROM document_sections s
        JOIN document_versions v ON v.id = s.version_id
        WHERE v.document_id = $1
          AND v.is_current = true
          AND (
            SELECT count(*)
            FROM document_versions current_version
            WHERE current_version.document_id = $1
              AND current_version.is_current = true
          ) = 1
        ORDER BY s.order_index ASC, s.stable_section_key ASC
        "#,
    )
    .bind(document_id)
    .fetch_all(pool)
    .await
}

async fn list_affected_sections(
    pool: &PgPool,
    change: &LegalChange,
    diff: &DocumentDiff,
) -> Result<Vec<DocumentSection>, sqlx::Error> {
    sqlx::query_as::<_, DocumentSection>(
        r#"
        SELECT DISTINCT
          s.id,
          s.version_id,
          s.stable_section_key,
          s.section_type,
          s.section_number,
          s.title,
          s.text_content,
          s.text_hash,
          s.order_index,
          s.parent_section_id,
          s.created_at
        FROM document_sections s
        JOIN jsonb_array_elements($3::jsonb) AS affected
          ON affected ->> 'stable_section_key' = s.stable_section_key
        WHERE s.version_id IN ($1, $2)
          AND affected ->> 'change_type' <> 'unchanged'
        ORDER BY s.version_id, s.order_index ASC, s.stable_section_key ASC
        "#,
    )
    .bind(diff.from_version_id)
    .bind(diff.to_version_id)
    .bind(&change.affected_sections)
    .fetch_all(pool)
    .await
}

async fn build_review_context(
    pool: &PgPool,
    task: Option<ReviewTask>,
    change: LegalChange,
) -> AppResult<ReviewContext> {
    let document = get_legal_document(pool, change.document_id).await?;
    let source = get_legal_source(pool, document.source_id).await?;
    let diff = get_document_diff(pool, &change).await?;
    let versions = list_document_versions(pool, document.id).await?;
    let affected_sections = list_affected_sections(pool, &change, &diff).await?;
    let current_versions = versions
        .iter()
        .filter(|version| version.is_current)
        .collect::<Vec<_>>();
    let current_version_id = (current_versions.len() == 1).then(|| current_versions[0].id);
    let reviewed_version_is_current = current_version_id == Some(diff.to_version_id);

    Ok(ReviewContext {
        task,
        legal_change: change,
        document,
        source,
        currentness: DocumentCurrentness {
            reviewed_version_id: diff.to_version_id,
            current_version_id,
            reviewed_version_is_current,
            is_stale: !reviewed_version_is_current,
        },
        diff,
        affected_sections,
        versions,
    })
}

pub async fn list_knowledge_items(pool: &PgPool) -> Result<Vec<KnowledgeItemView>, sqlx::Error> {
    query_knowledge_items(pool, false, None).await
}

pub async fn list_approved_knowledge(pool: &PgPool) -> Result<Vec<KnowledgeItemView>, sqlx::Error> {
    query_knowledge_items(pool, true, None).await
}

pub async fn search_approved_knowledge(
    pool: &PgPool,
    search: &str,
) -> Result<Vec<KnowledgeItemView>, sqlx::Error> {
    query_knowledge_items(pool, true, Some(search.trim())).await
}

async fn query_knowledge_items(
    pool: &PgPool,
    safe_only: bool,
    search: Option<&str>,
) -> Result<Vec<KnowledgeItemView>, sqlx::Error> {
    // ponytail: bound ILIKE is enough here; add a knowledge FTS index when volume demands it.
    sqlx::query_as::<_, KnowledgeItemView>(
        r#"
        SELECT
          k.id,
          k.procedure_key,
          k.topic_key,
          k.title_es,
          k.canonical_answer_es,
          k.summary_ru,
          k.summary_en,
          k.conditions_json,
          k.required_evidence_json,
          k.source_refs_json,
          k.review_task_id,
          k.status,
          k.effective_from,
          k.effective_until,
          k.approved_by,
          k.approved_at,
          k.created_at,
          k.updated_at,
          d.id AS document_id,
          reviewed_version.id AS reviewed_version_id,
          current_version.id AS current_version_id,
          COALESCE(
            current_version.current_count = 1
            AND reviewed_version.id = current_version.id
            AND reviewed_version.is_current,
            false
          ) AS reviewed_version_is_current,
          NOT COALESCE(
            current_version.current_count = 1
            AND reviewed_version.id = current_version.id
            AND reviewed_version.is_current,
            false
          ) AS is_stale,
          reviewed_version.legal_status,
          d.legal_area,
          d.title AS document_title,
          COALESCE(reviewed_version.source_url, d.source_url) AS source_url,
          source.authority,
          d.official_id,
          d.eli_id,
          reviewed_version.version_label,
          reviewed_version.version_date,
          reviewed_version.retrieved_at,
          d.last_checked_at
        FROM knowledge_items k
        LEFT JOIN review_tasks task ON task.id = k.review_task_id
        LEFT JOIN legal_changes change ON change.id = task.legal_change_id
        LEFT JOIN document_diffs diff
          ON diff.id = change.document_diff_id
         AND diff.document_id = change.document_id
        LEFT JOIN document_versions reviewed_version
          ON reviewed_version.id = diff.to_version_id
         AND reviewed_version.document_id = change.document_id
        LEFT JOIN legal_documents d ON d.id = change.document_id
        LEFT JOIN legal_sources source ON source.id = d.source_id
        LEFT JOIN LATERAL (
          SELECT
            version.id,
            count(*) OVER () AS current_count
          FROM document_versions version
          WHERE version.document_id = d.id
            AND version.is_current = true
          ORDER BY version.retrieved_at DESC
          LIMIT 1
        ) AS current_version ON true
        WHERE (
          NOT $1
          OR (
            k.status IN ('approved', 'published')
            AND task.status IN ('approved', 'published')
            AND task.task_type = 'legal_change_review'
            AND task.document_id = change.document_id
            AND task.reviewed_by IS NOT NULL
            AND btrim(task.reviewed_by) <> ''
            AND task.reviewed_at IS NOT NULL
            AND change.status = 'approved'
            AND k.approved_by IS NOT NULL
            AND btrim(k.approved_by) <> ''
            AND k.approved_at IS NOT NULL
            AND btrim(k.canonical_answer_es) <> ''
            AND current_version.current_count = 1
            AND reviewed_version.id = current_version.id
            AND reviewed_version.is_current = true
            AND reviewed_version.legal_status IN ('effective', 'partially_effective')
            AND d.status IN ('active', 'amended')
            AND source.enabled = true
            AND source.language = 'es'
            AND (reviewed_version.effective_date IS NULL OR reviewed_version.effective_date <= current_date)
            AND (k.effective_from IS NULL OR k.effective_from <= current_date)
            AND (k.effective_until IS NULL OR k.effective_until >= current_date)
          )
        )
          AND (
            $2::text IS NULL
            OR k.title_es ILIKE '%' || $2 || '%'
            OR k.canonical_answer_es ILIKE '%' || $2 || '%'
            OR k.procedure_key ILIKE '%' || $2 || '%'
          )
        ORDER BY k.updated_at DESC
        "#,
    )
    .bind(safe_only)
    .bind(search)
    .fetch_all(pool)
    .await
}
