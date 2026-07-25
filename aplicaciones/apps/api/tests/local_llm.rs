use chrono::Utc;
use reqwest::StatusCode;
use serde_json::{json, Value};
use spain_helper_api::{
    config::Config,
    error::AppError,
    legal::{fixtures, repository},
    local_llm::{
        self, ClaimRequest, CompleteJobRequest, FailJobRequest, HeartbeatRequest, LocalLlmStatus,
    },
    rag::agent::{self, GenerationPlan, PreparedAnswer},
    routes,
    services::session_service,
    state::AppState,
};
use sqlx::PgPool;
use tokio::task::JoinHandle;
use uuid::Uuid;

const WORKER_TOKEN: &str = "integration-worker-token";

#[sqlx::test]
async fn worker_auth_and_server_heartbeat_freshness(pool: PgPool) -> anyhow::Result<()> {
    let (base_url, server) = spawn_api(pool.clone()).await?;
    let client = reqwest::Client::new();

    assert_eq!(
        client
            .post(format!("{base_url}/api/local-worker/heartbeat"))
            .json(&heartbeat_request("worker-a"))
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        client
            .post(format!("{base_url}/api/local-worker/heartbeat"))
            .bearer_auth("wrong-token")
            .json(&heartbeat_request("worker-a"))
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        client
            .get(format!("{base_url}/api/local-worker/check"))
            .bearer_auth("wrong-token")
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        client
            .get(format!("{base_url}/api/local-worker/check"))
            .bearer_auth(WORKER_TOKEN)
            .send()
            .await?
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        client
            .post(format!("{base_url}/api/local-llm/jobs"))
            .json(&json!({
                "message": "/help",
                "session_id": "x".repeat(201)
            }))
            .send()
            .await?
            .status(),
        StatusCode::BAD_REQUEST
    );

    let heartbeat = client
        .post(format!("{base_url}/api/local-worker/heartbeat"))
        .bearer_auth(WORKER_TOKEN)
        .json(&heartbeat_request("worker-a"))
        .send()
        .await?;
    assert_eq!(heartbeat.status(), StatusCode::OK);
    assert_eq!(heartbeat.json::<LocalLlmStatus>().await?.status, "online");

    sqlx::query(
        "UPDATE local_llm_workers SET heartbeat_at = now() - interval '41 seconds' WHERE worker_id = 'worker-a'",
    )
    .execute(&pool)
    .await?;
    let stale = client
        .get(format!("{base_url}/api/local-llm/status"))
        .send()
        .await?
        .json::<LocalLlmStatus>()
        .await?;
    assert_eq!(stale.status, "offline");

    server.abort();
    Ok(())
}

#[sqlx::test]
async fn claims_are_atomic_and_expired_leases_are_recoverable(pool: PgPool) -> anyhow::Result<()> {
    session_service::get_or_create(&pool, "lease-session").await?;
    local_llm::heartbeat(&pool, heartbeat_request("worker-a")).await?;
    local_llm::heartbeat(&pool, heartbeat_request("worker-b")).await?;

    let job_id = local_llm::create_job(
        &pool,
        "lease-session",
        "general question",
        generation_plan(),
    )
    .await?;
    let first = local_llm::claim_job(
        &pool,
        ClaimRequest {
            worker_id: "worker-a".to_owned(),
        },
    )
    .await?
    .job
    .expect("first worker should claim");
    assert_eq!(first.id, job_id);
    assert!(local_llm::claim_job(
        &pool,
        ClaimRequest {
            worker_id: "worker-b".to_owned(),
        },
    )
    .await?
    .job
    .is_none());

    sqlx::query(
        "UPDATE local_llm_jobs SET lease_expires_at = now() - interval '1 second' WHERE id = $1",
    )
    .bind(job_id)
    .execute(&pool)
    .await?;
    let recovered = local_llm::claim_job(
        &pool,
        ClaimRequest {
            worker_id: "worker-b".to_owned(),
        },
    )
    .await?
    .job
    .expect("expired lease should be reclaimable");
    assert_eq!(recovered.id, job_id);
    assert_ne!(recovered.lease_token, first.lease_token);

    let stale_complete = local_llm::complete_job(
        &pool,
        job_id,
        complete_request("worker-a", first.lease_token),
        3,
    )
    .await
    .unwrap_err();
    assert!(matches!(stale_complete, AppError::Conflict(_)));
    let stale_fail =
        local_llm::fail_job(&pool, job_id, fail_request("worker-a", first.lease_token))
            .await
            .unwrap_err();
    assert!(matches!(stale_fail, AppError::Conflict(_)));

    local_llm::fail_job(
        &pool,
        job_id,
        fail_request("worker-b", recovered.lease_token),
    )
    .await?;
    assert_eq!(
        local_llm::get_public_job(&pool, job_id, "lease-session")
            .await?
            .status,
        "failed"
    );

    let expiring_id = local_llm::create_job(
        &pool,
        "lease-session",
        "expiring question",
        generation_plan(),
    )
    .await?;
    sqlx::query("UPDATE local_llm_jobs SET expires_at = now() - interval '1 second' WHERE id = $1")
        .bind(expiring_id)
        .execute(&pool)
        .await?;
    assert_eq!(
        local_llm::get_public_job(&pool, expiring_id, "lease-session")
            .await?
            .status,
        "expired"
    );
    Ok(())
}

#[sqlx::test]
async fn completion_discards_output_when_legal_lineage_became_stale(
    pool: PgPool,
) -> anyhow::Result<()> {
    let fixture = fixtures::run_fixture_ingestion(&pool).await?;
    repository::approve_review_task(
        &pool,
        fixture.review_task.id,
        Some("Legal Reviewer"),
        Some("current source checked"),
    )
    .await?;
    session_service::get_or_create(&pool, "stale-completion-session").await?;
    local_llm::heartbeat(&pool, heartbeat_request("worker-a")).await?;

    let state = AppState::new(test_config(), pool.clone(), "test-admin-token".to_owned())?;
    let PreparedAnswer::Generate(plan) =
        agent::prepare(&state, "¿Qué documentación exige extranjería?", None).await?
    else {
        panic!("approved legal material should produce a generation plan");
    };
    let job_id = local_llm::create_job(
        &pool,
        "stale-completion-session",
        "¿Qué documentación exige extranjería?",
        plan,
    )
    .await?;
    let claimed = local_llm::claim_job(
        &pool,
        ClaimRequest {
            worker_id: "worker-a".to_owned(),
        },
    )
    .await?
    .job
    .expect("worker should claim legal job");

    sqlx::query("UPDATE document_versions SET is_current = false WHERE id = $1")
        .bind(Uuid::parse_str(&fixture.version_2_id)?)
        .execute(&pool)
        .await?;

    let mut completion = complete_request("worker-a", claimed.lease_token);
    completion.answer = "STALE_MODEL_TEXT_MUST_NOT_BE_PUBLISHED [1]".to_owned();
    let response = local_llm::complete_job(&pool, job_id, completion, 3).await?;
    assert!(!response.answer.contains("STALE_MODEL_TEXT"));
    assert!(response.answer.contains("Legal Core"));
    let legal = response.legal.expect("safe refusal keeps legal envelope");
    assert!(!legal.reviewed);
    assert!(legal.sources.is_empty());
    assert!(!legal.currentness.reviewed_version_is_current);

    let stored = local_llm::get_public_job(&pool, job_id, "stale-completion-session").await?;
    assert_eq!(stored.status, "completed");
    let stored_response = stored.response.expect("completed response");
    assert!(!stored_response["answer"]
        .as_str()
        .unwrap()
        .contains("STALE_MODEL_TEXT"));
    assert_eq!(stored_response["legal"]["reviewed"], false);
    assert!(stored_response["legal"]["sources"]
        .as_array()
        .unwrap()
        .is_empty());
    Ok(())
}

#[sqlx::test]
async fn legal_jobs_refuse_safely_and_preserve_server_citations(
    pool: PgPool,
) -> anyhow::Result<()> {
    let (base_url, server) = spawn_api(pool.clone()).await?;
    let client = reqwest::Client::new();
    let legal_question = "¿Qué documentación exige extranjería?";

    let refusal = client
        .post(format!("{base_url}/api/local-llm/jobs"))
        .json(&json!({
            "message": legal_question,
            "session_id": "refusal-session"
        }))
        .send()
        .await?;
    assert_eq!(refusal.status(), StatusCode::OK);
    let refusal: Value = refusal.json().await?;
    assert_eq!(refusal["status"], "completed");
    assert_eq!(refusal["job_id"], Value::Null);
    assert_eq!(refusal["response"]["legal"]["reviewed"], false);
    assert_eq!(job_count(&pool).await?, 0);

    let legacy_immediate = client
        .post(format!("{base_url}/api/chat"))
        .json(&json!({
            "message": legal_question,
            "session_id": "legacy-immediate-session"
        }))
        .send()
        .await?;
    assert_eq!(legacy_immediate.status(), StatusCode::OK);
    let legacy_immediate: Value = legacy_immediate.json().await?;
    assert!(legacy_immediate["answer"].is_string());
    assert!(legacy_immediate["sources"].is_array());
    assert_eq!(legacy_immediate["legal"]["reviewed"], false);
    assert!(legacy_immediate["remaining_guest_questions"].is_number());
    assert_eq!(legacy_immediate["unlocked"], false);
    assert!(legacy_immediate.get("response").is_none());
    assert!(legacy_immediate.get("status").is_none());
    assert!(legacy_immediate.get("job_id").is_none());
    assert_eq!(job_count(&pool).await?, 0);

    let fixture = fixtures::run_fixture_ingestion(&pool).await?;
    repository::approve_review_task(
        &pool,
        fixture.review_task.id,
        Some("Legal Reviewer"),
        Some("current source checked"),
    )
    .await?;

    let offline = client
        .post(format!("{base_url}/api/local-llm/jobs"))
        .json(&json!({
            "message": legal_question,
            "session_id": "offline-session"
        }))
        .send()
        .await?;
    assert_eq!(offline.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(job_count(&pool).await?, 0);

    let mut busy_heartbeat = heartbeat_request("worker-a");
    busy_heartbeat.busy = true;
    client
        .post(format!("{base_url}/api/local-worker/heartbeat"))
        .bearer_auth(WORKER_TOKEN)
        .json(&busy_heartbeat)
        .send()
        .await?
        .error_for_status()?;
    let busy = client
        .post(format!("{base_url}/api/local-llm/jobs"))
        .json(&json!({
            "message": legal_question,
            "session_id": "busy-session"
        }))
        .send()
        .await?;
    assert_eq!(busy.status(), StatusCode::CONFLICT);
    assert_eq!(job_count(&pool).await?, 0);

    client
        .post(format!("{base_url}/api/local-worker/heartbeat"))
        .bearer_auth(WORKER_TOKEN)
        .json(&heartbeat_request("worker-a"))
        .send()
        .await?
        .error_for_status()?;
    let queued = client
        .post(format!("{base_url}/api/local-llm/jobs"))
        .json(&json!({
            "message": legal_question,
            "session_id": "legal-session"
        }))
        .send()
        .await?;
    assert_eq!(queued.status(), StatusCode::ACCEPTED);
    let queued: Value = queued.json().await?;
    assert_eq!(queued["status"], "queued");
    let job_id = Uuid::parse_str(queued["job_id"].as_str().unwrap())?;

    let claimed = client
        .post(format!("{base_url}/api/local-worker/jobs/claim"))
        .bearer_auth(WORKER_TOKEN)
        .json(&json!({ "worker_id": "worker-a" }))
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;
    assert_eq!(claimed["job"]["id"], job_id.to_string());
    assert!(claimed["job"]["prompt"]
        .as_str()
        .unwrap()
        .contains("APPROVED_CURRENT_MATERIAL"));
    let lease_token = claimed["job"]["lease_token"].as_str().unwrap();

    client
        .post(format!(
            "{base_url}/api/local-worker/jobs/{job_id}/complete"
        ))
        .bearer_auth(WORKER_TOKEN)
        .json(&json!({
            "worker_id": "worker-a",
            "lease_token": lease_token,
            "answer": "Debe presentar el pasaporte completo y los documentos indicados [1].",
            "model_id": "qwen3.6-27b",
            "started_at": Utc::now(),
            "completed_at": Utc::now()
        }))
        .send()
        .await?
        .error_for_status()?;

    assert_eq!(
        client
            .get(format!(
                "{base_url}/api/local-llm/jobs/{job_id}?session_id=other-session"
            ))
            .send()
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    let completed = client
        .get(format!(
            "{base_url}/api/local-llm/jobs/{job_id}?session_id=legal-session"
        ))
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;
    assert_eq!(completed["status"], "completed");
    assert_eq!(completed["model_id"], "qwen3.6-27b");
    assert_eq!(completed["response"]["legal"]["reviewed"], true);
    assert_eq!(
        completed["response"]["legal"]["currentness"]["reviewed_version_is_current"],
        true
    );
    assert_eq!(
        completed["response"]["legal"]["sources"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    client
        .post(format!("{base_url}/api/local-worker/heartbeat"))
        .bearer_auth(WORKER_TOKEN)
        .json(&heartbeat_request("worker-a"))
        .send()
        .await?
        .error_for_status()?;
    let legacy_queued = client
        .post(format!("{base_url}/api/chat"))
        .json(&json!({
            "message": legal_question,
            "session_id": "legacy-queued-session"
        }))
        .send()
        .await?;
    assert_eq!(legacy_queued.status(), StatusCode::OK);
    let legacy_queued: Value = legacy_queued.json().await?;
    assert_eq!(legacy_queued["status"], "queued");
    assert!(legacy_queued["job_id"].is_string());
    assert!(legacy_queued["answer"]
        .as_str()
        .unwrap()
        .contains("поставлен в очередь"));
    assert!(legacy_queued["sources"].is_array());
    assert!(legacy_queued["remaining_guest_questions"].is_number());
    assert_eq!(legacy_queued["unlocked"], false);
    assert!(legacy_queued.get("response").is_none());

    server.abort();
    Ok(())
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
        local_worker_token: Some(WORKER_TOKEN.to_owned()),
        log_level: "error".to_owned(),
        cors_origins: vec!["http://localhost".to_owned()],
    }
}

fn heartbeat_request(worker_id: &str) -> HeartbeatRequest {
    HeartbeatRequest {
        worker_id: worker_id.to_owned(),
        status: "online".to_owned(),
        model_id: Some("qwen3.6-27b".to_owned()),
        model_name: Some("Qwen3.6-27B-GGUF".to_owned()),
        timestamp: Utc::now(),
        lm_studio_ready: true,
        busy: false,
        last_error: None,
    }
}

fn generation_plan() -> GenerationPlan {
    GenerationPlan {
        prompt: "test prompt".to_owned(),
        sources: vec![],
        legal: None,
    }
}

fn complete_request(worker_id: &str, lease_token: Uuid) -> CompleteJobRequest {
    CompleteJobRequest {
        worker_id: worker_id.to_owned(),
        lease_token,
        answer: "completed answer".to_owned(),
        model_id: "qwen3.6-27b".to_owned(),
        started_at: Utc::now(),
        completed_at: Utc::now(),
    }
}

fn fail_request(worker_id: &str, lease_token: Uuid) -> FailJobRequest {
    FailJobRequest {
        worker_id: worker_id.to_owned(),
        lease_token,
        error: "Local model generation failed.".to_owned(),
        model_id: Some("qwen3.6-27b".to_owned()),
        started_at: Utc::now(),
        completed_at: Utc::now(),
    }
}

async fn job_count(pool: &PgPool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT count(*) FROM local_llm_jobs")
        .fetch_one(pool)
        .await
}
