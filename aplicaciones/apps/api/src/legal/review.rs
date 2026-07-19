use chrono::NaiveDate;
use serde_json::{json, Value};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

use super::models::{
    DocumentSection, KnowledgeItem, LegalChange, ReviewDecisionResult, ReviewTask,
};

pub async fn approve_review_task(
    pool: &PgPool,
    task_id: Uuid,
    reviewed_by: Option<&str>,
    reviewer_note: Option<&str>,
) -> AppResult<ReviewDecisionResult> {
    let reviewed_by = reviewer_name(reviewed_by);
    let reviewer_note = reviewer_note.unwrap_or_default().trim();
    let mut tx = pool.begin().await?;
    let task = lock_review_task(&mut tx, task_id).await?;
    let change = lock_legal_change(&mut tx, &task).await?;

    ensure_pending_transition(&task, &change)?;

    sqlx::query(
        r#"
        UPDATE legal_changes
        SET status = 'approved',
            updated_at = now()
        WHERE id = $1
          AND status = 'pending_review'
        "#,
    )
    .bind(change.id)
    .execute(&mut *tx)
    .await?;

    let task = sqlx::query_as::<_, ReviewTask>(
        r#"
        UPDATE review_tasks
        SET status = 'approved',
            reviewed_by = $2,
            reviewer_note = $3,
            reviewed_at = now(),
            updated_at = now()
        WHERE id = $1
          AND status = 'pending'
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
    .bind(task_id)
    .bind(reviewed_by)
    .bind(reviewer_note)
    .fetch_one(&mut *tx)
    .await?;

    let knowledge_item =
        materialize_approved_knowledge(&mut tx, &task, &change, reviewed_by, reviewer_note).await?;

    insert_review_audit(
        &mut tx,
        "legal_review.approved",
        &task,
        &change,
        reviewed_by,
        reviewer_note,
        Some(knowledge_item.id),
    )
    .await?;

    tx.commit().await?;

    Ok(ReviewDecisionResult {
        task,
        knowledge_item: Some(knowledge_item),
    })
}

pub async fn reject_review_task(
    pool: &PgPool,
    task_id: Uuid,
    reviewed_by: Option<&str>,
    reviewer_note: Option<&str>,
) -> AppResult<ReviewDecisionResult> {
    let reviewed_by = reviewer_name(reviewed_by);
    let reviewer_note = reviewer_note.unwrap_or_default().trim();
    let mut tx = pool.begin().await?;
    let task = lock_review_task(&mut tx, task_id).await?;
    let change = lock_legal_change(&mut tx, &task).await?;

    ensure_pending_transition(&task, &change)?;

    sqlx::query(
        r#"
        UPDATE legal_changes
        SET status = 'rejected',
            updated_at = now()
        WHERE id = $1
          AND status = 'pending_review'
        "#,
    )
    .bind(change.id)
    .execute(&mut *tx)
    .await?;

    let task = sqlx::query_as::<_, ReviewTask>(
        r#"
        UPDATE review_tasks
        SET status = 'rejected',
            reviewed_by = $2,
            reviewer_note = $3,
            reviewed_at = now(),
            updated_at = now()
        WHERE id = $1
          AND status = 'pending'
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
    .bind(task_id)
    .bind(reviewed_by)
    .bind(reviewer_note)
    .fetch_one(&mut *tx)
    .await?;

    insert_review_audit(
        &mut tx,
        "legal_review.rejected",
        &task,
        &change,
        reviewed_by,
        reviewer_note,
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(ReviewDecisionResult {
        task,
        knowledge_item: None,
    })
}

fn reviewer_name(value: Option<&str>) -> &str {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Legal Reviewer")
}

async fn lock_review_task(
    tx: &mut Transaction<'_, Postgres>,
    task_id: Uuid,
) -> AppResult<ReviewTask> {
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
        FOR UPDATE
        "#,
    )
    .bind(task_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("review task {task_id} not found")))
}

async fn lock_legal_change(
    tx: &mut Transaction<'_, Postgres>,
    task: &ReviewTask,
) -> AppResult<LegalChange> {
    let change_id = task.legal_change_id.ok_or_else(|| {
        AppError::Conflict(format!(
            "review task {} is not linked to a legal change",
            task.id
        ))
    })?;

    let change = sqlx::query_as::<_, LegalChange>(
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
        FOR UPDATE
        "#,
    )
    .bind(change_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("legal change {change_id} not found")))?;

    if task.document_id != Some(change.document_id) {
        return Err(AppError::Conflict(format!(
            "review task {} has inconsistent document linkage",
            task.id
        )));
    }

    Ok(change)
}

fn ensure_pending_transition(task: &ReviewTask, change: &LegalChange) -> AppResult<()> {
    if task.task_type != "legal_change_review" {
        return Err(AppError::Conflict(format!(
            "review task {} has unsupported type {}",
            task.id, task.task_type
        )));
    }
    if task.status != "pending" {
        return Err(AppError::Conflict(format!(
            "review task {} cannot transition from {}",
            task.id, task.status
        )));
    }
    if change.status != "pending_review" {
        return Err(AppError::Conflict(format!(
            "legal change {} cannot transition from {}",
            change.id, change.status
        )));
    }
    Ok(())
}

#[derive(FromRow)]
struct KnowledgeMaterial {
    legal_change_id: Uuid,
    document_diff_id: Uuid,
    document_id: Uuid,
    source_id: Uuid,
    source_key: String,
    authority: String,
    source_language: String,
    official_id: Option<String>,
    eli_id: Option<String>,
    document_title: String,
    legal_area: String,
    procedure_key: Option<String>,
    document_source_url: Option<String>,
    version_id: Uuid,
    version_label: String,
    version_source_url: Option<String>,
    effective_date: Option<NaiveDate>,
    legal_status: String,
    version_is_current: bool,
    affected_sections: Value,
}

async fn materialize_approved_knowledge(
    tx: &mut Transaction<'_, Postgres>,
    task: &ReviewTask,
    change: &LegalChange,
    reviewed_by: &str,
    reviewer_note: &str,
) -> AppResult<KnowledgeItem> {
    let material = sqlx::query_as::<_, KnowledgeMaterial>(
        r#"
        SELECT
          lc.id AS legal_change_id,
          dd.id AS document_diff_id,
          d.id AS document_id,
          s.id AS source_id,
          s.source_key,
          s.authority,
          s.language AS source_language,
          d.official_id,
          d.eli_id,
          d.title AS document_title,
          d.legal_area,
          d.procedure_key,
          d.source_url AS document_source_url,
          v.id AS version_id,
          v.version_label,
          v.source_url AS version_source_url,
          v.effective_date,
          v.legal_status,
          v.is_current AS version_is_current,
          lc.affected_sections
        FROM legal_changes lc
        JOIN document_diffs dd
          ON dd.id = lc.document_diff_id
         AND dd.document_id = lc.document_id
        JOIN legal_documents d ON d.id = lc.document_id
        JOIN legal_sources s ON s.id = d.source_id
        JOIN document_versions v
          ON v.id = dd.to_version_id
         AND v.document_id = d.id
        WHERE lc.id = $1
        "#,
    )
    .bind(change.id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        AppError::Conflict(format!(
            "legal change {} has incomplete document lineage",
            change.id
        ))
    })?;

    if material.source_language != "es" {
        return Err(AppError::Conflict(
            "approved knowledge requires canonical Spanish source material".to_owned(),
        ));
    }

    let procedure_key = material.procedure_key.clone().ok_or_else(|| {
        AppError::Conflict(format!(
            "document {} needs a procedure_key before approval",
            material.document_id
        ))
    })?;

    let expected_section_count = material
        .affected_sections
        .as_array()
        .ok_or_else(|| {
            AppError::Conflict(format!(
                "legal change {} has invalid affected section metadata",
                change.id
            ))
        })?
        .iter()
        .filter(|section| {
            matches!(
                section.get("change_type").and_then(Value::as_str),
                Some("added" | "modified")
            )
        })
        .count();

    // ponytail: metadata/removal-only reviews use the reviewed version's sections;
    // add explicit supersession tombstones when that workflow exists.
    let sections = sqlx::query_as::<_, DocumentSection>(
        r#"
        SELECT
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
        FROM document_sections
        WHERE version_id = $1
          AND (
            NOT $3
            OR stable_section_key IN (
              SELECT entry ->> 'stable_section_key'
              FROM jsonb_array_elements($2::jsonb) AS entry
              WHERE entry ->> 'change_type' IN ('added', 'modified')
                AND entry ->> 'new_hash' = document_sections.text_hash
            )
          )
        ORDER BY order_index ASC, stable_section_key ASC
        "#,
    )
    .bind(material.version_id)
    .bind(&material.affected_sections)
    .bind(expected_section_count > 0)
    .fetch_all(&mut **tx)
    .await?;

    if expected_section_count > 0 && sections.len() != expected_section_count {
        return Err(AppError::Conflict(format!(
            "legal change {} no longer matches its reviewed section hashes",
            change.id
        )));
    }

    let canonical_answer_es = sections
        .iter()
        .map(|section| section.text_content.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    if canonical_answer_es.is_empty() {
        return Err(AppError::Conflict(format!(
            "legal change {} has no reviewed current section text to approve",
            change.id
        )));
    }

    let section_refs = sections
        .iter()
        .map(|section| {
            json!({
                "section_id": section.id,
                "stable_section_key": section.stable_section_key,
                "section_number": section.section_number,
                "title": section.title,
                "text_hash": section.text_hash,
            })
        })
        .collect::<Vec<_>>();

    let source_refs_json = json!([{
        "legal_source_id": material.source_id,
        "source_key": material.source_key,
        "authority": material.authority,
        "language": material.source_language,
        "document_id": material.document_id,
        "official_id": material.official_id,
        "eli_id": material.eli_id,
        "document_source_url": material.document_source_url,
        "document_version_id": material.version_id,
        "version_label": material.version_label,
        "version_source_url": material.version_source_url,
        "document_diff_id": material.document_diff_id,
        "legal_change_id": material.legal_change_id,
        "legal_area": material.legal_area,
        "procedure_key": procedure_key,
        "legal_status": material.legal_status,
        "is_current_at_approval": material.version_is_current,
        "sections": section_refs,
        "review": {
            "review_task_id": task.id,
            "reviewer": reviewed_by,
            "reviewer_note": reviewer_note,
        }
    }]);

    let existing_ids = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id
        FROM knowledge_items
        WHERE review_task_id = $1
        ORDER BY created_at ASC
        FOR UPDATE
        "#,
    )
    .bind(task.id)
    .fetch_all(&mut **tx)
    .await?;

    if existing_ids.len() > 1 {
        return Err(AppError::Conflict(format!(
            "review task {} has duplicate knowledge items",
            task.id
        )));
    }

    if let Some(id) = existing_ids.first() {
        return Ok(sqlx::query_as::<_, KnowledgeItem>(
            r#"
            UPDATE knowledge_items
            SET procedure_key = $2,
                topic_key = $3,
                title_es = $4,
                canonical_answer_es = $5,
                summary_ru = '',
                summary_en = '',
                conditions_json = '[]'::jsonb,
                required_evidence_json = '[]'::jsonb,
                source_refs_json = $6,
                review_task_id = $7,
                status = 'approved',
                effective_from = $8,
                effective_until = NULL,
                approved_by = $9,
                approved_at = now(),
                updated_at = now()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&procedure_key)
        .bind(&material.legal_area)
        .bind(&material.document_title)
        .bind(&canonical_answer_es)
        .bind(&source_refs_json)
        .bind(task.id)
        .bind(material.effective_date)
        .bind(reviewed_by)
        .fetch_one(&mut **tx)
        .await?);
    }

    Ok(sqlx::query_as::<_, KnowledgeItem>(
        r#"
        INSERT INTO knowledge_items (
          procedure_key,
          topic_key,
          title_es,
          canonical_answer_es,
          source_refs_json,
          review_task_id,
          status,
          effective_from,
          approved_by,
          approved_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 'approved', $7, $8, now())
        RETURNING *
        "#,
    )
    .bind(procedure_key)
    .bind(material.legal_area)
    .bind(material.document_title)
    .bind(canonical_answer_es)
    .bind(source_refs_json)
    .bind(task.id)
    .bind(material.effective_date)
    .bind(reviewed_by)
    .fetch_one(&mut **tx)
    .await?)
}

async fn insert_review_audit(
    tx: &mut Transaction<'_, Postgres>,
    event_type: &str,
    task: &ReviewTask,
    change: &LegalChange,
    actor: &str,
    reviewer_note: &str,
    knowledge_item_id: Option<Uuid>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO audit_events (
          event_type,
          entity_type,
          entity_id,
          actor,
          details_json
        )
        VALUES ($1, 'review_task', $2, $3, $4)
        "#,
    )
    .bind(event_type)
    .bind(task.id)
    .bind(actor)
    .bind(json!({
        "previous_status": "pending",
        "new_status": task.status,
        "legal_change_id": change.id,
        "document_id": change.document_id,
        "reviewer_note": reviewer_note,
        "knowledge_item_id": knowledge_item_id,
    }))
    .execute(&mut **tx)
    .await?;
    Ok(())
}
