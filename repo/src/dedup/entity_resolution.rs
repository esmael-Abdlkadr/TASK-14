use sqlx::PgPool;
use uuid::Uuid;

use crate::db::bulk_data as bulk_db;
use crate::dedup::fingerprint;
use crate::errors::AppError;
use crate::models::{DuplicateFlag, DuplicateStatus, MergeConflict, MergeRequest};

/// Check a KB entry for duplicates by name, content hash, and URL.
/// Returns duplicate flags created.
pub async fn check_kb_entry_duplicates(
    pool: &PgPool,
    entry_id: Uuid,
    item_name: &str,
    disposal_instructions: &str,
    rule_source: Option<&str>,
) -> Result<Vec<DuplicateFlag>, AppError> {
    let mut flags = Vec::new();

    // 1. Exact name match
    let name_hash = fingerprint::compute_content_hash(item_name);
    let name_matches = fingerprint::fingerprint_and_check(
        pool, "kb_entry", entry_id, "content_hash", &name_hash, Some(item_name),
    ).await?;

    for m in &name_matches {
        let flag = bulk_db::create_duplicate_flag(
            pool, "kb_entry", entry_id, m.entity_id, "exact_name", 1.0,
            Some(&serde_json::json!({"matched_text": item_name})),
        ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        flags.push(flag);
    }

    // 2. Content hash (disposal instructions)
    let content_hash = fingerprint::compute_content_hash(disposal_instructions);
    let content_matches = fingerprint::fingerprint_and_check(
        pool, "kb_entry", entry_id, "content_hash", &content_hash, Some(disposal_instructions),
    ).await?;

    for m in &content_matches {
        if !flags.iter().any(|f| f.target_id == m.entity_id) {
            let flag = bulk_db::create_duplicate_flag(
                pool, "kb_entry", entry_id, m.entity_id, "near_duplicate", 0.9,
                Some(&serde_json::json!({"match_type": "content_hash"})),
            ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
            flags.push(flag);
        }
    }

    // 3. URL normalization for rule_source
    if let Some(url) = rule_source {
        let normalized = fingerprint::normalize_url(url);
        let url_hash = fingerprint::compute_content_hash(&normalized);
        let url_matches = fingerprint::fingerprint_and_check(
            pool, "kb_entry", entry_id, "normalized_url", &url_hash, Some(&normalized),
        ).await?;

        for m in &url_matches {
            if !flags.iter().any(|f| f.target_id == m.entity_id) {
                let flag = bulk_db::create_duplicate_flag(
                    pool, "kb_entry", entry_id, m.entity_id, "url_normalized", 0.8,
                    Some(&serde_json::json!({"normalized_url": &normalized})),
                ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
                flags.push(flag);
            }
        }
    }

    Ok(flags)
}

/// Check a user for duplicates by key fields (username)
pub async fn check_user_duplicates(
    pool: &PgPool,
    user_id: Uuid,
    username: &str,
) -> Result<Vec<DuplicateFlag>, AppError> {
    let key_hash = fingerprint::compute_key_fields_hash(&[("username", username)]);
    let matches = fingerprint::fingerprint_and_check(
        pool, "user", user_id, "key_fields", &key_hash, Some(username),
    ).await?;

    let mut flags = Vec::new();
    for m in &matches {
        let flag = bulk_db::create_duplicate_flag(
            pool, "user", user_id, m.entity_id, "key_fields", 1.0,
            Some(&serde_json::json!({"matched_field": "username"})),
        ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        flags.push(flag);
    }

    Ok(flags)
}

/// Detect merge conflicts between source and target entities.
/// Returns a list of fields that differ.
pub fn detect_conflicts(
    source_data: &serde_json::Value,
    target_data: &serde_json::Value,
) -> Vec<(String, serde_json::Value, serde_json::Value)> {
    let mut conflicts = Vec::new();

    if let (Some(src), Some(tgt)) = (source_data.as_object(), target_data.as_object()) {
        // Check all fields in both
        let mut all_keys: Vec<&String> = src.keys().chain(tgt.keys()).collect();
        all_keys.sort();
        all_keys.dedup();

        for key in all_keys {
            let sv = src.get(key).cloned().unwrap_or(serde_json::Value::Null);
            let tv = tgt.get(key).cloned().unwrap_or(serde_json::Value::Null);

            // Skip metadata fields
            if ["id", "created_at", "updated_at", "created_by"].contains(&key.as_str()) {
                continue;
            }

            if sv != tv {
                conflicts.push((key.clone(), sv, tv));
            }
        }
    }

    conflicts
}

/// Build provenance record tracking the origin of each merged field
pub fn build_provenance(
    conflicts: &[(String, serde_json::Value, serde_json::Value)],
    merge_conflicts: &[MergeConflict],
    source_id: Uuid,
    target_id: Uuid,
) -> serde_json::Value {
    let mut provenance = serde_json::Map::new();

    for mc in merge_conflicts {
        let origin = match mc.resolution.as_deref() {
            Some("keep_source") => source_id.to_string(),
            Some("keep_target") => target_id.to_string(),
            Some("custom") => "custom".to_string(),
            _ => "unresolved".to_string(),
        };
        provenance.insert(mc.field_name.clone(), serde_json::json!({
            "origin": origin,
            "source_value": mc.source_value,
            "target_value": mc.target_value,
            "resolution": mc.resolution,
        }));
    }

    serde_json::Value::Object(provenance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_conflicts_basic() {
        let src = serde_json::json!({"name": "Plastic Bottle", "region": "north", "id": "123"});
        let tgt = serde_json::json!({"name": "Plastic Bottle", "region": "south", "id": "456"});
        let conflicts = detect_conflicts(&src, &tgt);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].0, "region");
    }

    #[test]
    fn test_detect_conflicts_no_diff() {
        let src = serde_json::json!({"name": "Bottle", "region": "north"});
        let tgt = serde_json::json!({"name": "Bottle", "region": "north"});
        let conflicts = detect_conflicts(&src, &tgt);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_detect_conflicts_missing_fields() {
        let src = serde_json::json!({"name": "A", "extra": "x"});
        let tgt = serde_json::json!({"name": "B"});
        let conflicts = detect_conflicts(&src, &tgt);
        assert_eq!(conflicts.len(), 2); // name + extra
    }
}
