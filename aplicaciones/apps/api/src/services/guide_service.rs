use crate::db::models::Guide;
use sqlx::PgPool;

pub async fn list_published(pool: &PgPool) -> Result<Vec<Guide>, sqlx::Error> {
    sqlx::query_as::<_, Guide>(
        "SELECT * FROM guides WHERE is_published = true ORDER BY updated_at DESC",
    )
    .fetch_all(pool)
    .await
}
