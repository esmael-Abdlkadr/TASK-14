use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::Engine;
use rand::RngCore;
use sqlx::PgPool;

use crate::errors::AppError;
use super::key_management::get_data_key;

/// Encrypt a sensitive field value using AES-256-GCM.
/// Returns base64-encoded ciphertext with prepended nonce.
pub async fn encrypt_field(pool: &PgPool, plaintext: &str) -> Result<String, AppError> {
    let dek = get_data_key(pool, "default").await?;
    let key = Key::<Aes256Gcm>::from_slice(&dek);
    let cipher = Aes256Gcm::new(key);

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| AppError::EncryptionError(format!("Field encryption failed: {}", e)))?;

    // Prepend nonce to ciphertext for storage
    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);

    Ok(base64::engine::general_purpose::STANDARD.encode(&combined))
}

/// Decrypt a sensitive field value.
/// Expects base64-encoded data with prepended 12-byte nonce.
pub async fn decrypt_field(pool: &PgPool, encrypted: &str) -> Result<String, AppError> {
    let dek = get_data_key(pool, "default").await?;
    let key = Key::<Aes256Gcm>::from_slice(&dek);
    let cipher = Aes256Gcm::new(key);

    let combined = base64::engine::general_purpose::STANDARD
        .decode(encrypted)
        .map_err(|e| AppError::DecryptionError(format!("Invalid base64: {}", e)))?;

    if combined.len() < 12 {
        return Err(AppError::DecryptionError("Data too short".to_string()));
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| AppError::DecryptionError(format!("Field decryption failed: {}", e)))?;

    String::from_utf8(plaintext)
        .map_err(|e| AppError::DecryptionError(format!("Invalid UTF-8: {}", e)))
}

/// Mask a sensitive field for UI display (e.g., "john@example.com" → "jo***@***.com")
pub fn mask_field(value: &str, visible_prefix: usize, visible_suffix: usize) -> String {
    let len = value.len();
    if len <= visible_prefix + visible_suffix {
        return "*".repeat(len);
    }

    let prefix: String = value.chars().take(visible_prefix).collect();
    let suffix: String = value.chars().skip(len - visible_suffix).collect();
    let masked_len = len - visible_prefix - visible_suffix;

    format!("{}{}{}", prefix, "*".repeat(masked_len), suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_field() {
        assert_eq!(mask_field("john.doe@example.com", 3, 4), "joh*************.com");
        assert_eq!(mask_field("ab", 1, 1), "**");
        assert_eq!(mask_field("abc", 1, 1), "a*c");
        assert_eq!(mask_field("secret123", 2, 2), "se*****23");
    }
}
