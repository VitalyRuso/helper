use crate::security::hash_key;
use sqlx::PgPool;

pub async fn is_valid(pool: &PgPool, key: &str) -> Result<bool, sqlx::Error> {
    let key_hash = hash_key(key);
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM access_keys WHERE key_hash = $1 AND is_active = true)",
    )
    .bind(key_hash)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}
