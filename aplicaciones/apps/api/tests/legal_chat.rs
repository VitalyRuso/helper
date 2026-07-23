use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use spain_helper_api::{
    error::AppError,
    legal::{
        answer::{self, QuestionRoute},
        fixtures, repository,
    },
};
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test]
async fn legal_answers_use_only_matching_approved_current_material(
    pool: PgPool,
) -> anyhow::Result<()> {
    let pending = fixtures::run_fixture_ingestion(&pool).await?;
    let QuestionRoute::Legal(route) =
        answer::route_question("¿Qué documentación exige extranjería?")
    else {
        panic!("expected legal route");
    };

    let generated = Arc::new(AtomicBool::new(false));
    let generated_in_closure = Arc::clone(&generated);
    let unavailable = answer::answer(
        &pool,
        "¿Qué documentación exige extranjería?",
        &route,
        move |_| {
            let generated = Arc::clone(&generated_in_closure);
            async move {
                generated.store(true, Ordering::SeqCst);
                Ok::<_, AppError>("must not be generated".to_owned())
            }
        },
    )
    .await?;

    assert!(!generated.load(Ordering::SeqCst));
    assert!(!unavailable.metadata.reviewed);
    assert!(unavailable.metadata.sources.is_empty());
    assert_eq!(unavailable.metadata.currentness.status, "unavailable");
    assert!(unavailable.answer.contains("Legal Core"));

    repository::reject_review_task(&pool, pending.review_task.id, None, None).await?;
    let unavailable_after_rejection = answer::answer(
        &pool,
        "¿Qué documentación exige extranjería?",
        &route,
        |_| async { Ok::<_, AppError>("must not be generated".to_owned()) },
    )
    .await?;
    assert!(!unavailable_after_rejection.metadata.reviewed);

    let approved_fixture = fixtures::run_fixture_ingestion(&pool).await?;
    let approved = repository::approve_review_task(
        &pool,
        approved_fixture.review_task.id,
        Some("Legal Reviewer"),
        Some("Current Spanish source reviewed."),
    )
    .await?
    .knowledge_item
    .expect("approval should materialize knowledge");

    let captured_prompt = Arc::new(Mutex::new(String::new()));
    let prompt_in_closure = Arc::clone(&captured_prompt);
    let legal_answer = answer::answer(
        &pool,
        "¿Qué documentación exige extranjería?",
        &route,
        move |prompt| {
            let captured_prompt = Arc::clone(&prompt_in_closure);
            async move {
                *captured_prompt.lock().expect("prompt lock") = prompt;
                Ok::<_, AppError>(
                    "El Artículo 2 exige pasaporte completo, certificado de empadronamiento y justificante de medios económicos [1]."
                        .to_owned(),
                )
            }
        },
    )
    .await?;

    assert!(legal_answer.metadata.reviewed);
    assert!(
        legal_answer
            .metadata
            .currentness
            .reviewed_version_is_current
    );
    assert_eq!(
        legal_answer.metadata.currentness.status,
        "current_lineage_check_date_unknown"
    );
    assert_eq!(legal_answer.metadata.sources.len(), 1);
    assert_eq!(
        legal_answer.metadata.sources[0].knowledge_item_id,
        approved.id
    );
    assert_eq!(
        legal_answer.metadata.sources[0].official_id.as_deref(),
        Some("FIXTURE-EXTRANJERIA-001")
    );
    assert_eq!(
        legal_answer.metadata.sources[0].version_label.as_deref(),
        Some("fixture-v2")
    );
    assert_eq!(legal_answer.metadata.reviewer_role, "Legal Reviewer");
    assert!(legal_answer.answer.contains("Artículo 2"));
    assert!(legal_answer
        .answer
        .contains("no constituye asesoramiento jurídico"));

    let prompt = captured_prompt.lock().expect("prompt lock");
    assert!(prompt.contains("APPROVED_CURRENT_MATERIAL"));
    assert!(prompt.contains(&approved.canonical_answer_es));
    assert!(!prompt.contains("page_context"));
    drop(prompt);

    let reviewed_version_id = Uuid::parse_str(&approved_fixture.version_2_id)?;
    sqlx::query("UPDATE document_versions SET is_current = false WHERE id = $1")
        .bind(reviewed_version_id)
        .execute(&pool)
        .await?;

    let generated = Arc::new(AtomicBool::new(false));
    let generated_in_closure = Arc::clone(&generated);
    let stale = answer::answer(
        &pool,
        "¿Qué documentación exige extranjería?",
        &route,
        move |_| {
            let generated = Arc::clone(&generated_in_closure);
            async move {
                generated.store(true, Ordering::SeqCst);
                Ok::<_, AppError>("must not be generated".to_owned())
            }
        },
    )
    .await?;

    assert!(!generated.load(Ordering::SeqCst));
    assert!(!stale.metadata.reviewed);
    assert!(stale.metadata.sources.is_empty());
    assert_eq!(stale.metadata.currentness.status, "unavailable");
    Ok(())
}
