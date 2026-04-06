#[cfg(test)]
mod extended_validation_tests {
    use crate::scheduling::validation::validate_submission;
    use crate::models::{SubtaskResponseInput, TemplateSubtask, ValidationResult};
    use chrono::Utc;
    use uuid::Uuid;

    fn st(title: &str, etype: &str, required: bool, options: Option<serde_json::Value>) -> TemplateSubtask {
        TemplateSubtask {
            id: Uuid::new_v4(), template_id: Uuid::new_v4(),
            title: title.to_string(), description: None,
            sort_order: 0, is_required: required,
            expected_type: etype.to_string(), options,
            created_at: Utc::now(),
        }
    }

    fn resp(subtask_id: Uuid, val: serde_json::Value) -> SubtaskResponseInput {
        SubtaskResponseInput { subtask_id, response_value: val }
    }

    // ── Checkbox validation ─────────────────────────────────

    #[test]
    fn checkbox_true_valid() {
        let s = st("Check", "checkbox", true, None);
        let r = validate_submission(&[s.clone()], &[resp(s.id, serde_json::json!({"checked": true}))]);
        assert!(r.is_valid);
    }

    #[test]
    fn checkbox_false_valid() {
        let s = st("Check", "checkbox", true, None);
        let r = validate_submission(&[s.clone()], &[resp(s.id, serde_json::json!({"checked": false}))]);
        assert!(r.is_valid);
    }

    #[test]
    fn checkbox_missing_checked_field() {
        let s = st("Check", "checkbox", true, None);
        let r = validate_submission(&[s.clone()], &[resp(s.id, serde_json::json!({"value": true}))]);
        assert!(!r.is_valid);
    }

    // ── Text validation ─────────────────────────────────────

    #[test]
    fn text_with_content_valid() {
        let s = st("Notes", "text", true, None);
        let r = validate_submission(&[s.clone()], &[resp(s.id, serde_json::json!({"text": "All good"}))]);
        assert!(r.is_valid);
    }

    #[test]
    fn text_whitespace_only_invalid() {
        let s = st("Notes", "text", true, None);
        let r = validate_submission(&[s.clone()], &[resp(s.id, serde_json::json!({"text": "   "}))]);
        assert!(!r.is_valid);
    }

    #[test]
    fn text_optional_empty_ok() {
        let s = st("Notes", "text", false, None);
        let r = validate_submission(&[s], &[]);
        assert!(r.is_valid);
    }

    // ── Number validation ───────────────────────────────────

    #[test]
    fn number_in_range() {
        let s = st("Temp", "number", true, Some(serde_json::json!({"min": 0, "max": 100})));
        let r = validate_submission(&[s.clone()], &[resp(s.id, serde_json::json!({"number": 50}))]);
        assert!(r.is_valid);
    }

    #[test]
    fn number_at_min_boundary() {
        let s = st("Temp", "number", true, Some(serde_json::json!({"min": 0, "max": 100})));
        let r = validate_submission(&[s.clone()], &[resp(s.id, serde_json::json!({"number": 0}))]);
        assert!(r.is_valid);
    }

    #[test]
    fn number_at_max_boundary() {
        let s = st("Temp", "number", true, Some(serde_json::json!({"min": 0, "max": 100})));
        let r = validate_submission(&[s.clone()], &[resp(s.id, serde_json::json!({"number": 100}))]);
        assert!(r.is_valid);
    }

    #[test]
    fn number_below_min() {
        let s = st("Temp", "number", true, Some(serde_json::json!({"min": 0, "max": 100})));
        let r = validate_submission(&[s.clone()], &[resp(s.id, serde_json::json!({"number": -1}))]);
        assert!(!r.is_valid);
    }

    #[test]
    fn number_above_max() {
        let s = st("Temp", "number", true, Some(serde_json::json!({"min": 0, "max": 100})));
        let r = validate_submission(&[s.clone()], &[resp(s.id, serde_json::json!({"number": 101}))]);
        assert!(!r.is_valid);
    }

    #[test]
    fn number_missing_field() {
        let s = st("Temp", "number", true, None);
        let r = validate_submission(&[s.clone()], &[resp(s.id, serde_json::json!({"value": 5}))]);
        assert!(!r.is_valid);
    }

    // ── Select validation ───────────────────────────────────

    #[test]
    fn select_valid_choice() {
        let s = st("Cond", "select", true, Some(serde_json::json!({"choices": ["good", "fair", "poor"]})));
        let r = validate_submission(&[s.clone()], &[resp(s.id, serde_json::json!({"selected": "good"}))]);
        assert!(r.is_valid);
    }

    #[test]
    fn select_invalid_choice() {
        let s = st("Cond", "select", true, Some(serde_json::json!({"choices": ["good", "fair", "poor"]})));
        let r = validate_submission(&[s.clone()], &[resp(s.id, serde_json::json!({"selected": "excellent"}))]);
        assert!(!r.is_valid);
    }

    #[test]
    fn select_empty_string() {
        let s = st("Cond", "select", true, Some(serde_json::json!({"choices": ["a", "b"]})));
        let r = validate_submission(&[s.clone()], &[resp(s.id, serde_json::json!({"selected": ""}))]);
        assert!(!r.is_valid);
    }

    // ── Photo validation ────────────────────────────────────

    #[test]
    fn photo_required_missing() {
        let s = st("Photo", "photo", true, None);
        let r = validate_submission(&[s.clone()], &[resp(s.id, serde_json::json!({}))]);
        assert!(!r.is_valid);
    }

    #[test]
    fn photo_provided_valid() {
        let s = st("Photo", "photo", true, None);
        let r = validate_submission(&[s.clone()], &[resp(s.id, serde_json::json!({"photo_id": "abc123"}))]);
        assert!(r.is_valid);
    }

    // ── Multiple subtasks ───────────────────────────────────

    #[test]
    fn all_required_present() {
        let s1 = st("A", "checkbox", true, None);
        let s2 = st("B", "text", true, None);
        let resps = vec![
            resp(s1.id, serde_json::json!({"checked": true})),
            resp(s2.id, serde_json::json!({"text": "done"})),
        ];
        let r = validate_submission(&[s1, s2], &resps);
        assert!(r.is_valid);
        assert!(r.errors.is_empty());
    }

    #[test]
    fn one_required_missing_of_three() {
        let s1 = st("A", "checkbox", true, None);
        let s2 = st("B", "text", true, None);
        let s3 = st("C", "number", false, None);
        let resps = vec![
            resp(s1.id, serde_json::json!({"checked": true})),
            // s2 missing
        ];
        let r = validate_submission(&[s1, s2, s3], &resps);
        assert!(!r.is_valid);
        assert_eq!(r.errors.len(), 1);
    }

    // ── Unknown subtask warning ─────────────────────────────

    #[test]
    fn response_for_unknown_subtask_warns() {
        let s1 = st("A", "checkbox", true, None);
        let resps = vec![
            resp(s1.id, serde_json::json!({"checked": true})),
            resp(Uuid::new_v4(), serde_json::json!({"text": "orphan"})),
        ];
        let r = validate_submission(&[s1], &resps);
        assert!(r.is_valid); // warnings don't block
        assert_eq!(r.warnings.len(), 1);
    }
}
