use crate::{error::AppError, state::AppState};
use axum::http::HeaderMap;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

pub fn hash_key(key: &str) -> String {
    hex::encode(Sha256::digest(key.as_bytes()))
}

pub async fn seed_access_key_hashes(pool: &PgPool, keys: &[String]) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE access_keys SET is_active = false WHERE label = 'env'")
        .execute(pool)
        .await?;
    for key in keys {
        sqlx::query(
            r#"
            INSERT INTO access_keys (key_hash, label)
            VALUES ($1, 'env')
            ON CONFLICT (key_hash) DO UPDATE SET is_active = true, label = 'env'
            "#,
        )
        .bind(hash_key(key))
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub fn require_admin(headers: &HeaderMap, state: &AppState) -> Result<(), AppError> {
    let Some(value) = headers.get("authorization").and_then(|v| v.to_str().ok()) else {
        return Err(AppError::Unauthorized);
    };
    let token = value.strip_prefix("Bearer ").unwrap_or(value);
    if token == state.admin_token.as_str() {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}
