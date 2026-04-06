#[cfg(test)]
mod registration_security_tests {
    use crate::auth::password::*;
    use crate::encryption::field_encryption::mask_field;

    // P3-A: Registration lockdown — password validation still enforced

    #[test]
    fn register_requires_strong_password() {
        assert!(validate_password("weak").is_err());
        assert!(validate_password("NoSpecialChar1").is_err());
        assert!(validate_password("nouppercase1!x").is_err());
        assert!(validate_password("NOLOWERCASE1!X").is_err());
        assert!(validate_password("NoDigitHere!!x").is_err());
    }

    #[test]
    fn register_accepts_valid_password() {
        assert!(validate_password("SecurePass1!xy").is_ok());
        assert!(validate_password("MyP@ssw0rd!!xx").is_ok());
    }

    // P3-B: Masking behavior for device fingerprints

    #[test]
    fn device_fingerprint_masked_in_response() {
        let fp = "abcdef1234567890abcdef1234567890";
        let masked = mask_field(fp, 4, 4);
        assert!(masked.starts_with("abcd"));
        assert!(masked.ends_with("7890"));
        assert!(masked.contains("*"));
        assert_ne!(masked, fp);
    }

    #[test]
    fn short_fingerprint_fully_masked() {
        let fp = "abc";
        let masked = mask_field(fp, 4, 4);
        assert_eq!(masked, "***");
    }

    #[test]
    fn mask_preserves_length() {
        let fp = "0123456789abcdef";
        let masked = mask_field(fp, 4, 4);
        assert_eq!(masked.len(), fp.len());
    }

    // P3-B: Encryption failure detection (no plaintext fallback)

    #[test]
    fn hash_password_produces_argon2_format() {
        let hash = hash_password("TestPass1!xx").unwrap();
        assert!(hash.starts_with("$argon2"));
        assert!(hash.len() > 50);
    }

    #[test]
    fn verify_rejects_corrupted_hash() {
        // A completely invalid hash string should error
        assert!(verify_password("Original1!xx", "not-argon2-at-all").is_err());
    }

    #[test]
    fn verify_wrong_password_returns_false() {
        let hash = hash_password("Original1!xx").unwrap();
        assert!(!verify_password("Different1!xx", &hash).unwrap());
    }
}
