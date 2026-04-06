use sqlx::PgPool;
use uuid::Uuid;

use crate::db::review as review_db;
use crate::errors::AppError;
use crate::models::{
    ConsistencyCheckItem, ConsistencyCheckOutput, ConsistencyRule, ConsistencySeverity,
    ReviewScore, ScoreInput, ScorecardDimension,
};

/// Run consistency checks against a set of scores before review submission.
/// Returns flagged contradictions that must be addressed.
pub fn check_consistency(
    dimensions: &[ScorecardDimension],
    rules: &[ConsistencyRule],
    scores: &[ScoreInput],
) -> ConsistencyCheckOutput {
    let mut results = Vec::new();

    for rule in rules {
        if !rule.is_active {
            continue;
        }

        // Find scores for the two dimensions referenced by the rule
        let score_a = scores.iter().find(|s| s.dimension_id == rule.dimension_a_id);
        let score_b = scores.iter().find(|s| s.dimension_id == rule.dimension_b_id);

        let (rating_a, rating_b) = match (score_a, score_b) {
            (Some(a), Some(b)) => (a.rating, b.rating),
            _ => continue, // Can't check if either dimension is unscored
        };

        // Check: if A is in range_a, then B should be in range_b
        let a_in_range = rating_a >= rule.range_a_min && rating_a <= rule.range_a_max;
        let b_in_range = rating_b >= rule.range_b_min && rating_b <= rule.range_b_max;

        if a_in_range && !b_in_range {
            let dim_a_name = dimensions
                .iter()
                .find(|d| d.id == rule.dimension_a_id)
                .map(|d| d.name.as_str())
                .unwrap_or("Unknown");
            let dim_b_name = dimensions
                .iter()
                .find(|d| d.id == rule.dimension_b_id)
                .map(|d| d.name.as_str())
                .unwrap_or("Unknown");

            let message = format!(
                "'{}' rated {} (in range {}-{}), but '{}' rated {} (expected range {}-{}). {}",
                dim_a_name, rating_a, rule.range_a_min, rule.range_a_max,
                dim_b_name, rating_b, rule.range_b_min, rule.range_b_max,
                rule.description.as_deref().unwrap_or(&rule.name),
            );

            results.push(ConsistencyCheckItem {
                rule_name: rule.name.clone(),
                severity: rule.severity.clone(),
                message,
                dimension_a: dim_a_name.to_string(),
                dimension_b: dim_b_name.to_string(),
            });
        }
    }

    let has_errors = results.iter().any(|r| r.severity == ConsistencySeverity::Error);
    let has_warnings = results.iter().any(|r| r.severity == ConsistencySeverity::Warning);

    ConsistencyCheckOutput {
        has_errors,
        has_warnings,
        results,
    }
}

/// Validate review scores: required comments, valid ratings, all dimensions scored
pub fn validate_review_scores(
    dimensions: &[ScorecardDimension],
    scores: &[ScoreInput],
) -> Result<(), AppError> {
    // All dimensions must be scored
    for dim in dimensions {
        let score = scores.iter().find(|s| s.dimension_id == dim.id);
        match score {
            None => {
                return Err(AppError::BadRequest(format!(
                    "Missing score for dimension '{}'", dim.name
                )));
            }
            Some(s) => {
                // Validate rating is within rating_levels
                let valid_ratings: Vec<i32> = dim.rating_levels
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .filter_map(|l| l.get("value").and_then(|v| v.as_i64()))
                    .map(|v| v as i32)
                    .collect();

                if !valid_ratings.is_empty() && !valid_ratings.contains(&s.rating) {
                    return Err(AppError::BadRequest(format!(
                        "Invalid rating {} for '{}'. Valid: {:?}", s.rating, dim.name, valid_ratings
                    )));
                }

                // Check comment requirements
                if dim.comment_required && s.comment.as_ref().map_or(true, |c| c.trim().is_empty()) {
                    return Err(AppError::BadRequest(format!(
                        "Comment required for dimension '{}'", dim.name
                    )));
                }

                if let Some(threshold) = dim.comment_required_below {
                    if s.rating <= threshold && s.comment.as_ref().map_or(true, |c| c.trim().is_empty()) {
                        return Err(AppError::BadRequest(format!(
                            "Comment required for '{}' when rating is {} or below (you rated {})",
                            dim.name, threshold, s.rating
                        )));
                    }
                }
            }
        }
    }

    Ok(())
}

/// Compute the overall weighted score from dimension scores
pub fn compute_weighted_score(
    dimensions: &[ScorecardDimension],
    scores: &[ScoreInput],
) -> f32 {
    let mut weighted_sum = 0.0f32;
    let mut total_weight = 0.0f32;

    for dim in dimensions {
        if let Some(score) = scores.iter().find(|s| s.dimension_id == dim.id) {
            weighted_sum += score.rating as f32 * dim.weight;
            total_weight += dim.weight;
        }
    }

    if total_weight > 0.0 {
        weighted_sum / total_weight
    } else {
        0.0
    }
}

/// Persist consistency check results to DB
pub async fn persist_consistency_results(
    pool: &PgPool,
    review_id: Uuid,
    rules: &[ConsistencyRule],
    output: &ConsistencyCheckOutput,
) -> Result<(), AppError> {
    review_db::clear_consistency_results(pool, review_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    for item in &output.results {
        let rule = rules.iter().find(|r| r.name == item.rule_name);
        if let Some(rule) = rule {
            review_db::save_consistency_result(
                pool, review_id, rule.id, &item.severity, &item.message,
            )
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_dim(id: Uuid, name: &str, weight: f32, comment_required: bool, below: Option<i32>) -> ScorecardDimension {
        ScorecardDimension {
            id,
            scorecard_id: Uuid::new_v4(),
            name: name.to_string(),
            description: None,
            weight,
            sort_order: 0,
            rating_levels: serde_json::json!([
                {"value":1,"label":"Poor"},{"value":2,"label":"Fair"},
                {"value":3,"label":"Good"},{"value":4,"label":"Great"},{"value":5,"label":"Excellent"}
            ]),
            comment_required,
            comment_required_below: below,
            created_at: Utc::now(),
        }
    }

    fn make_rule(dim_a: Uuid, a_min: i32, a_max: i32, dim_b: Uuid, b_min: i32, b_max: i32) -> ConsistencyRule {
        ConsistencyRule {
            id: Uuid::new_v4(),
            scorecard_id: Uuid::new_v4(),
            name: "Test Rule".to_string(),
            description: Some("If A is low, B should also be low".to_string()),
            severity: ConsistencySeverity::Warning,
            dimension_a_id: dim_a,
            range_a_min: a_min,
            range_a_max: a_max,
            dimension_b_id: dim_b,
            range_b_min: b_min,
            range_b_max: b_max,
            is_active: true,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_consistency_pass() {
        let d1 = Uuid::new_v4();
        let d2 = Uuid::new_v4();
        let dims = vec![make_dim(d1, "Cleanliness", 1.0, false, None), make_dim(d2, "Safety", 1.0, false, None)];
        let rules = vec![make_rule(d1, 1, 2, d2, 1, 2)]; // if clean=low, safety should be low
        let scores = vec![
            ScoreInput { dimension_id: d1, rating: 1, comment: None },
            ScoreInput { dimension_id: d2, rating: 2, comment: None },
        ];
        let result = check_consistency(&dims, &rules, &scores);
        assert!(!result.has_errors);
        assert!(!result.has_warnings);
        assert!(result.results.is_empty());
    }

    #[test]
    fn test_consistency_violation() {
        let d1 = Uuid::new_v4();
        let d2 = Uuid::new_v4();
        let dims = vec![make_dim(d1, "Cleanliness", 1.0, false, None), make_dim(d2, "Safety", 1.0, false, None)];
        let rules = vec![make_rule(d1, 1, 2, d2, 1, 2)]; // if clean=low, safety should also be low
        let scores = vec![
            ScoreInput { dimension_id: d1, rating: 1, comment: None },
            ScoreInput { dimension_id: d2, rating: 5, comment: None }, // contradiction!
        ];
        let result = check_consistency(&dims, &rules, &scores);
        assert!(result.has_warnings);
        assert_eq!(result.results.len(), 1);
        assert!(result.results[0].message.contains("Cleanliness"));
    }

    #[test]
    fn test_weighted_score() {
        let d1 = Uuid::new_v4();
        let d2 = Uuid::new_v4();
        let dims = vec![make_dim(d1, "A", 2.0, false, None), make_dim(d2, "B", 1.0, false, None)];
        let scores = vec![
            ScoreInput { dimension_id: d1, rating: 5, comment: None },
            ScoreInput { dimension_id: d2, rating: 2, comment: None },
        ];
        let ws = compute_weighted_score(&dims, &scores);
        // (5*2 + 2*1) / (2+1) = 12/3 = 4.0
        assert!((ws - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_validate_missing_score() {
        let d1 = Uuid::new_v4();
        let dims = vec![make_dim(d1, "Test", 1.0, false, None)];
        let result = validate_review_scores(&dims, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_comment_required_below() {
        let d1 = Uuid::new_v4();
        let dims = vec![make_dim(d1, "Test", 1.0, false, Some(2))];
        let scores = vec![ScoreInput { dimension_id: d1, rating: 1, comment: None }];
        let result = validate_review_scores(&dims, &scores);
        assert!(result.is_err());

        let scores2 = vec![ScoreInput { dimension_id: d1, rating: 1, comment: Some("Reason".into()) }];
        let result2 = validate_review_scores(&dims, &scores2);
        assert!(result2.is_ok());
    }
}
