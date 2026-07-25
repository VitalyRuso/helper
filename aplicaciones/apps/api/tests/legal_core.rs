use axum::{body::to_bytes, http::StatusCode, response::IntoResponse};
use serde_json::Value;
use spain_helper_api::{
    error::AppError,
    legal::{fixtures, repository},
};
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test]
async fn fixture_ingestion_creates_complete_legal_graph(pool: PgPool) -> anyhow::Result<()> {
    let fixture = fixtures::run_fixture_ingestion(&pool).await?;
    let document_id = Uuid::parse_str(&fixture.document_id)?;

    let counts = sqlx::query_as::<_, (i64, i64, i64, i64, i64)>(
        r#"
        SELECT
          (SELECT count(*) FROM document_versions WHERE document_id = $1),
          (SELECT count(*)
             FROM document_sections section
             JOIN document_versions version ON version.id = section.version_id
            WHERE version.document_id = $1),
          (SELECT count(*) FROM document_diffs WHERE document_id = $1),
          (SELECT count(*) FROM legal_changes WHERE document_id = $1),
          (SELECT count(*) FROM review_tasks WHERE document_id = $1 AND id = $2)
        "#,
    )
    .bind(document_id)
    .bind(fixture.review_task.id)
    .fetch_one(&pool)
    .await?;

    assert_eq!(counts, (2, 5, 1, 1, 1));
    let context = repository::get_review_task_context(&pool, fixture.review_task.id).await?;
    assert_eq!(context.versions.len(), 2);
    assert_eq!(context.affected_sections.len(), 3);
    assert!(context.currentness.reviewed_version_is_current);
    assert_eq!(
        repository::list_current_document_sections(&pool, document_id)
            .await?
            .len(),
        3
    );

    repository::insert_document_version(
        &pool,
        document_id,
        "duplicate-current",
        fixtures::FIXTURE_V2,
        &fixtures::sha256_hex(fixtures::FIXTURE_V2),
        true,
    )
    .await
    .unwrap_err();
    let current_version_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM document_versions WHERE document_id = $1 AND is_current = true",
    )
    .bind(document_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(current_version_id, Uuid::parse_str(&fixture.version_2_id)?);
    Ok(())
}

#[sqlx::test]
async fn approve_records_metadata_audit_and_one_knowledge_item(pool: PgPool) -> anyhow::Result<()> {
    let fixture = fixtures::run_fixture_ingestion(&pool).await?;
    let task_id = fixture.review_task.id;
    let change_id = fixture.review_task.legal_change_id.unwrap();
    let draft_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO knowledge_items (
          procedure_key, topic_key, title_es, canonical_answer_es, review_task_id, status
        )
        VALUES ('draft', 'draft', 'Draft', 'Draft', $1, 'draft')
        RETURNING id
        "#,
    )
    .bind(task_id)
    .fetch_one(&pool)
    .await?;

    let decision = repository::approve_review_task(
        &pool,
        task_id,
        Some("Ana, Legal Reviewer"),
        Some("Reviewed against the current Spanish text."),
    )
    .await?;

    assert_eq!(decision.task.status, "approved");
    assert_eq!(
        decision.task.reviewed_by.as_deref(),
        Some("Ana, Legal Reviewer")
    );
    assert_eq!(
        decision.task.reviewer_note,
        "Reviewed against the current Spanish text."
    );
    assert!(decision.task.reviewed_at.is_some());

    let knowledge = decision.knowledge_item.unwrap();
    assert_eq!(knowledge.id, draft_id);
    assert_eq!(knowledge.review_task_id, Some(task_id));
    assert_eq!(knowledge.status, "approved");
    assert_eq!(
        knowledge.approved_by.as_deref(),
        Some("Ana, Legal Reviewer")
    );
    assert_eq!(knowledge.title_es, "Instrucción de extranjería de prueba");
    assert!(!knowledge.canonical_answer_es.is_empty());
    assert_eq!(
        knowledge.source_refs_json[0]["document_id"],
        fixture.document_id
    );
    assert_eq!(
        knowledge.source_refs_json[0]["document_version_id"],
        fixture.version_2_id
    );
    assert_eq!(
        knowledge.source_refs_json[0]["review"]["reviewer_note"],
        "Reviewed against the current Spanish text."
    );

    let change_status: String =
        sqlx::query_scalar("SELECT status FROM legal_changes WHERE id = $1")
            .bind(change_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(change_status, "approved");

    let audit = sqlx::query_as::<_, (String, String, Value)>(
        r#"
        SELECT event_type, actor, details_json
        FROM audit_events
        WHERE entity_type = 'review_task' AND entity_id = $1
        "#,
    )
    .bind(task_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(audit.0, "legal_review.approved");
    assert_eq!(audit.1, "Ana, Legal Reviewer");
    assert_eq!(audit.2["new_status"], "approved");

    let repeated = repository::approve_review_task(&pool, task_id, None, None)
        .await
        .unwrap_err();
    assert!(matches!(repeated, AppError::Conflict(_)));

    let knowledge_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM knowledge_items WHERE review_task_id = $1")
            .bind(task_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(knowledge_count, 1);
    Ok(())
}

#[sqlx::test]
async fn approve_rolls_back_when_reviewed_section_hash_changed(pool: PgPool) -> anyhow::Result<()> {
    let fixture = fixtures::run_fixture_ingestion(&pool).await?;
    let version_id = Uuid::parse_str(&fixture.version_2_id)?;

    sqlx::query(
        "UPDATE document_sections SET text_hash = 'changed-after-review' WHERE version_id = $1 AND stable_section_key = 'article-2'",
    )
    .bind(version_id)
    .execute(&pool)
    .await?;

    let error = repository::approve_review_task(&pool, fixture.review_task.id, None, None)
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Conflict(_)));

    let statuses = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT task.status, change.status
        FROM review_tasks task
        JOIN legal_changes change ON change.id = task.legal_change_id
        WHERE task.id = $1
        "#,
    )
    .bind(fixture.review_task.id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        statuses,
        ("pending".to_owned(), "pending_review".to_owned())
    );

    let writes: i64 = sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM knowledge_items WHERE review_task_id = $1) + (SELECT count(*) FROM audit_events WHERE entity_id = $1)",
    )
    .bind(fixture.review_task.id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(writes, 0);
    Ok(())
}

#[sqlx::test]
async fn removal_review_uses_current_reviewed_sections(pool: PgPool) -> anyhow::Result<()> {
    let fixture = fixtures::run_fixture_ingestion(&pool).await?;
    let change_id = fixture.review_task.legal_change_id.unwrap();
    sqlx::query(
        r#"
        UPDATE legal_changes
        SET change_type = 'removed_section',
            affected_sections = '[{"stable_section_key":"article-removed","change_type":"removed"}]'::jsonb
        WHERE id = $1
        "#,
    )
    .bind(change_id)
    .execute(&pool)
    .await?;

    let knowledge = repository::approve_review_task(&pool, fixture.review_task.id, None, None)
        .await?
        .knowledge_item
        .unwrap();
    assert!(knowledge.canonical_answer_es.contains("Artículo 1"));
    assert!(knowledge.canonical_answer_es.contains("Artículo 3"));
    Ok(())
}

#[sqlx::test]
async fn reject_records_status_and_audit_without_knowledge(pool: PgPool) -> anyhow::Result<()> {
    let fixture = fixtures::run_fixture_ingestion(&pool).await?;
    let task_id = fixture.review_task.id;
    let change_id = fixture.review_task.legal_change_id.unwrap();

    let decision = repository::reject_review_task(
        &pool,
        task_id,
        Some("Luis, Legal Reviewer"),
        Some("The change needs correction."),
    )
    .await?;

    assert_eq!(decision.task.status, "rejected");
    assert_eq!(
        decision.task.reviewed_by.as_deref(),
        Some("Luis, Legal Reviewer")
    );
    assert_eq!(decision.task.reviewer_note, "The change needs correction.");
    assert!(decision.task.reviewed_at.is_some());
    assert!(decision.knowledge_item.is_none());

    let change_status: String =
        sqlx::query_scalar("SELECT status FROM legal_changes WHERE id = $1")
            .bind(change_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(change_status, "rejected");

    let audit_event: String = sqlx::query_scalar(
        "SELECT event_type FROM audit_events WHERE entity_type = 'review_task' AND entity_id = $1",
    )
    .bind(task_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(audit_event, "legal_review.rejected");

    let knowledge_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM knowledge_items WHERE review_task_id = $1")
            .bind(task_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(knowledge_count, 0);
    Ok(())
}

#[sqlx::test]
async fn safe_retrieval_requires_approved_current_lineage(pool: PgPool) -> anyhow::Result<()> {
    let approved_fixture = fixtures::run_fixture_ingestion(&pool).await?;
    let approved = repository::approve_review_task(
        &pool,
        approved_fixture.review_task.id,
        Some("Legal Reviewer"),
        None,
    )
    .await?
    .knowledge_item
    .unwrap();

    let rejected_fixture = fixtures::run_fixture_ingestion(&pool).await?;
    repository::reject_review_task(&pool, rejected_fixture.review_task.id, None, None).await?;
    let pending_fixture = fixtures::run_fixture_ingestion(&pool).await?;

    sqlx::query(
        r#"
        INSERT INTO knowledge_items (
          procedure_key, topic_key, title_es, canonical_answer_es, review_task_id, status,
          approved_by, approved_at
        )
        VALUES
          ('rejected-fixture', 'immigration', 'Rejected', 'Must stay private', $1, 'approved', 'Legal Reviewer', now()),
          ('pending-fixture', 'immigration', 'Pending', 'Must stay private', $2, 'approved', 'Legal Reviewer', now())
        "#,
    )
    .bind(rejected_fixture.review_task.id)
    .bind(pending_fixture.review_task.id)
    .execute(&pool)
    .await?;

    let safe = repository::list_approved_knowledge(&pool).await?;
    assert_eq!(safe.len(), 1);
    assert_eq!(safe[0].item.id, approved.id);
    assert!(safe[0].reviewed_version_is_current);
    assert!(!safe[0].is_stale);
    assert_eq!(
        repository::search_approved_knowledge(&pool, "pasaporte")
            .await?
            .len(),
        1
    );
    assert!(repository::search_approved_knowledge(&pool, "private")
        .await?
        .is_empty());

    let reviewed_version_id = Uuid::parse_str(&approved_fixture.version_2_id)?;
    sqlx::query("UPDATE document_versions SET is_current = false WHERE id = $1")
        .bind(reviewed_version_id)
        .execute(&pool)
        .await?;

    assert!(repository::list_approved_knowledge(&pool).await?.is_empty());

    let admin = repository::list_knowledge_items(&pool).await?;
    let stale = admin
        .iter()
        .find(|view| view.item.id == approved.id)
        .unwrap();
    assert!(stale.is_stale);
    assert!(!stale.reviewed_version_is_current);
    assert_eq!(stale.reviewed_version_id, Some(reviewed_version_id));
    assert_eq!(stale.current_version_id, None);
    Ok(())
}

#[sqlx::test]
async fn missing_review_task_is_clean_not_found_response(pool: PgPool) -> anyhow::Result<()> {
    let task_id = Uuid::new_v4();
    let error = repository::approve_review_task(&pool, task_id, None, None)
        .await
        .unwrap_err();

    assert!(matches!(&error, AppError::NotFound(_)));
    let response = error.into_response();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body: Value = serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await?)?;
    assert_eq!(body["error"], format!("review task {task_id} not found"));
    assert!(!body["error"].as_str().unwrap().contains("no rows returned"));
    Ok(())
}
