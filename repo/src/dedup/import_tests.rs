#[cfg(test)]
mod import_validation_tests {
    use crate::dedup::fingerprint::*;
    use crate::dedup::entity_resolution::detect_conflicts;

    // G.5 - KB fuzzy ranking: content hash equivalence

    #[test]
    fn same_item_name_produces_same_hash() {
        let h1 = compute_content_hash("Plastic Bottle");
        let h2 = compute_content_hash("plastic bottle");
        assert_eq!(h1, h2); // case-insensitive match
    }

    #[test]
    fn similar_names_produce_different_hashes() {
        let h1 = compute_content_hash("Plastic Bottle");
        let h2 = compute_content_hash("Glass Bottle");
        assert_ne!(h1, h2);
    }

    #[test]
    fn url_dedup_catches_protocol_variants() {
        let u1 = normalize_url("https://example.com/rule");
        let u2 = normalize_url("http://example.com/rule");
        assert_eq!(u1, u2);
    }

    // G.4 - Bulk operation key-field matching

    #[test]
    fn user_key_fields_match_by_username() {
        let h1 = compute_key_fields_hash(&[("username", "inspector1")]);
        let h2 = compute_key_fields_hash(&[("username", "inspector1")]);
        assert_eq!(h1, h2);
    }

    #[test]
    fn user_key_fields_differ_for_different_users() {
        let h1 = compute_key_fields_hash(&[("username", "inspector1")]);
        let h2 = compute_key_fields_hash(&[("username", "inspector2")]);
        assert_ne!(h1, h2);
    }

    // G.4 - Merge conflict detection for bulk operations

    #[test]
    fn merge_detects_region_conflict() {
        let src = serde_json::json!({"item_name": "Bottle", "region": "north", "disposal": "recycle"});
        let tgt = serde_json::json!({"item_name": "Bottle", "region": "south", "disposal": "recycle"});
        let conflicts = detect_conflicts(&src, &tgt);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].0, "region");
    }

    #[test]
    fn merge_no_conflict_when_identical() {
        let src = serde_json::json!({"item_name": "Bottle", "region": "north"});
        let tgt = serde_json::json!({"item_name": "Bottle", "region": "north"});
        assert!(detect_conflicts(&src, &tgt).is_empty());
    }
}
