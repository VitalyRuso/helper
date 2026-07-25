use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    legal::answer::{self as legal_answer, LegalMetadata},
    rag::agent::{self, Citation, GenerationPlan},
};

const HEARTBEAT_FRESHNESS_SECONDS: i64 = 40;
const LEASE_SECONDS: i64 = 390;
const SAFE_ERROR_LIMIT: usize = 500;
const MAX_ANSWER_CHARS: usize = 100_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub answer: String,
    pub sources: Vec<Citation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legal: Option<LegalMetadata>,
    pub remaining_guest_questions: i32,
    pub unlocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalLlmStatus {
    pub status: String,
    pub model_id: Option<String>,
    pub model_name: Option<String>,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    pub worker_id: String,
    pub status: String,
    pub model_id: Option<String>,
    pub model_name: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub lm_studio_ready: bool,
    pub busy: bool,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimRequest {
    pub worker_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimedJob {
    pub id: Uuid,
    pub lease_token: Uuid,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimResponse {
    pub job: Option<ClaimedJob>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteJobRequest {
    pub worker_id: String,
    pub lease_token: Uuid,
    pub answer: String,
    pub model_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailJobRequest {
    pub worker_id: String,
    pub lease_token: Uuid,
    pub error: String,
    pub model_id: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicJob {
    pub id: Uuid,
    pub status: String,
    pub response: Option<Value>,
    pub error: Option<String>,
    pub model_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredJobRequest {
    message: String,
    plan: GenerationPlan,
}

pub async fn heartbeat(pool: &PgPool, request: HeartbeatRequest) -> AppResult<LocalLlmStatus> {
    let worker_id = required_text(&request.worker_id, "worker_id", 200)?;
    let status = request.status.trim().to_ascii_lowercase();
    if !matches!(status.as_str(), "online" | "offline") {
        return Err(AppError::BadRequest(
            "status must be online or offline".to_owned(),
        ));
    }

    let model_id = optional_text(request.model_id.as_deref(), "model_id", 300)?;
    let model_name = optional_text(request.model_name.as_deref(), "model_name", 300)?;
    if status == "online" && (!request.lm_studio_ready || model_id.is_none()) {
        return Err(AppError::BadRequest(
            "online requires LM Studio readiness and a loaded model".to_owned(),
        ));
    }
    let last_error = request
        .last_error
        .as_deref()
        .map(safe_error)
        .filter(|value| !value.is_empty());

    sqlx::query(
        r#"
        INSERT INTO local_llm_workers (
          worker_id, reported_status, model_id, model_name, reported_at,
          heartbeat_at, lm_studio_ready, busy, last_error
        )
        VALUES ($1, $2, $3, $4, $5, now(), $6, $7, $8)
        ON CONFLICT (worker_id) DO UPDATE SET
          reported_status = EXCLUDED.reported_status,
          model_id = EXCLUDED.model_id,
          model_name = EXCLUDED.model_name,
          reported_at = EXCLUDED.reported_at,
          heartbeat_at = now(),
          lm_studio_ready = EXCLUDED.lm_studio_ready,
          busy = EXCLUDED.busy,
          last_error = EXCLUDED.last_error,
          updated_at = now()
        "#,
    )
    .bind(worker_id)
    .bind(status)
    .bind(model_id)
    .bind(model_name)
    .bind(request.timestamp)
    .bind(request.lm_studio_ready)
    .bind(request.busy)
    .bind(last_error)
    .execute(pool)
    .await?;

    get_status(pool).await
}

pub async fn get_status(pool: &PgPool) -> AppResult<LocalLlmStatus> {
    let worker = sqlx::query_as::<_, (Option<String>, Option<String>, DateTime<Utc>, bool, bool)>(
        r#"
        SELECT
          model_id,
          model_name,
          heartbeat_at,
          (
            heartbeat_at >= now() - make_interval(secs => $1)
            AND reported_status = 'online'
            AND lm_studio_ready = true
            AND NULLIF(btrim(model_id), '') IS NOT NULL
          ) AS available,
          busy
        FROM local_llm_workers
        ORDER BY heartbeat_at DESC
        LIMIT 1
        "#,
    )
    .bind(HEARTBEAT_FRESHNESS_SECONDS as f64)
    .fetch_optional(pool)
    .await?;

    let Some((model_id, model_name, heartbeat_at, available, reported_busy)) = worker else {
        return Ok(LocalLlmStatus {
            status: "offline".to_owned(),
            model_id: None,
            model_name: None,
            last_heartbeat_at: None,
        });
    };

    let active_job: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM local_llm_jobs
          WHERE
            (status = 'queued' AND expires_at > now())
            OR (
              status = 'running'
              AND (expires_at > now() OR lease_expires_at > now())
            )
        )
        "#,
    )
    .fetch_one(pool)
    .await?;

    let status = if !available {
        "offline"
    } else if reported_busy || active_job {
        "busy"
    } else {
        "online"
    };

    Ok(LocalLlmStatus {
        status: status.to_owned(),
        model_id,
        model_name,
        last_heartbeat_at: Some(heartbeat_at),
    })
}

pub async fn require_available(pool: &PgPool) -> AppResult<()> {
    match get_status(pool).await?.status.as_str() {
        "online" => Ok(()),
        "busy" => Err(AppError::Conflict("Local AI is busy".to_owned())),
        _ => Err(AppError::ServiceUnavailable(
            "Локальная нейронка не подключена".to_owned(),
        )),
    }
}

pub async fn create_job(
    pool: &PgPool,
    session_id: &str,
    message: &str,
    plan: GenerationPlan,
) -> AppResult<Uuid> {
    let payload = serde_json::to_value(StoredJobRequest {
        message: message.to_owned(),
        plan,
    })
    .map_err(anyhow_error)?;
    let mut transaction = pool.begin().await?;
    expire_jobs(&mut transaction).await?;

    let worker_busy: Option<bool> = sqlx::query_scalar(
        r#"
        SELECT busy
        FROM local_llm_workers
        WHERE heartbeat_at >= now() - make_interval(secs => $1)
          AND reported_status = 'online'
          AND lm_studio_ready = true
          AND NULLIF(btrim(model_id), '') IS NOT NULL
        ORDER BY heartbeat_at DESC
        LIMIT 1
        "#,
    )
    .bind(HEARTBEAT_FRESHNESS_SECONDS as f64)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(worker_busy) = worker_busy else {
        return Err(AppError::ServiceUnavailable(
            "Локальная нейронка не подключена".to_owned(),
        ));
    };
    if worker_busy {
        return Err(AppError::Conflict("Local AI is busy".to_owned()));
    }

    let active_job: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM local_llm_jobs WHERE status IN ('queued', 'running'))",
    )
    .fetch_one(&mut *transaction)
    .await?;
    if active_job {
        return Err(AppError::Conflict("Local AI is busy".to_owned()));
    }

    let result = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO local_llm_jobs (session_id, request_payload)
        VALUES ($1, $2)
        RETURNING id
        "#,
    )
    .bind(session_id)
    .bind(payload)
    .fetch_one(&mut *transaction)
    .await;

    let id = match result {
        Ok(id) => id,
        Err(sqlx::Error::Database(error))
            if error.constraint() == Some("local_llm_jobs_single_active_idx") =>
        {
            return Err(AppError::Conflict("Local AI is busy".to_owned()));
        }
        Err(error) => return Err(error.into()),
    };

    transaction.commit().await?;
    Ok(id)
}

pub async fn get_public_job(pool: &PgPool, id: Uuid, session_id: &str) -> AppResult<PublicJob> {
    sqlx::query(
        r#"
        UPDATE local_llm_jobs
        SET status = 'expired',
            safe_error = 'Local AI job expired before completion.',
            updated_at = now()
        WHERE id = $1
          AND (
            (status = 'queued' AND expires_at <= now())
            OR (
              status = 'running'
              AND expires_at <= now()
              AND lease_expires_at <= now()
            )
          )
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;

    sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            Option<Value>,
            Option<String>,
            Option<String>,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
            Option<DateTime<Utc>>,
        ),
    >(
        r#"
        SELECT
          id, status, response_payload, safe_error, model_id,
          created_at, started_at, completed_at
        FROM local_llm_jobs
        WHERE id = $1 AND session_id = $2
        "#,
    )
    .bind(id)
    .bind(session_id)
    .fetch_optional(pool)
    .await?
    .map(
        |(id, status, response, error, model_id, created_at, started_at, completed_at)| PublicJob {
            id,
            status,
            response,
            error,
            model_id,
            created_at,
            started_at,
            completed_at,
        },
    )
    .ok_or_else(|| AppError::NotFound("local AI job not found".to_owned()))
}

pub async fn claim_job(pool: &PgPool, request: ClaimRequest) -> AppResult<ClaimResponse> {
    let worker_id = required_text(&request.worker_id, "worker_id", 200)?;
    let mut transaction = pool.begin().await?;
    expire_jobs(&mut transaction).await?;

    let worker_ready: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM local_llm_workers
          WHERE worker_id = $1
            AND heartbeat_at >= now() - make_interval(secs => $2)
            AND reported_status = 'online'
            AND lm_studio_ready = true
            AND NULLIF(btrim(model_id), '') IS NOT NULL
        )
        "#,
    )
    .bind(&worker_id)
    .bind(HEARTBEAT_FRESHNESS_SECONDS as f64)
    .fetch_one(&mut *transaction)
    .await?;
    if !worker_ready {
        return Err(AppError::ServiceUnavailable(
            "worker is not online with a loaded model".to_owned(),
        ));
    }

    let claimed = sqlx::query_as::<_, (Uuid, Uuid, Value)>(
        r#"
        WITH candidate AS (
          SELECT id
          FROM local_llm_jobs
          WHERE expires_at > now()
            AND (
              status = 'queued'
              OR (status = 'running' AND lease_expires_at <= now())
            )
          ORDER BY created_at
          FOR UPDATE SKIP LOCKED
          LIMIT 1
        )
        UPDATE local_llm_jobs AS job
        SET status = 'running',
            lease_owner = $1,
            lease_token = gen_random_uuid(),
            lease_expires_at = now() + make_interval(secs => $2),
            attempt_count = attempt_count + 1,
            started_at = COALESCE(started_at, now()),
            updated_at = now()
        FROM candidate
        WHERE job.id = candidate.id
        RETURNING job.id, job.lease_token, job.request_payload
        "#,
    )
    .bind(&worker_id)
    .bind(LEASE_SECONDS as f64)
    .fetch_optional(&mut *transaction)
    .await?;

    let job = if let Some((id, lease_token, payload)) = claimed {
        let stored: StoredJobRequest = serde_json::from_value(payload).map_err(anyhow_error)?;
        sqlx::query(
            "UPDATE local_llm_workers SET busy = true, updated_at = now() WHERE worker_id = $1",
        )
        .bind(&worker_id)
        .execute(&mut *transaction)
        .await?;
        Some(ClaimedJob {
            id,
            lease_token,
            prompt: stored.plan.prompt,
        })
    } else {
        None
    };

    transaction.commit().await?;
    Ok(ClaimResponse { job })
}

pub async fn complete_job(
    pool: &PgPool,
    id: Uuid,
    request: CompleteJobRequest,
    guest_question_limit: i32,
) -> AppResult<ChatResponse> {
    let worker_id = required_text(&request.worker_id, "worker_id", 200)?;
    let model_id = required_text(&request.model_id, "model_id", 300)?;
    let generated = bounded_content(&request.answer, "answer", MAX_ANSWER_CHARS)?;

    let mut transaction = pool.begin().await?;
    let leased = sqlx::query_as::<_, (String, Value)>(
        r#"
        SELECT session_id, request_payload
        FROM local_llm_jobs
        WHERE id = $1
          AND status = 'running'
          AND lease_owner = $2
          AND lease_token = $3
          AND lease_expires_at > now()
        FOR UPDATE
        "#,
    )
    .bind(id)
    .bind(&worker_id)
    .bind(request.lease_token)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(invalid_lease)?;

    let stored: StoredJobRequest = serde_json::from_value(leased.1).map_err(anyhow_error)?;
    let stale_legal_language = if let Some(metadata) = stored.plan.legal.as_ref() {
        (!legal_metadata_is_still_current(&mut transaction, metadata).await?)
            .then(|| metadata.language.clone())
    } else {
        None
    };
    let answer = if let Some(language) = stale_legal_language {
        let legal = legal_answer::unavailable(&language);
        agent::AgentAnswer {
            answer: legal.answer,
            sources: vec![],
            legal: Some(legal.metadata),
        }
    } else {
        agent::finish(stored.plan, generated)?
    };
    let (question_count, has_access): (i32, bool) = sqlx::query_as(
        r#"
        UPDATE sessions
        SET question_count = question_count + 1, updated_at = now()
        WHERE session_id = $1
        RETURNING question_count, has_access
        "#,
    )
    .bind(&leased.0)
    .fetch_one(&mut *transaction)
    .await?;

    store_messages(&mut transaction, &leased.0, &stored.message, &answer).await?;
    let response = ChatResponse {
        answer: answer.answer,
        sources: answer.sources,
        legal: answer.legal,
        remaining_guest_questions: (guest_question_limit - question_count).max(0),
        unlocked: has_access,
    };
    let response_payload = serde_json::to_value(&response).map_err(anyhow_error)?;

    sqlx::query(
        r#"
        UPDATE local_llm_jobs
        SET status = 'completed',
            response_payload = $2,
            model_id = $3,
            completed_at = now(),
            worker_started_at = $4,
            worker_completed_at = $5,
            lease_expires_at = NULL,
            safe_error = NULL,
            updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(response_payload)
    .bind(model_id)
    .bind(request.started_at)
    .bind(request.completed_at)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "UPDATE local_llm_workers SET busy = false, updated_at = now() WHERE worker_id = $1",
    )
    .bind(worker_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    Ok(response)
}

pub async fn fail_job(pool: &PgPool, id: Uuid, request: FailJobRequest) -> AppResult<()> {
    let worker_id = required_text(&request.worker_id, "worker_id", 200)?;
    let error = safe_error(&request.error);
    if error.is_empty() {
        return Err(AppError::BadRequest("error is required".to_owned()));
    }
    let model_id = optional_text(request.model_id.as_deref(), "model_id", 300)?;

    let mut transaction = pool.begin().await?;
    let updated = sqlx::query_scalar::<_, String>(
        r#"
        UPDATE local_llm_jobs
        SET status = 'failed',
            model_id = $4,
            safe_error = $5,
            completed_at = now(),
            worker_started_at = $6,
            worker_completed_at = $7,
            lease_expires_at = NULL,
            updated_at = now()
        WHERE id = $1
          AND status = 'running'
          AND lease_owner = $2
          AND lease_token = $3
          AND lease_expires_at > now()
        RETURNING lease_owner
        "#,
    )
    .bind(id)
    .bind(&worker_id)
    .bind(request.lease_token)
    .bind(model_id)
    .bind(error)
    .bind(request.started_at)
    .bind(request.completed_at)
    .fetch_optional(&mut *transaction)
    .await?;
    if updated.is_none() {
        return Err(invalid_lease());
    }

    sqlx::query(
        "UPDATE local_llm_workers SET busy = false, updated_at = now() WHERE worker_id = $1",
    )
    .bind(worker_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn store_immediate_answer(
    pool: &PgPool,
    session_id: &str,
    message: &str,
    answer: agent::AgentAnswer,
    guest_question_limit: i32,
) -> AppResult<ChatResponse> {
    let mut transaction = pool.begin().await?;
    let (question_count, has_access): (i32, bool) = sqlx::query_as(
        r#"
        UPDATE sessions
        SET question_count = question_count + 1, updated_at = now()
        WHERE session_id = $1
        RETURNING question_count, has_access
        "#,
    )
    .bind(session_id)
    .fetch_one(&mut *transaction)
    .await?;

    store_messages(&mut transaction, session_id, message, &answer).await?;
    transaction.commit().await?;
    Ok(ChatResponse {
        answer: answer.answer,
        sources: answer.sources,
        legal: answer.legal,
        remaining_guest_questions: (guest_question_limit - question_count).max(0),
        unlocked: has_access,
    })
}

async fn store_messages(
    transaction: &mut Transaction<'_, Postgres>,
    session_id: &str,
    message: &str,
    answer: &agent::AgentAnswer,
) -> AppResult<()> {
    let stored_sources = answer
        .legal
        .as_ref()
        .map(|legal| serde_json::to_value(&legal.sources))
        .unwrap_or_else(|| serde_json::to_value(&answer.sources))
        .map_err(anyhow_error)?;

    sqlx::query(
        "INSERT INTO chat_messages (session_id, role, content, sources) VALUES ($1, 'user', $2, '[]')",
    )
    .bind(session_id)
    .bind(message)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "INSERT INTO chat_messages (session_id, role, content, sources) VALUES ($1, 'assistant', $2, $3)",
    )
    .bind(session_id)
    .bind(&answer.answer)
    .bind(stored_sources)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn legal_metadata_is_still_current(
    transaction: &mut Transaction<'_, Postgres>,
    metadata: &LegalMetadata,
) -> AppResult<bool> {
    if !metadata.reviewed || metadata.sources.is_empty() {
        return Ok(false);
    }

    let mut knowledge_item_ids = Vec::with_capacity(metadata.sources.len());
    let mut reviewed_version_ids = Vec::with_capacity(metadata.sources.len());
    for source in &metadata.sources {
        let Some(reviewed_version_id) = source.reviewed_version_id else {
            return Ok(false);
        };
        if knowledge_item_ids.contains(&source.knowledge_item_id) {
            return Ok(false);
        }
        knowledge_item_ids.push(source.knowledge_item_id);
        reviewed_version_ids.push(reviewed_version_id);
    }

    let valid = sqlx::query_as::<_, (Uuid, Uuid)>(
        r#"
        SELECT k.id, reviewed_version.id
        FROM unnest($1::uuid[], $2::uuid[])
          AS expected(knowledge_item_id, reviewed_version_id)
        JOIN knowledge_items k ON k.id = expected.knowledge_item_id
        JOIN review_tasks task ON task.id = k.review_task_id
        JOIN legal_changes change ON change.id = task.legal_change_id
        JOIN document_diffs diff
          ON diff.id = change.document_diff_id
         AND diff.document_id = change.document_id
        JOIN document_versions reviewed_version
          ON reviewed_version.id = expected.reviewed_version_id
         AND reviewed_version.id = diff.to_version_id
         AND reviewed_version.document_id = change.document_id
        JOIN legal_documents d ON d.id = change.document_id
        JOIN legal_sources source ON source.id = d.source_id
        WHERE k.status IN ('approved', 'published')
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
          AND reviewed_version.is_current = true
          AND NOT EXISTS (
            SELECT 1
            FROM document_versions other
            WHERE other.document_id = d.id
              AND other.is_current = true
              AND other.id <> reviewed_version.id
          )
          AND reviewed_version.legal_status IN ('effective', 'partially_effective')
          AND d.status IN ('active', 'amended')
          AND source.enabled = true
          AND source.language = 'es'
          AND (
            reviewed_version.effective_date IS NULL
            OR reviewed_version.effective_date <= current_date
          )
          AND (k.effective_from IS NULL OR k.effective_from <= current_date)
          AND (k.effective_until IS NULL OR k.effective_until >= current_date)
        FOR UPDATE OF k, task, change, diff, reviewed_version, d, source
        "#,
    )
    .bind(&knowledge_item_ids)
    .bind(&reviewed_version_ids)
    .fetch_all(&mut **transaction)
    .await?;

    Ok(valid.len() == metadata.sources.len())
}

async fn expire_jobs(transaction: &mut Transaction<'_, Postgres>) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE local_llm_jobs
        SET status = 'expired',
            safe_error = 'Local AI job expired before completion.',
            updated_at = now()
        WHERE
          (status = 'queued' AND expires_at <= now())
          OR (
            status = 'running'
            AND expires_at <= now()
            AND lease_expires_at <= now()
          )
        "#,
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn required_text(value: &str, field: &str, max_len: usize) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::BadRequest(format!("{field} is required")));
    }
    if value.chars().count() > max_len {
        return Err(AppError::BadRequest(format!("{field} is too long")));
    }
    if value.chars().any(char::is_control) {
        return Err(AppError::BadRequest(format!(
            "{field} contains invalid characters"
        )));
    }
    Ok(value.to_owned())
}

fn optional_text(value: Option<&str>, field: &str, max_len: usize) -> AppResult<Option<String>> {
    value
        .map(|value| required_text(value, field, max_len))
        .transpose()
}

fn bounded_content(value: &str, field: &str, max_len: usize) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::BadRequest(format!("{field} is required")));
    }
    if value.chars().count() > max_len {
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

fn safe_error(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .take(SAFE_ERROR_LIMIT)
        .collect::<String>()
        .trim()
        .to_owned()
}

fn invalid_lease() -> AppError {
    AppError::Conflict("job lease is not active or owned by this worker".to_owned())
}

fn anyhow_error(error: serde_json::Error) -> AppError {
    AppError::Anyhow(error.into())
}

#[cfg(test)]
mod tests {
    use super::{bounded_content, MAX_ANSWER_CHARS};

    #[test]
    fn generated_answer_is_bounded_but_allows_layout_whitespace() {
        assert!(bounded_content("first line\nsecond line", "answer", MAX_ANSWER_CHARS).is_ok());
        assert!(bounded_content(
            &"x".repeat(MAX_ANSWER_CHARS + 1),
            "answer",
            MAX_ANSWER_CHARS
        )
        .is_err());
    }
}
