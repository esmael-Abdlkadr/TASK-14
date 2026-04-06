use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use crate::errors::AppError;

const MIN_PASSWORD_LENGTH: usize = 12;

/// Validate password meets minimum requirements: at least 12 characters.
pub fn validate_password(password: &str) -> Result<(), AppError> {
    if password.len() < MIN_PASSWORD_LENGTH {
        return Err(AppError::PasswordTooShort);
    }

    let has_upper = password.chars().any(|c| c.is_uppercase());
    let has_lower = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());

    if !has_upper || !has_lower || !has_digit || !has_special {
        return Err(AppError::PasswordRequirementsNotMet(
            "Password must contain uppercase, lowercase, digit, and special character".to_string(),
        ));
    }

    Ok(())
}

/// Hash password using Argon2id
pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| AppError::InternalError(format!("Password hashing failed: {}", e)))
}

/// Verify password against stored Argon2 hash
pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, AppError> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|e| AppError::InternalError(format!("Invalid password hash format: {}", e)))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_too_short() {
        assert!(validate_password("short").is_err());
    }

    #[test]
    fn test_password_valid() {
        assert!(validate_password("SecurePass1!xy").is_ok());
    }

    #[test]
    fn test_hash_and_verify() {
        let password = "TestPassword1!";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("WrongPassword1!", &hash).unwrap());
    }
}
