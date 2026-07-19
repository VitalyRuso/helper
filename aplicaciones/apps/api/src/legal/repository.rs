use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use super::models::{
    DocumentDiff, DocumentSection, DocumentVersion, LegalChange, LegalDocument, LegalSource,
    ReviewTask,
};

pub use super::retrieval::{
    get_legal_change_context, get_legal_document, get_review_task_context, list_approved_knowledge,
    list_current_document_sections, list_document_versions, list_knowledge_items,
    search_approved_knowledge,
};
pub use super::review::{approve_review_task, reject_review_task};

pub async fn list_legal_sources(pool: &PgPool) -> Result<Vec<LegalSource>, sqlx::Error> {
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
        ORDER BY title ASC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn list_pending_review_tasks(pool: &PgPool) -> Result<Vec<ReviewTask>, sqlx::Error> {
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
        WHERE status = 'pending'
        ORDER BY
          CASE priority
            WHEN 'critical' THEN 1
            WHEN 'high' THEN 2
            WHEN 'medium' THEN 3
            ELSE 4
          END,
          created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn upsert_fixture_source(pool: &PgPool) -> Result<LegalSource, sqlx::Error> {
    sqlx::query_as::<_, LegalSource>(
        r#"
        INSERT INTO legal_sources (
          source_key,
          title,
          authority,
          jurisdiction,
          source_type,
          base_url,
          acquisition_method,
          trust_level,
          language,
          terms_or_reuse_notes,
          parser_version
        )
        VALUES (
          'fixture-migraciones',
          'Fixture Migraciones Source',
          'Spain Helper Fixture',
          'ES',
          'manual',
          NULL,
          'fixture',
          'manual_import',
          'es',
          'Synthetic local fixture for development only.',
          'fixture-v1'
        )
        ON CONFLICT (source_key) DO UPDATE SET
          updated_at = now()
        RETURNING
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
        "#,
    )
    .fetch_one(pool)
    .await
}

pub async fn upsert_fixture_document(
    pool: &PgPool,
    source_id: Uuid,
) -> Result<LegalDocument, sqlx::Error> {
    let inserted = sqlx::query_as::<_, LegalDocument>(
        r#"
        INSERT INTO legal_documents (
          source_id,
          official_id,
          eli_id,
          title,
          document_type,
          legal_area,
          procedure_key,
          source_url,
          status
        )
        VALUES (
          $1,
          'FIXTURE-EXTRANJERIA-001',
          NULL,
          'Instrucción de extranjería de prueba',
          'instruction',
          'immigration',
          'fixture_extranjeria',
          NULL,
          'active'
        )
        ON CONFLICT DO NOTHING
        RETURNING
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
        "#,
    )
    .bind(source_id)
    .fetch_optional(pool)
    .await?;

    if let Some(document) = inserted {
        return Ok(document);
    }

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
        WHERE official_id = 'FIXTURE-EXTRANJERIA-001'
        LIMIT 1
        "#,
    )
    .fetch_one(pool)
    .await
}

pub async fn insert_document_version(
    pool: &PgPool,
    document_id: Uuid,
    version_label: &str,
    normalized_text: &str,
    content_hash: &str,
    is_current: bool,
) -> Result<DocumentVersion, sqlx::Error> {
    let mut tx = pool.begin().await?;

    if is_current {
        // ponytail: serializes app writers; add a partial unique index if versions get external writers.
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM legal_documents WHERE id = $1 FOR UPDATE")
            .bind(document_id)
            .fetch_one(&mut *tx)
            .await?;

        sqlx::query(
            r#"
            UPDATE document_versions
            SET is_current = false
            WHERE document_id = $1
            "#,
        )
        .bind(document_id)
        .execute(&mut *tx)
        .await?;
    }

    let version = sqlx::query_as::<_, DocumentVersion>(
        r#"
        INSERT INTO document_versions (
          document_id,
          version_label,
          normalized_text,
          content_hash,
          parser_version,
          legal_status,
          is_current
        )
        VALUES ($1, $2, $3, $4, 'fixture-v1', 'effective', $5)
        RETURNING
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
        "#,
    )
    .bind(document_id)
    .bind(version_label)
    .bind(normalized_text)
    .bind(content_hash)
    .bind(is_current)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(version)
}

pub async fn insert_document_section(
    pool: &PgPool,
    version_id: Uuid,
    stable_section_key: &str,
    section_type: &str,
    section_number: Option<&str>,
    title: &str,
    text_content: &str,
    text_hash: &str,
    order_index: i32,
) -> Result<DocumentSection, sqlx::Error> {
    sqlx::query_as::<_, DocumentSection>(
        r#"
        INSERT INTO document_sections (
          version_id,
          stable_section_key,
          section_type,
          section_number,
          title,
          text_content,
          text_hash,
          order_index
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING
          id,
          version_id,
          stable_section_key,
          section_type,
          section_number,
          title,
          text_content,
          text_hash,
          order_index,
          parent_section_id,
          created_at
        "#,
    )
    .bind(version_id)
    .bind(stable_section_key)
    .bind(section_type)
    .bind(section_number)
    .bind(title)
    .bind(text_content)
    .bind(text_hash)
    .bind(order_index)
    .fetch_one(pool)
    .await
}

pub async fn insert_document_diff(
    pool: &PgPool,
    document_id: Uuid,
    from_version_id: Option<Uuid>,
    to_version_id: Uuid,
    diff_type: &str,
    summary: &str,
    diff_json: Value,
) -> Result<DocumentDiff, sqlx::Error> {
    sqlx::query_as::<_, DocumentDiff>(
        r#"
        INSERT INTO document_diffs (
          document_id,
          from_version_id,
          to_version_id,
          diff_type,
          summary,
          diff_json
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING
          id,
          document_id,
          from_version_id,
          to_version_id,
          diff_type,
          summary,
          diff_json,
          created_at
        "#,
    )
    .bind(document_id)
    .bind(from_version_id)
    .bind(to_version_id)
    .bind(diff_type)
    .bind(summary)
    .bind(diff_json)
    .fetch_one(pool)
    .await
}

pub async fn insert_legal_change(
    pool: &PgPool,
    document_diff_id: Uuid,
    document_id: Uuid,
    change_type: &str,
    priority: &str,
    affected_sections: Value,
    detected_summary: &str,
) -> Result<LegalChange, sqlx::Error> {
    sqlx::query_as::<_, LegalChange>(
        r#"
        INSERT INTO legal_changes (
          document_diff_id,
          document_id,
          change_type,
          priority,
          affected_sections,
          detected_summary,
          requires_analysis,
          status
        )
        VALUES ($1, $2, $3, $4, $5, $6, true, 'pending_review')
        RETURNING
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
        "#,
    )
    .bind(document_diff_id)
    .bind(document_id)
    .bind(change_type)
    .bind(priority)
    .bind(affected_sections)
    .bind(detected_summary)
    .fetch_one(pool)
    .await
}

pub async fn insert_review_task(
    pool: &PgPool,
    legal_change_id: Uuid,
    document_id: Uuid,
    title: &str,
    priority: &str,
    ai_summary: &str,
) -> Result<ReviewTask, sqlx::Error> {
    sqlx::query_as::<_, ReviewTask>(
        r#"
        INSERT INTO review_tasks (
          legal_change_id,
          document_id,
          task_type,
          title,
          status,
          priority,
          ai_summary
        )
        VALUES ($1, $2, 'legal_change_review', $3, 'pending', $4, $5)
        RETURNING
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
        "#,
    )
    .bind(legal_change_id)
    .bind(document_id)
    .bind(title)
    .bind(priority)
    .bind(ai_summary)
    .fetch_one(pool)
    .await
}
