use sqlx::PgPool;

use super::models::{LegalSource, ReviewTask};

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
