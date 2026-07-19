use crate::db::models::Article;
use sqlx::PgPool;

pub async fn list_published(pool: &PgPool) -> Result<Vec<Article>, sqlx::Error> {
    sqlx::query_as::<_, Article>(
        "SELECT * FROM articles WHERE is_published = true ORDER BY updated_at DESC",
    )
    .fetch_all(pool)
    .await
}
