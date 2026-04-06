#[cfg(test)]
mod extended_resolution_tests {
    use crate::dedup::entity_resolution::detect_conflicts;

    #[test]
    fn no_conflicts_identical() {
        let a = serde_json::json!({"name": "X", "region": "north"});
        let b = serde_json::json!({"name": "X", "region": "north"});
        assert!(detect_conflicts(&a, &b).is_empty());
    }

    #[test]
    fn one_field_differs() {
        let a = serde_json::json!({"name": "X", "region": "north"});
        let b = serde_json::json!({"name": "X", "region": "south"});
        let c = detect_conflicts(&a, &b);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].0, "region");
    }

    #[test]
    fn all_fields_differ() {
        let a = serde_json::json!({"name": "A", "region": "north"});
        let b = serde_json::json!({"name": "B", "region": "south"});
        let c = detect_conflicts(&a, &b);
        assert_eq!(c.len(), 2);
    }

    #[test]
    fn metadata_fields_skipped() {
        let a = serde_json::json!({"id": "1", "created_at": "t1", "name": "X"});
        let b = serde_json::json!({"id": "2", "created_at": "t2", "name": "X"});
        let c = detect_conflicts(&a, &b);
        assert!(c.is_empty()); // id and created_at skipped
    }

    #[test]
    fn source_has_extra_field() {
        let a = serde_json::json!({"name": "X", "extra": "yes"});
        let b = serde_json::json!({"name": "X"});
        let c = detect_conflicts(&a, &b);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].0, "extra");
    }

    #[test]
    fn target_has_extra_field() {
        let a = serde_json::json!({"name": "X"});
        let b = serde_json::json!({"name": "X", "extra": "yes"});
        let c = detect_conflicts(&a, &b);
        assert_eq!(c.len(), 1);
    }

    #[test]
    fn null_vs_missing_no_conflict() {
        let a = serde_json::json!({"name": "X", "desc": null});
        let b = serde_json::json!({"name": "X"});
        let c = detect_conflicts(&a, &b);
        // absent defaults to Value::Null, so null == null → no conflict
        assert_eq!(c.len(), 0);
    }

    #[test]
    fn null_vs_value_is_conflict() {
        let a = serde_json::json!({"name": "X", "desc": null});
        let b = serde_json::json!({"name": "X", "desc": "something"});
        let c = detect_conflicts(&a, &b);
        assert_eq!(c.len(), 1);
    }

    #[test]
    fn empty_objects() {
        let a = serde_json::json!({});
        let b = serde_json::json!({});
        assert!(detect_conflicts(&a, &b).is_empty());
    }

    #[test]
    fn non_object_values() {
        let a = serde_json::json!("string");
        let b = serde_json::json!(42);
        assert!(detect_conflicts(&a, &b).is_empty()); // non-objects return empty
    }
}
