#[cfg(test)]
mod coi_and_blind_tests {
    use crate::review::assignment::anonymize_submitter_name;
    use crate::review::consistency::*;
    use crate::models::*;
    use uuid::Uuid;
    use chrono::Utc;

    // G.8 - Blind review anonymization

    #[test]
    fn anonymize_hides_real_name() {
        let result = anonymize_submitter_name("John Doe");
        assert_eq!(result, "Anonymous Submitter");
        assert!(!result.contains("John"));
    }

    #[test]
    fn anonymize_empty_name() {
        let result = anonymize_submitter_name("");
        assert_eq!(result, "Anonymous Submitter");
    }

    // G.8 - COI rejection: reviewer cannot review self
    // (This is enforced in assignment::manual_assign where reviewer_id == submitter_id)

    // G.7 - Review consistency: contradictory ratings blocked

    fn make_dim(id: Uuid, name: &str) -> ScorecardDimension {
        ScorecardDimension {
            id, scorecard_id: Uuid::new_v4(), name: name.into(), description: None,
            weight: 1.0, sort_order: 0,
            rating_levels: serde_json::json!([
                {"value":1,"label":"Poor"},{"value":2,"label":"Fair"},
                {"value":3,"label":"Good"},{"value":4,"label":"Great"},{"value":5,"label":"Excellent"}
            ]),
            comment_required: false, comment_required_below: None, created_at: Utc::now(),
        }
    }

    fn make_error_rule(a: Uuid, b: Uuid) -> ConsistencyRule {
        ConsistencyRule {
            id: Uuid::new_v4(), scorecard_id: Uuid::new_v4(),
            name: "Error Rule".into(), description: None,
            severity: ConsistencySeverity::Error,
            dimension_a_id: a, range_a_min: 1, range_a_max: 2,
            dimension_b_id: b, range_b_min: 4, range_b_max: 5,
            is_active: true, created_at: Utc::now(),
        }
    }

    // G.7 - Error-severity consistency violation blocks submission (would return 422)
    #[test]
    fn error_consistency_violation_has_errors_flag() {
        let d1 = Uuid::new_v4(); let d2 = Uuid::new_v4();
        let dims = vec![make_dim(d1, "A"), make_dim(d2, "B")];
        let rules = vec![make_error_rule(d1, d2)]; // if A in 1-2, B must be in 4-5
        let scores = vec![
            ScoreInput { dimension_id: d1, rating: 1, comment: None },
            ScoreInput { dimension_id: d2, rating: 1, comment: None }, // violates: B should be 4-5
        ];
        let result = check_consistency(&dims, &rules, &scores);
        assert!(result.has_errors); // This would cause a 422 response
        assert!(!result.results.is_empty());
    }

    // G.7 - Warning consistency: can be acknowledged
    #[test]
    fn warning_consistency_is_acknowledgeable() {
        let d1 = Uuid::new_v4(); let d2 = Uuid::new_v4();
        let dims = vec![make_dim(d1, "A"), make_dim(d2, "B")];
        let mut rule = make_error_rule(d1, d2);
        rule.severity = ConsistencySeverity::Warning;
        let scores = vec![
            ScoreInput { dimension_id: d1, rating: 1, comment: None },
            ScoreInput { dimension_id: d2, rating: 1, comment: None },
        ];
        let result = check_consistency(&dims, &[rule], &scores);
        assert!(result.has_warnings);
        assert!(!result.has_errors); // Warnings don't block when acknowledged
    }

    // G.7 - Valid scores pass consistency
    #[test]
    fn valid_scores_pass_all_checks() {
        let d1 = Uuid::new_v4(); let d2 = Uuid::new_v4();
        let dims = vec![make_dim(d1, "A"), make_dim(d2, "B")];
        let rules = vec![make_error_rule(d1, d2)]; // if A in 1-2, B must be in 4-5
        let scores = vec![
            ScoreInput { dimension_id: d1, rating: 1, comment: None },
            ScoreInput { dimension_id: d2, rating: 5, comment: None }, // satisfies rule
        ];
        let result = check_consistency(&dims, &rules, &scores);
        assert!(!result.has_errors);
        assert!(!result.has_warnings);
    }
}
