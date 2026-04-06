#[cfg(test)]
mod extended_consistency_tests {
    use crate::review::consistency::*;
    use crate::models::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn dim(id: Uuid, name: &str, w: f32, req: bool, below: Option<i32>) -> ScorecardDimension {
        ScorecardDimension {
            id, scorecard_id: Uuid::new_v4(), name: name.into(), description: None,
            weight: w, sort_order: 0,
            rating_levels: serde_json::json!([
                {"value":1,"label":"Poor"},{"value":2,"label":"Fair"},
                {"value":3,"label":"Good"},{"value":4,"label":"Great"},{"value":5,"label":"Excellent"}
            ]),
            comment_required: req, comment_required_below: below, created_at: Utc::now(),
        }
    }

    fn rule(a: Uuid, amin: i32, amax: i32, b: Uuid, bmin: i32, bmax: i32, sev: ConsistencySeverity) -> ConsistencyRule {
        ConsistencyRule {
            id: Uuid::new_v4(), scorecard_id: Uuid::new_v4(),
            name: "Rule".into(), description: None, severity: sev,
            dimension_a_id: a, range_a_min: amin, range_a_max: amax,
            dimension_b_id: b, range_b_min: bmin, range_b_max: bmax,
            is_active: true, created_at: Utc::now(),
        }
    }

    fn score(dim_id: Uuid, rating: i32, comment: Option<&str>) -> ScoreInput {
        ScoreInput { dimension_id: dim_id, rating, comment: comment.map(String::from) }
    }

    // ── Weighted score computation ──────────────────────────

    #[test]
    fn weighted_score_equal_weights() {
        let d1 = Uuid::new_v4(); let d2 = Uuid::new_v4();
        let dims = vec![dim(d1, "A", 1.0, false, None), dim(d2, "B", 1.0, false, None)];
        let scores = vec![score(d1, 4, None), score(d2, 2, None)];
        let ws = compute_weighted_score(&dims, &scores);
        assert!((ws - 3.0).abs() < 0.01);
    }

    #[test]
    fn weighted_score_unequal_weights() {
        let d1 = Uuid::new_v4(); let d2 = Uuid::new_v4();
        let dims = vec![dim(d1, "A", 3.0, false, None), dim(d2, "B", 1.0, false, None)];
        let scores = vec![score(d1, 5, None), score(d2, 1, None)];
        // (5*3 + 1*1) / (3+1) = 16/4 = 4.0
        let ws = compute_weighted_score(&dims, &scores);
        assert!((ws - 4.0).abs() < 0.01);
    }

    #[test]
    fn weighted_score_no_dimensions() {
        let ws = compute_weighted_score(&[], &[]);
        assert!((ws - 0.0).abs() < 0.01);
    }

    #[test]
    fn weighted_score_single_dimension() {
        let d1 = Uuid::new_v4();
        let dims = vec![dim(d1, "A", 2.0, false, None)];
        let scores = vec![score(d1, 3, None)];
        let ws = compute_weighted_score(&dims, &scores);
        assert!((ws - 3.0).abs() < 0.01);
    }

    // ── Consistency checks ──────────────────────────────────

    #[test]
    fn no_rules_no_violations() {
        let d1 = Uuid::new_v4();
        let dims = vec![dim(d1, "A", 1.0, false, None)];
        let r = check_consistency(&dims, &[], &[score(d1, 3, None)]);
        assert!(!r.has_errors);
        assert!(!r.has_warnings);
    }

    #[test]
    fn error_severity_blocks() {
        let d1 = Uuid::new_v4(); let d2 = Uuid::new_v4();
        let dims = vec![dim(d1, "A", 1.0, false, None), dim(d2, "B", 1.0, false, None)];
        let rules = vec![rule(d1, 1, 2, d2, 1, 2, ConsistencySeverity::Error)];
        let scores = vec![score(d1, 1, None), score(d2, 5, None)];
        let r = check_consistency(&dims, &rules, &scores);
        assert!(r.has_errors);
    }

    #[test]
    fn warning_severity_does_not_block() {
        let d1 = Uuid::new_v4(); let d2 = Uuid::new_v4();
        let dims = vec![dim(d1, "A", 1.0, false, None), dim(d2, "B", 1.0, false, None)];
        let rules = vec![rule(d1, 1, 2, d2, 1, 2, ConsistencySeverity::Warning)];
        let scores = vec![score(d1, 1, None), score(d2, 5, None)];
        let r = check_consistency(&dims, &rules, &scores);
        assert!(!r.has_errors);
        assert!(r.has_warnings);
    }

    #[test]
    fn rule_not_triggered_when_a_out_of_range() {
        let d1 = Uuid::new_v4(); let d2 = Uuid::new_v4();
        let dims = vec![dim(d1, "A", 1.0, false, None), dim(d2, "B", 1.0, false, None)];
        let rules = vec![rule(d1, 1, 2, d2, 1, 2, ConsistencySeverity::Error)];
        let scores = vec![score(d1, 5, None), score(d2, 5, None)]; // d1=5, outside 1-2
        let r = check_consistency(&dims, &rules, &scores);
        assert!(!r.has_errors);
    }

    #[test]
    fn inactive_rule_skipped() {
        let d1 = Uuid::new_v4(); let d2 = Uuid::new_v4();
        let dims = vec![dim(d1, "A", 1.0, false, None), dim(d2, "B", 1.0, false, None)];
        let mut r = rule(d1, 1, 5, d2, 1, 2, ConsistencySeverity::Error);
        r.is_active = false;
        let scores = vec![score(d1, 1, None), score(d2, 5, None)];
        let result = check_consistency(&dims, &[r], &scores);
        assert!(!result.has_errors);
    }

    // ── Validate review scores ──────────────────────────────

    #[test]
    fn all_dimensions_scored_valid() {
        let d1 = Uuid::new_v4(); let d2 = Uuid::new_v4();
        let dims = vec![dim(d1, "A", 1.0, false, None), dim(d2, "B", 1.0, false, None)];
        let scores = vec![score(d1, 3, None), score(d2, 4, None)];
        assert!(validate_review_scores(&dims, &scores).is_ok());
    }

    #[test]
    fn missing_dimension_score_rejected() {
        let d1 = Uuid::new_v4(); let d2 = Uuid::new_v4();
        let dims = vec![dim(d1, "A", 1.0, false, None), dim(d2, "B", 1.0, false, None)];
        let scores = vec![score(d1, 3, None)]; // d2 missing
        assert!(validate_review_scores(&dims, &scores).is_err());
    }

    #[test]
    fn invalid_rating_value_rejected() {
        let d1 = Uuid::new_v4();
        let dims = vec![dim(d1, "A", 1.0, false, None)];
        let scores = vec![score(d1, 99, None)]; // not in 1-5
        assert!(validate_review_scores(&dims, &scores).is_err());
    }

    #[test]
    fn comment_required_missing_rejected() {
        let d1 = Uuid::new_v4();
        let dims = vec![dim(d1, "A", 1.0, true, None)]; // comment_required=true
        let scores = vec![score(d1, 3, None)]; // no comment
        assert!(validate_review_scores(&dims, &scores).is_err());
    }

    #[test]
    fn comment_required_provided_accepted() {
        let d1 = Uuid::new_v4();
        let dims = vec![dim(d1, "A", 1.0, true, None)];
        let scores = vec![score(d1, 3, Some("Looks good"))];
        assert!(validate_review_scores(&dims, &scores).is_ok());
    }

    #[test]
    fn comment_required_below_threshold_missing() {
        let d1 = Uuid::new_v4();
        let dims = vec![dim(d1, "A", 1.0, false, Some(3))]; // below 3 needs comment
        let scores = vec![score(d1, 2, None)]; // rated 2, no comment
        assert!(validate_review_scores(&dims, &scores).is_err());
    }

    #[test]
    fn comment_required_below_threshold_above_ok() {
        let d1 = Uuid::new_v4();
        let dims = vec![dim(d1, "A", 1.0, false, Some(3))];
        let scores = vec![score(d1, 4, None)]; // rated 4, no comment needed
        assert!(validate_review_scores(&dims, &scores).is_ok());
    }
}
