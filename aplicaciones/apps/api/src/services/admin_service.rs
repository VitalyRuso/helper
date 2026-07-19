use serde::Serialize;
use sqlx::PgPool;

#[derive(Serialize)]
pub struct Stats {
    pub categories: i64,
    pub articles: i64,
    pub guides: i64,
    pub sessions: i64,
}

pub async fn stats(pool: &PgPool) -> Result<Stats, sqlx::Error> {
    let categories = count(pool, "categories").await?;
    let articles = count(pool, "articles").await?;
    let guides = count(pool, "guides").await?;
    let sessions = count(pool, "sessions").await?;
    Ok(Stats {
        categories,
        articles,
        guides,
        sessions,
    })
}

async fn count(pool: &PgPool, table: &str) -> Result<i64, sqlx::Error> {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    sqlx::query_scalar::<_, i64>(&sql).fetch_one(pool).await
}
