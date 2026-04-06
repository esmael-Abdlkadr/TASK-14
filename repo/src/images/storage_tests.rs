#[cfg(test)]
mod extended_storage_tests {
    use crate::images::storage::*;

    // ── validate_image ──────────────────────────────────────

    #[test]
    fn jpeg_valid() {
        let mut d = vec![0xFF, 0xD8, 0xFF, 0xE0];
        d.extend_from_slice(&[0u8; 200]);
        assert_eq!(validate_image(&d, "image/jpeg").unwrap(), "image/jpeg");
    }

    #[test]
    fn png_valid() {
        let mut d = vec![0x89, 0x50, 0x4E, 0x47];
        d.extend_from_slice(&[0u8; 200]);
        assert_eq!(validate_image(&d, "image/png").unwrap(), "image/png");
    }

    #[test]
    fn gif_rejected() {
        let d = vec![0x47, 0x49, 0x46, 0x38, 0x39, 0x61]; // GIF89a
        assert!(validate_image(&d, "image/gif").is_err());
    }

    #[test]
    fn bmp_rejected() {
        let d = vec![0x42, 0x4D, 0x00, 0x00, 0x00, 0x00];
        assert!(validate_image(&d, "image/bmp").is_err());
    }

    #[test]
    fn empty_file_rejected() {
        assert!(validate_image(&[], "image/jpeg").is_err());
    }

    #[test]
    fn three_bytes_rejected() {
        assert!(validate_image(&[0xFF, 0xD8, 0xFF], "image/jpeg").is_err());
    }

    #[test]
    fn exactly_5mb_accepted() {
        let mut d = vec![0xFF, 0xD8, 0xFF, 0xE0];
        d.extend_from_slice(&vec![0u8; 5 * 1024 * 1024 - 4]);
        assert!(validate_image(&d, "image/jpeg").is_ok());
    }

    #[test]
    fn over_5mb_rejected() {
        let mut d = vec![0xFF, 0xD8, 0xFF, 0xE0];
        d.extend_from_slice(&vec![0u8; 5 * 1024 * 1024 + 1]);
        assert!(validate_image(&d, "image/jpeg").is_err());
    }

    #[test]
    fn mime_mismatch_jpeg_claimed_png() {
        let d = vec![0xFF, 0xD8, 0xFF, 0xE0, 0, 0, 0, 0];
        assert!(validate_image(&d, "image/png").is_err());
    }

    #[test]
    fn mime_mismatch_png_claimed_jpeg() {
        let mut d = vec![0x89, 0x50, 0x4E, 0x47];
        d.extend_from_slice(&[0u8; 100]);
        assert!(validate_image(&d, "image/jpeg").is_err());
    }

    // ── compute_fingerprint ─────────────────────────────────

    #[test]
    fn fingerprint_deterministic() {
        let d = b"identical content";
        assert_eq!(compute_fingerprint(d), compute_fingerprint(d));
    }

    #[test]
    fn fingerprint_different_content() {
        assert_ne!(compute_fingerprint(b"aaa"), compute_fingerprint(b"bbb"));
    }

    #[test]
    fn fingerprint_is_64_hex_chars() {
        let fp = compute_fingerprint(b"test");
        assert_eq!(fp.len(), 64);
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn fingerprint_empty_input() {
        let fp = compute_fingerprint(b"");
        assert_eq!(fp.len(), 64); // SHA-256 always produces 32 bytes = 64 hex
    }

    #[test]
    fn fingerprint_single_byte_difference() {
        assert_ne!(compute_fingerprint(b"\x00"), compute_fingerprint(b"\x01"));
    }
}
