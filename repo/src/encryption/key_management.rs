use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use rand::RngCore;
use sqlx::PgPool;

use crate::errors::AppError;

/// Master key derived from environment variable (in production, use HSM or secure vault).
/// For this offline system, the master key is loaded from configuration.
fn get_master_key() -> Result<Key<Aes256Gcm>, AppError> {
    let key_hex = std::env::var("CIVICSORT_MASTER_KEY").map_err(|_| {
        AppError::EncryptionError("CIVICSORT_MASTER_KEY environment variable not set".to_string())
    })?;

    let key_bytes = hex::decode(&key_hex).map_err(|e| {
        AppError::EncryptionError(format!("Invalid master key hex: {}", e))
    })?;

    if key_bytes.len() != 32 {
        return Err(AppError::EncryptionError(
            "Master key must be 32 bytes (64 hex chars)".to_string(),
        ));
    }

    Ok(*Key::<Aes256Gcm>::from_slice(&key_bytes))
}

/// Generate a new data encryption key (DEK), encrypt it with the master key, and store it.
pub async fn create_data_key(pool: &PgPool, key_name: &str) -> Result<(), AppError> {
    let master_key = get_master_key()?;
    let cipher = Aes256Gcm::new(&master_key);

    // Generate a random DEK
    let mut dek = [0u8; 32];
    OsRng.fill_bytes(&mut dek);

    // Generate nonce for wrapping
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt the DEK with the master key
    let encrypted_dek = cipher
        .encrypt(nonce, dek.as_ref())
        .map_err(|e| AppError::EncryptionError(format!("Failed to wrap DEK: {}", e)))?;

    sqlx::query(
        r#"
        INSERT INTO encryption_keys (key_name, encrypted_key, nonce, algorithm)
        VALUES ($1, $2, $3, 'AES-256-GCM')
        ON CONFLICT (key_name) DO NOTHING
        "#,
    )
    .bind(key_name)
    .bind(&encrypted_dek)
    .bind(&nonce_bytes.to_vec())
    .execute(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(())
}

/// Retrieve and decrypt a data encryption key
pub async fn get_data_key(pool: &PgPool, key_name: &str) -> Result<Vec<u8>, AppError> {
    let master_key = get_master_key()?;
    let cipher = Aes256Gcm::new(&master_key);

    let row: Option<(Vec<u8>, Vec<u8>)> = sqlx::query_as(
        "SELECT encrypted_key, nonce FROM encryption_keys WHERE key_name = $1 AND is_active = TRUE",
    )
    .bind(key_name)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let (encrypted_key, nonce_bytes) = row.ok_or(AppError::KeyNotFound)?;

    let nonce = Nonce::from_slice(&nonce_bytes);
    let dek = cipher
        .decrypt(nonce, encrypted_key.as_ref())
        .map_err(|e| AppError::DecryptionError(format!("Failed to unwrap DEK: {}", e)))?;

    Ok(dek)
}

/// Initialize the default data encryption key if it doesn't exist
pub async fn init_default_key(pool: &PgPool) -> Result<(), AppError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM encryption_keys WHERE key_name = 'default' AND is_active = TRUE)",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    if !exists {
        create_data_key(pool, "default").await?;
        log::info!("Default encryption key created");
    }

    Ok(())
}
