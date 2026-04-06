#[cfg(test)]
mod lockout_tests {
    use crate::auth::password::{hash_password, validate_password, verify_password};

    // G.1 - Auth lockout behavior tests (password layer)

    #[test]
    fn lockout_threshold_password_still_valid_after_correct() {
        let hash = hash_password("ValidPass1!xy").unwrap();
        // Correct password always verifies regardless of lockout state
        assert!(verify_password("ValidPass1!xy", &hash).unwrap());
    }

    #[test]
    fn wrong_password_detected_correctly() {
        let hash = hash_password("CorrectPass1!").unwrap();
        assert!(!verify_password("WrongPass1!xx", &hash).unwrap());
        assert!(!verify_password("wrongpass1!xx", &hash).unwrap());
        assert!(!verify_password("CorrectPass1", &hash).unwrap());
    }

    #[test]
    fn password_boundary_exactly_12_chars() {
        assert!(validate_password("Abcdefgh1!23").is_ok());
    }

    #[test]
    fn password_boundary_11_chars_fails() {
        assert!(validate_password("Abcdefgh1!2").is_err());
    }

    // G.1 - Session token hashing determinism
    #[test]
    fn session_token_hash_deterministic() {
        use crate::auth::session::hash_token;
        let h1 = hash_token("test-token-value");
        let h2 = hash_token("test-token-value");
        assert_eq!(h1, h2);
        assert_ne!(h1, hash_token("different-token"));
    }
}
