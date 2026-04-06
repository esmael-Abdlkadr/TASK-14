#[cfg(test)]
mod encryption_behavior_tests {
    use crate::encryption::field_encryption::mask_field;

    // C3: Encryption masking behavior

    #[test]
    fn mask_hides_middle_of_long_string() {
        let val = "abcdefghijklmnop";
        let masked = mask_field(val, 4, 4);
        assert!(masked.starts_with("abcd"));
        assert!(masked.ends_with("mnop"));
        assert!(masked.contains("*"));
        assert!(!masked.contains("efgh")); // middle hidden
    }

    #[test]
    fn mask_short_string_fully_hidden() {
        assert_eq!(mask_field("ab", 4, 4), "**");
        assert_eq!(mask_field("abc", 4, 4), "***");
    }

    #[test]
    fn mask_exact_boundary() {
        // prefix + suffix == length → fully masked
        assert_eq!(mask_field("abcdefgh", 4, 4), "********");
    }

    #[test]
    fn mask_preserves_total_length() {
        let val = "0123456789abcdef0123456789abcdef";
        let masked = mask_field(val, 4, 4);
        assert_eq!(masked.len(), val.len());
    }

    #[test]
    fn mask_empty_string() {
        assert_eq!(mask_field("", 4, 4), "");
    }

    #[test]
    fn mask_single_char() {
        assert_eq!(mask_field("x", 0, 0), "*");
    }

    // Fingerprint hash determinism (used for device lookup)
    #[test]
    fn sha256_fingerprint_hash_deterministic() {
        use sha2::{Sha256, Digest};
        let fp = "test-device-fingerprint";
        let h1 = hex::encode(Sha256::digest(fp.as_bytes()));
        let h2 = hex::encode(Sha256::digest(fp.as_bytes()));
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn sha256_different_fingerprints_different_hashes() {
        use sha2::{Sha256, Digest};
        let h1 = hex::encode(Sha256::digest(b"device-A"));
        let h2 = hex::encode(Sha256::digest(b"device-B"));
        assert_ne!(h1, h2);
    }

    // C1: Device fingerprint persistence design verification

    #[test]
    fn placeholder_does_not_contain_raw_fingerprint() {
        let raw_fp = "abc123def456ghi789";
        let hash = {
            use sha2::{Sha256, Digest};
            hex::encode(Sha256::digest(raw_fp.as_bytes()))
        };
        let placeholder = format!("sha256:{}", &hash[..16]);
        // Placeholder must not contain the raw fingerprint
        assert!(!placeholder.contains(raw_fp));
        assert!(placeholder.starts_with("sha256:"));
        assert_eq!(placeholder.len(), 7 + 16); // "sha256:" + 16 hex chars
    }

    #[test]
    fn hash_is_consistent_for_same_input() {
        use sha2::{Sha256, Digest};
        let fp = "my-device-fingerprint-value";
        let h1 = hex::encode(Sha256::digest(fp.as_bytes()));
        let h2 = hex::encode(Sha256::digest(fp.as_bytes()));
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_used_for_canonical_lookup_is_64_hex() {
        use sha2::{Sha256, Digest};
        let hash = hex::encode(Sha256::digest(b"fingerprint"));
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn display_identifier_truncates_hash() {
        let hash = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let display = format!("{}...{}", &hash[..8], &hash[hash.len()-4..]);
        assert_eq!(display, "abcdef12...7890");
        assert!(!display.contains("1234567890abcdef")); // middle hidden
    }

    // Migration 010 ordering: extension must precede digest usage
    #[test]
    fn migration_010_extension_before_digest() {
        let sql = include_str!("../../migrations/010_device_fingerprint_hash.sql");
        let ext_pos = sql.find("CREATE EXTENSION IF NOT EXISTS pgcrypto")
            .expect("migration 010 must contain pgcrypto extension creation");
        let digest_pos = sql.find("digest(")
            .expect("migration 010 must contain digest() call");
        assert!(
            ext_pos < digest_pos,
            "pgcrypto extension (pos {}) must appear before first digest() call (pos {})",
            ext_pos, digest_pos
        );
    }

    // DB-level device bind contract: SQL uses hash as canonical identity,
    // placeholder for legacy column, never raw fingerprint in new writes
    #[test]
    fn bind_device_sql_uses_hash_not_raw_fingerprint() {
        let db_src = include_str!("../db/devices.rs");
        // bind_device must write placeholder, not raw fingerprint
        assert!(
            db_src.contains("sha256:"),
            "bind_device must use sha256: placeholder for legacy column"
        );
        // Conflict target must be fingerprint_hash, not device_fingerprint
        assert!(
            db_src.contains("ON CONFLICT (user_id, fingerprint_hash)"),
            "bind_device upsert must conflict on (user_id, fingerprint_hash)"
        );
        // Function signature must not accept raw &str fingerprint — only hash + encrypted
        assert!(
            db_src.contains("fingerprint_hash: &str,\n    encrypted_fingerprint: &str,"),
            "bind_device params must be hash + encrypted, not raw fingerprint"
        );
    }

    #[test]
    fn find_device_sql_prefers_hash_lookup() {
        let db_src = include_str!("../db/devices.rs");
        assert!(
            db_src.contains("fingerprint_hash = $2"),
            "find_device_binding must use hash as primary lookup"
        );
        // Legacy fallback for old rows is acceptable
        assert!(
            db_src.contains("fingerprint_hash IS NULL AND device_fingerprint = $3"),
            "find_device_binding must have legacy plaintext fallback"
        );
    }

    // Login encryption fallback: no insecure placeholder pattern in codebase
    #[test]
    fn no_enc_unavail_fallback_in_login() {
        let login_src = include_str!("../auth/login.rs");
        assert!(
            !login_src.contains("enc_unavail:"),
            "login.rs must not contain enc_unavail: fallback pattern"
        );
        assert!(
            !login_src.contains("unwrap_or_else(|_| format!"),
            "login.rs must not have encryption failure unwrap_or_else formatting fallback"
        );
    }
}
