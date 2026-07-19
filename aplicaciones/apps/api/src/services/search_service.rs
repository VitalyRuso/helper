use serde::Serialize;
use sqlx::PgPool;

#[derive(Serialize)]
pub struct SearchResult {
    pub kind: String,
    pub title_ru: String,
    pub slug: String,
    pub summary_ru: String,
}

pub async fn search(pool: &PgPool, q: &str) -> Result<Vec<SearchResult>, sqlx::Error> {
    let like = format!("%{}%", q);
    let rows = sqlx::query_as::<_, (String, String, String, String)>(
        r#"
        SELECT 'article', title_ru, slug, summary_ru FROM articles
        WHERE is_published = true AND (title_ru ILIKE $1 OR summary_ru ILIKE $1 OR body_ru_markdown ILIKE $1)
        UNION ALL
        SELECT 'guide', title_ru, slug, summary_ru FROM guides
        WHERE is_published = true AND (title_ru ILIKE $1 OR summary_ru ILIKE $1)
        LIMIT 20
        "#,
    )
    .bind(like)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(kind, title_ru, slug, summary_ru)| SearchResult {
            kind,
            title_ru,
            slug,
            summary_ru,
        })
        .collect())
}
