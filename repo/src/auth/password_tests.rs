#[cfg(test)]
mod extended_password_tests {
    use crate::auth::password::*;

    // ── validate_password: normal inputs ────────────────────

    #[test]
    fn valid_password_accepted() {
        assert!(validate_password("SecurePass1!xy").is_ok());
    }

    #[test]
    fn valid_password_with_special_chars() {
        assert!(validate_password("P@ssw0rd!2026#").is_ok());
    }

    // ── validate_password: boundary inputs ──────────────────

    #[test]
    fn exactly_12_chars_valid() {
        assert!(validate_password("Abcdefgh1!23").is_ok());
    }

    #[test]
    fn exactly_11_chars_rejected() {
        assert!(validate_password("Abcdefgh1!2").is_err());
    }

    #[test]
    fn empty_password_rejected() {
        assert!(validate_password("").is_err());
    }

    #[test]
    fn single_char_rejected() {
        assert!(validate_password("A").is_err());
    }

    // ── validate_password: missing character classes ─────────

    #[test]
    fn no_uppercase_rejected() {
        let result = validate_password("abcdefghij1!");
        assert!(result.is_err());
    }

    #[test]
    fn no_lowercase_rejected() {
        let result = validate_password("ABCDEFGHIJ1!");
        assert!(result.is_err());
    }

    #[test]
    fn no_digit_rejected() {
        let result = validate_password("Abcdefghijk!");
        assert!(result.is_err());
    }

    #[test]
    fn no_special_char_rejected() {
        let result = validate_password("Abcdefghij12");
        assert!(result.is_err());
    }

    // ── hash_password / verify_password ─────────────────────

    #[test]
    fn hash_is_not_plaintext() {
        let hash = hash_password("TestPassword1!").unwrap();
        assert_ne!(hash, "TestPassword1!");
        assert!(hash.starts_with("$argon2"));
    }

    #[test]
    fn verify_correct_password() {
        let hash = hash_password("MySecure1!pw").unwrap();
        assert!(verify_password("MySecure1!pw", &hash).unwrap());
    }

    #[test]
    fn verify_wrong_password() {
        let hash = hash_password("MySecure1!pw").unwrap();
        assert!(!verify_password("WrongPassword1!", &hash).unwrap());
    }

    #[test]
    fn different_hashes_for_same_password() {
        let h1 = hash_password("SamePass1!xx").unwrap();
        let h2 = hash_password("SamePass1!xx").unwrap();
        assert_ne!(h1, h2); // different salts
    }

    #[test]
    fn verify_against_invalid_hash_format() {
        let result = verify_password("test", "not-a-valid-hash");
        assert!(result.is_err());
    }
}
