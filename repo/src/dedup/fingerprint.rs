use sha2::{Sha256, Digest};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::bulk_data as bulk_db;
use crate::errors::AppError;
use crate::models::ContentFingerprint;

/// Compute a content fingerprint from text by normalizing and hashing.
/// Normalization: lowercase, collapse whitespace, strip punctuation.
pub fn compute_content_hash(text: &str) -> String {
    let normalized = normalize_text(text);
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    hex::encode(hasher.finalize())
}

/// Compute a key-fields fingerprint by hashing sorted key=value pairs.
pub fn compute_key_fields_hash(fields: &[(&str, &str)]) -> String {
    let mut sorted: Vec<_> = fields.to_vec();
    sorted.sort_by_key(|(k, _)| k.to_string());
    let combined: String = sorted
        .iter()
        .map(|(k, v)| format!("{}={}", k, normalize_text(v)))
        .collect::<Vec<_>>()
        .join("|");
    let mut hasher = Sha256::new();
    hasher.update(combined.as_bytes());
    hex::encode(hasher.finalize())
}

/// Normalize a URL for dedup comparison.
/// Strips protocol, www prefix, trailing slashes, fragments, sorts query params.
pub fn normalize_url(url: &str) -> String {
    let mut u = url.trim().to_lowercase();

    // Strip protocol
    for prefix in &["https://", "http://"] {
        if u.starts_with(prefix) {
            u = u[prefix.len()..].to_string();
        }
    }

    // Strip www.
    if u.starts_with("www.") {
        u = u[4..].to_string();
    }

    // Strip fragment
    if let Some(pos) = u.find('#') {
        u = u[..pos].to_string();
    }

    // Sort query params
    if let Some(pos) = u.find('?') {
        let (path, query) = u.split_at(pos);
        let query = &query[1..]; // strip ?
        let mut params: Vec<&str> = query.split('&').collect();
        params.sort();
        u = format!("{}?{}", path, params.join("&"));
    }

    // Strip trailing slash
    u = u.trim_end_matches('/').to_string();

    u
}

/// Normalize text for comparison: lowercase, collapse whitespace, strip non-alphanumeric
fn normalize_text(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Store a content fingerprint and return any existing matches
pub async fn fingerprint_and_check(
    pool: &PgPool,
    entity_type: &str,
    entity_id: Uuid,
    fp_type: &str,
    fingerprint: &str,
    source_text: Option<&str>,
) -> Result<Vec<ContentFingerprint>, AppError> {
    // Store this fingerprint
    bulk_db::upsert_fingerprint(pool, entity_type, entity_id, fp_type, fingerprint, source_text)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Find matches (excluding self)
    let matches = bulk_db::find_matching_fingerprints(pool, entity_type, fp_type, fingerprint)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(matches.into_iter().filter(|m| m.entity_id != entity_id).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash_deterministic() {
        let h1 = compute_content_hash("Plastic Bottle Recycling");
        let h2 = compute_content_hash("Plastic Bottle Recycling");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_content_hash_normalized() {
        let h1 = compute_content_hash("Plastic  Bottle  Recycling!");
        let h2 = compute_content_hash("plastic bottle recycling");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_content_hash_different() {
        let h1 = compute_content_hash("Plastic Bottle");
        let h2 = compute_content_hash("Glass Bottle");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_key_fields_hash() {
        let h1 = compute_key_fields_hash(&[("name", "Plastic Bottle"), ("region", "north")]);
        let h2 = compute_key_fields_hash(&[("region", "north"), ("name", "Plastic Bottle")]);
        assert_eq!(h1, h2); // order independent
    }

    #[test]
    fn test_normalize_url_basic() {
        assert_eq!(normalize_url("https://www.Example.com/path/"), "example.com/path");
        assert_eq!(normalize_url("http://example.com/path#frag"), "example.com/path");
    }

    #[test]
    fn test_normalize_url_query_sort() {
        let u1 = normalize_url("https://example.com/page?b=2&a=1");
        let u2 = normalize_url("https://example.com/page?a=1&b=2");
        assert_eq!(u1, u2);
    }

    #[test]
    fn test_normalize_text() {
        assert_eq!(normalize_text("  Hello,  World!  "), "hello world");
        assert_eq!(normalize_text("Café-Latté"), "café latté");
    }
}
