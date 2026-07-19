use sqlx::PgPool;

#[derive(Debug)]
pub struct SessionUsage {
    pub question_count: i32,
    pub has_access: bool,
}

pub async fn get_or_create(pool: &PgPool, session_id: &str) -> Result<SessionUsage, sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO sessions (session_id)
        VALUES ($1)
        ON CONFLICT (session_id) DO NOTHING
        "#,
    )
    .bind(session_id)
    .execute(pool)
    .await?;

    sqlx::query_as::<_, (i32, bool)>(
        "SELECT question_count, has_access FROM sessions WHERE session_id = $1",
    )
    .bind(session_id)
    .fetch_one(pool)
    .await
    .map(|(question_count, has_access)| SessionUsage {
        question_count,
        has_access,
    })
}

pub async fn increment_questions(pool: &PgPool, session_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE sessions SET question_count = question_count + 1, updated_at = now() WHERE session_id = $1",
    )
    .bind(session_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn unlock(pool: &PgPool, session_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO sessions (session_id, has_access)
        VALUES ($1, true)
        ON CONFLICT (session_id) DO UPDATE SET has_access = true, updated_at = now()
        "#,
    )
    .bind(session_id)
    .execute(pool)
    .await?;
    Ok(())
}
