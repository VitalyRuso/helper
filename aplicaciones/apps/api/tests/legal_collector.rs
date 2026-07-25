use reqwest::StatusCode;
use serde_json::{json, Value};
use spain_helper_api::{
    config::Config,
    error::AppError,
    legal::collector::{self, CollectorIntakeRequest},
    routes,
    state::AppState,
};
use sqlx::PgPool;
use tokio::task::JoinHandle;
use uuid::Uuid;

const COLLECTOR_TOKEN: &str = "collector-test-token";

#[sqlx::test]
async fn wrong_collector_token_is_unauthorized(pool: PgPool) -> anyhow::Result<()> {
    let (base_url, _server) = spawn_api(pool).await?;
    let response = reqwest::Client::new()
        .post(format!("{base_url}/api/legal-collector/intake"))
        .bearer_auth("wrong-token")
        .json(&sample_payload(
            "https://www.boe.es/test/legal-a",
            TEXT_V1,
            TEXT_V1,
        ))
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[sqlx::test]
async fn new_document_creates_pending_review_without_knowledge(pool: PgPool) -> anyhow::Result<()> {
    let result = collector::ingest(&pool, sample_request(TEXT_V1, TEXT_V1)?).await?;

    assert_eq!(result.status, "pending_review");
    assert!(result.review_task_id.is_some());

    let counts = counts(&pool, result.document_id).await?;
    assert_eq!(counts, (1, 1, 1, 0));
    Ok(())
}

#[sqlx::test]
async fn unchanged_content_does_not_create_duplicate_review_task(
    pool: PgPool,
) -> anyhow::Result<()> {
    let first = collector::ingest(&pool, sample_request(TEXT_V1, TEXT_V1)?).await?;
    let second = collector::ingest(&pool, sample_request(TEXT_V1, TEXT_V1)?).await?;

    assert_eq!(second.status, "unchanged");
    assert_eq!(second.document_id, first.document_id);
    assert!(second.review_task_id.is_none());
    assert_eq!(counts(&pool, first.document_id).await?, (1, 1, 1, 0));
    Ok(())
}

#[sqlx::test]
async fn section_must_be_exact_source_fragment(pool: PgPool) -> anyhow::Result<()> {
    let error = collector::ingest(&pool, sample_request(TEXT_V1, "texto inventado")?)
        .await
        .unwrap_err();

    assert!(matches!(error, AppError::BadRequest(_)));
    Ok(())
}

#[sqlx::test]
async fn unofficial_domain_is_rejected(pool: PgPool) -> anyhow::Result<()> {
    let request: CollectorIntakeRequest = serde_json::from_value(sample_payload(
        "https://example.com/legal-a",
        TEXT_V1,
        TEXT_V1,
    ))?;
    let error = collector::ingest(&pool, request).await.unwrap_err();

    assert!(matches!(error, AppError::BadRequest(_)));
    Ok(())
}

async fn counts(pool: &PgPool, document_id: Uuid) -> anyhow::Result<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as::<_, (i64, i64, i64, i64)>(
        r#"
        SELECT
          (SELECT count(*) FROM document_versions WHERE document_id = $1),
          (SELECT count(*) FROM review_tasks WHERE document_id = $1 AND status = 'pending'),
          (SELECT count(*) FROM legal_changes WHERE document_id = $1 AND status = 'pending_review'),
          (SELECT count(*) FROM knowledge_items WHERE status IN ('approved', 'published'))
        "#,
    )
    .bind(document_id)
    .fetch_one(pool)
    .await?)
}

async fn spawn_api(pool: PgPool) -> anyhow::Result<(String, JoinHandle<()>)> {
    let state = AppState::new(test_config(), pool, "test-admin-token".to_owned())?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move {
        axum::serve(listener, routes::router(state))
            .await
            .expect("test API should serve");
    });
    Ok((format!("http://{address}"), server))
}

fn sample_request(
    normalized_text: &str,
    section_text: &str,
) -> anyhow::Result<CollectorIntakeRequest> {
    Ok(serde_json::from_value(sample_payload(
        "https://www.boe.es/test/legal-a",
        normalized_text,
        section_text,
    ))?)
}

fn sample_payload(source_url: &str, normalized_text: &str, section_text: &str) -> Value {
    json!({
        "source": {
            "source_key": "test-boe",
            "title": "BOE test source",
            "authority": "BOE"
        },
        "document": {
            "official_id": "BOE-TEST-001",
            "title": "Ley de prueba",
            "document_type": "law",
            "legal_area": "immigration",
            "procedure_key": "test_procedure",
            "source_url": source_url
        },
        "version": {
            "version_label": "test-v1",
            "normalized_text": normalized_text,
            "legal_status": "effective"
        },
        "analysis": {
            "summary": "Documento oficial pendiente de revision humana.",
            "priority": "medium",
            "change_type": "modified_wording",
            "model_provider": "vertex",
            "model_name": "gemini-test",
            "model_location": "europe-west1",
            "sections": [{
                "stable_section_key": "article-1",
                "section_type": "article",
                "section_number": "1",
                "title": "Articulo 1",
                "text_content": section_text
            }]
        }
    })
}

fn test_config() -> Config {
    Config {
        app_name: "test".to_owned(),
        app_env: "test".to_owned(),
        api_host: "127.0.0.1".to_owned(),
        api_port: 0,
        database_url: "postgres://test".to_owned(),
        qdrant_url: "http://127.0.0.1:1".to_owned(),
        qdrant_collection: "test".to_owned(),
        docs_dir: "./docs".to_owned(),
        chunk_size: 1200,
        chunk_overlap: 200,
        top_k: 5,
        llm_provider: "ollama".to_owned(),
        ollama_base_url: "http://127.0.0.1:1".to_owned(),
        ollama_model: "test".to_owned(),
        openai_api_key: None,
        openai_model: "test".to_owned(),
        embedding_provider: "local-hashing".to_owned(),
        embedding_model: "local-hashing-v1".to_owned(),
        embedding_fallback: "disabled".to_owned(),
        access_keys: vec![],
        guest_question_limit: 3,
        admin_username: "admin".to_owned(),
        admin_password: "password".to_owned(),
        admin_token: Some("test-admin-token".to_owned()),
        local_worker_token: None,
        legal_collector_token: Some(COLLECTOR_TOKEN.to_owned()),
        log_level: "error".to_owned(),
        cors_origins: vec!["http://localhost".to_owned()],
    }
}

const TEXT_V1: &str = "Articulo 1. Objeto.\nTexto oficial de prueba.";
