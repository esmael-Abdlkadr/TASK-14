use base64::Engine;
use rand::Rng;
use sha2::{Sha256, Digest};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db;
use crate::errors::AppError;
use crate::models::Session;

/// Generate a cryptographically random session token
pub fn generate_session_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..64).map(|_| rng.gen()).collect();
    base64::engine::general_purpose::STANDARD.encode(&bytes)
}

/// Hash a session token for storage (we never store raw tokens)
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Create a new session for a user
pub async fn create_session(
    pool: &PgPool,
    user_id: Uuid,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) -> Result<(String, Session), AppError> {
    let token = generate_session_token();
    let token_hash = hash_token(&token);

    let session = db::sessions::create_session(pool, user_id, &token_hash, ip_address, user_agent)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok((token, session))
}

/// Validate a session token and return the session if valid
pub async fn validate_session(pool: &PgPool, token: &str) -> Result<Session, AppError> {
    let token_hash = hash_token(token);

    let session = db::sessions::find_valid_session(pool, &token_hash)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::SessionNotFound)?;

    // Check idle timeout
    let is_active = db::sessions::check_idle_timeout(pool, session.id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    if !is_active {
        return Err(AppError::SessionExpired);
    }

    // Touch session to reset idle timer
    db::sessions::touch_session(pool, session.id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(session)
}

/// Invalidate a session (logout)
pub async fn invalidate_session(pool: &PgPool, session_id: Uuid) -> Result<(), AppError> {
    db::sessions::invalidate_session(pool, session_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))
}

#[cfg(test)]
mod session_crypto_tests {
    use super::{generate_session_token, hash_token};

    #[test]
    fn generated_token_is_unique_and_non_trivial() {
        let a = generate_session_token();
        let b = generate_session_token();
        assert_ne!(a, b);
        assert!(a.len() > 40);
        assert!(a.is_ascii());
    }

    #[test]
    fn hash_token_is_deterministic_hex() {
        let h1 = hash_token("same");
        let h2 = hash_token("same");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
        assert!(h1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn hash_token_differs_for_different_inputs() {
        assert_ne!(hash_token("a"), hash_token("b"));
    }
}
