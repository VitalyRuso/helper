use crate::db::models::Category;
use sqlx::PgPool;

pub async fn list(pool: &PgPool) -> Result<Vec<Category>, sqlx::Error> {
    sqlx::query_as::<_, Category>("SELECT * FROM categories ORDER BY sort_order, title_ru")
        .fetch_all(pool)
        .await
}
