#[cfg(test)]
mod extended_fingerprint_tests {
    use crate::dedup::fingerprint::*;

    // ── Content hash ────────────────────────────────────────

    #[test]
    fn content_hash_ignores_case() {
        assert_eq!(compute_content_hash("Hello World"), compute_content_hash("hello world"));
    }

    #[test]
    fn content_hash_ignores_extra_spaces() {
        assert_eq!(compute_content_hash("a  b  c"), compute_content_hash("a b c"));
    }

    #[test]
    fn content_hash_ignores_punctuation() {
        assert_eq!(compute_content_hash("hello, world!"), compute_content_hash("hello world"));
    }

    #[test]
    fn content_hash_empty_string() {
        let h = compute_content_hash("");
        assert!(!h.is_empty());
    }

    #[test]
    fn content_hash_whitespace_only() {
        let h1 = compute_content_hash("   ");
        let h2 = compute_content_hash("");
        assert_eq!(h1, h2);
    }

    // ── Key fields hash ─────────────────────────────────────

    #[test]
    fn key_fields_order_independent() {
        let h1 = compute_key_fields_hash(&[("a", "1"), ("b", "2")]);
        let h2 = compute_key_fields_hash(&[("b", "2"), ("a", "1")]);
        assert_eq!(h1, h2);
    }

    #[test]
    fn key_fields_different_values() {
        let h1 = compute_key_fields_hash(&[("name", "Alice")]);
        let h2 = compute_key_fields_hash(&[("name", "Bob")]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn key_fields_case_insensitive() {
        let h1 = compute_key_fields_hash(&[("name", "Alice")]);
        let h2 = compute_key_fields_hash(&[("name", "alice")]);
        assert_eq!(h1, h2);
    }

    #[test]
    fn key_fields_empty() {
        let h = compute_key_fields_hash(&[]);
        assert!(!h.is_empty());
    }

    // ── URL normalization ───────────────────────────────────

    #[test]
    fn url_strips_https() {
        assert_eq!(normalize_url("https://example.com"), "example.com");
    }

    #[test]
    fn url_strips_http() {
        assert_eq!(normalize_url("http://example.com"), "example.com");
    }

    #[test]
    fn url_strips_www() {
        assert_eq!(normalize_url("https://www.example.com"), "example.com");
    }

    #[test]
    fn url_strips_trailing_slash() {
        assert_eq!(normalize_url("https://example.com/path/"), "example.com/path");
    }

    #[test]
    fn url_strips_fragment() {
        assert_eq!(normalize_url("https://example.com/page#section"), "example.com/page");
    }

    #[test]
    fn url_sorts_query_params() {
        let u1 = normalize_url("https://example.com/page?z=3&a=1&m=2");
        assert_eq!(u1, "example.com/page?a=1&m=2&z=3");
    }

    #[test]
    fn url_lowercases() {
        assert_eq!(normalize_url("HTTPS://EXAMPLE.COM/PATH"), "example.com/path");
    }

    #[test]
    fn url_empty_string() {
        assert_eq!(normalize_url(""), "");
    }

    #[test]
    fn url_no_protocol() {
        assert_eq!(normalize_url("example.com/path"), "example.com/path");
    }

    #[test]
    fn url_equivalent_after_normalization() {
        let u1 = normalize_url("https://www.Example.com/page?b=2&a=1#top");
        let u2 = normalize_url("http://example.com/page?a=1&b=2");
        assert_eq!(u1, u2);
    }
}
