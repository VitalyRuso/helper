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

pub fn require_worker(headers: &HeaderMap, state: &AppState) -> Result<(), AppError> {
    let Some(expected) = state.config.local_worker_token.as_deref() else {
        return Err(AppError::Unauthorized);
    };
    let Some(token) = bearer(headers) else {
        return Err(AppError::Unauthorized);
    };

    if constant_time_hash_eq(token, expected) {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}

pub fn require_legal_collector(headers: &HeaderMap, state: &AppState) -> Result<(), AppError> {
    let Some(expected) = state.config.legal_collector_token.as_deref() else {
        return Err(AppError::Unauthorized);
    };
    let Some(token) = bearer(headers) else {
        return Err(AppError::Unauthorized);
    };

    if constant_time_hash_eq(token, expected) {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty())
}

pub fn constant_time_hash_eq(left: &str, right: &str) -> bool {
    let left = Sha256::digest(left.as_bytes());
    let right = Sha256::digest(right.as_bytes());
    left.iter()
        .zip(right.iter())
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[cfg(test)]
mod tests {
    use super::constant_time_hash_eq;

    #[test]
    fn hashed_token_comparison_matches_only_equal_values() {
        assert!(constant_time_hash_eq("worker-secret", "worker-secret"));
        assert!(!constant_time_hash_eq(
            "worker-secret",
            "worker-secret-wrong"
        ));
    }
}
