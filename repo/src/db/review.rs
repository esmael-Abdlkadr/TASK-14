use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    AssignmentMethod, ConflictOfInterest, ConsistencyCheckResult, ConsistencyRule,
    ConsistencySeverity, CreateConsistencyRuleInput, CreateDimensionInput, Review,
    ReviewAssignment, ReviewAssignmentStatus, ReviewScore, ReviewStatus, ReviewTargetType,
    ReviewerDepartment, Scorecard, ScorecardDimension,
};

// ── Scorecards ──────────────────────────────────────────────

pub async fn create_scorecard(
    pool: &PgPool,
    name: &str,
    description: Option<&str>,
    target_type: &ReviewTargetType,
    passing_score: Option<f32>,
    created_by: Option<Uuid>,
) -> Result<Scorecard, sqlx::Error> {
    sqlx::query_as::<_, Scorecard>(
        r#"INSERT INTO scorecards (name, description, target_type, passing_score, created_by)
        VALUES ($1, $2, $3, $4, $5) RETURNING *"#,
    )
    .bind(name).bind(description).bind(target_type).bind(passing_score).bind(created_by)
    .fetch_one(pool).await
}

pub async fn get_scorecard(pool: &PgPool, id: Uuid) -> Result<Option<Scorecard>, sqlx::Error> {
    sqlx::query_as::<_, Scorecard>("SELECT * FROM scorecards WHERE id = $1")
        .bind(id).fetch_optional(pool).await
}

pub async fn list_scorecards(pool: &PgPool, target_type: Option<&ReviewTargetType>) -> Result<Vec<Scorecard>, sqlx::Error> {
    match target_type {
        Some(tt) => sqlx::query_as::<_, Scorecard>(
            "SELECT * FROM scorecards WHERE is_active = TRUE AND target_type = $1 ORDER BY name",
        ).bind(tt).fetch_all(pool).await,
        None => sqlx::query_as::<_, Scorecard>(
            "SELECT * FROM scorecards WHERE is_active = TRUE ORDER BY name",
        ).fetch_all(pool).await,
    }
}

pub async fn update_scorecard(
    pool: &PgPool, id: Uuid, name: &str, description: Option<&str>, passing_score: Option<f32>,
) -> Result<Scorecard, sqlx::Error> {
    sqlx::query_as::<_, Scorecard>(
        "UPDATE scorecards SET name=$2, description=$3, passing_score=$4, updated_at=NOW() WHERE id=$1 RETURNING *",
    ).bind(id).bind(name).bind(description).bind(passing_score).fetch_one(pool).await
}

pub async fn deactivate_scorecard(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE scorecards SET is_active=FALSE, updated_at=NOW() WHERE id=$1")
        .bind(id).execute(pool).await?;
    Ok(())
}

// ── Dimensions ──────────────────────────────────────────────

pub async fn create_dimension(
    pool: &PgPool, scorecard_id: Uuid, name: &str, description: Option<&str>,
    weight: f32, sort_order: i32, rating_levels: Option<&serde_json::Value>,
    comment_required: bool, comment_required_below: Option<i32>,
) -> Result<ScorecardDimension, sqlx::Error> {
    let default_levels = serde_json::json!([
        {"value": 1, "label": "Poor"}, {"value": 2, "label": "Below Average"},
        {"value": 3, "label": "Average"}, {"value": 4, "label": "Good"},
        {"value": 5, "label": "Excellent"}
    ]);
    let levels = rating_levels.unwrap_or(&default_levels);

    sqlx::query_as::<_, ScorecardDimension>(
        r#"INSERT INTO scorecard_dimensions
        (scorecard_id, name, description, weight, sort_order, rating_levels, comment_required, comment_required_below)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING *"#,
    )
    .bind(scorecard_id).bind(name).bind(description).bind(weight).bind(sort_order)
    .bind(levels).bind(comment_required).bind(comment_required_below)
    .fetch_one(pool).await
}

pub async fn get_dimensions(pool: &PgPool, scorecard_id: Uuid) -> Result<Vec<ScorecardDimension>, sqlx::Error> {
    sqlx::query_as::<_, ScorecardDimension>(
        "SELECT * FROM scorecard_dimensions WHERE scorecard_id=$1 ORDER BY sort_order",
    ).bind(scorecard_id).fetch_all(pool).await
}

pub async fn set_dimensions(
    pool: &PgPool, scorecard_id: Uuid, dims: &[CreateDimensionInput],
) -> Result<Vec<ScorecardDimension>, sqlx::Error> {
    sqlx::query("DELETE FROM scorecard_dimensions WHERE scorecard_id=$1")
        .bind(scorecard_id).execute(pool).await?;
    let mut result = Vec::new();
    for (i, d) in dims.iter().enumerate() {
        let dim = create_dimension(
            pool, scorecard_id, &d.name, d.description.as_deref(),
            d.weight.unwrap_or(1.0), d.sort_order.unwrap_or(i as i32),
            d.rating_levels.as_ref(), d.comment_required.unwrap_or(false),
            d.comment_required_below,
        ).await?;
        result.push(dim);
    }
    Ok(result)
}

// ── Consistency Rules ───────────────────────────────────────

pub async fn create_consistency_rule(
    pool: &PgPool, scorecard_id: Uuid, input: &CreateConsistencyRuleInput,
) -> Result<ConsistencyRule, sqlx::Error> {
    let severity = input.severity.as_ref().unwrap_or(&ConsistencySeverity::Warning);
    sqlx::query_as::<_, ConsistencyRule>(
        r#"INSERT INTO consistency_rules
        (scorecard_id, name, description, severity, dimension_a_id, range_a_min, range_a_max, dimension_b_id, range_b_min, range_b_max)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING *"#,
    )
    .bind(scorecard_id).bind(&input.name).bind(input.description.as_deref()).bind(severity)
    .bind(input.dimension_a_id).bind(input.range_a_min).bind(input.range_a_max)
    .bind(input.dimension_b_id).bind(input.range_b_min).bind(input.range_b_max)
    .fetch_one(pool).await
}

pub async fn get_consistency_rules(pool: &PgPool, scorecard_id: Uuid) -> Result<Vec<ConsistencyRule>, sqlx::Error> {
    sqlx::query_as::<_, ConsistencyRule>(
        "SELECT * FROM consistency_rules WHERE scorecard_id=$1 AND is_active=TRUE",
    ).bind(scorecard_id).fetch_all(pool).await
}

// ── Assignments ─────────────────────────────────────────────

pub async fn create_assignment(
    pool: &PgPool, reviewer_id: Uuid, target_type: &ReviewTargetType, target_id: Uuid,
    scorecard_id: Uuid, method: &AssignmentMethod, is_blind: bool,
    assigned_by: Option<Uuid>, due_date: Option<NaiveDate>,
) -> Result<ReviewAssignment, sqlx::Error> {
    sqlx::query_as::<_, ReviewAssignment>(
        r#"INSERT INTO review_assignments
        (reviewer_id, target_type, target_id, scorecard_id, method, is_blind, assigned_by, due_date)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING *"#,
    )
    .bind(reviewer_id).bind(target_type).bind(target_id).bind(scorecard_id)
    .bind(method).bind(is_blind).bind(assigned_by).bind(due_date)
    .fetch_one(pool).await
}

pub async fn get_assignment(pool: &PgPool, id: Uuid) -> Result<Option<ReviewAssignment>, sqlx::Error> {
    sqlx::query_as::<_, ReviewAssignment>("SELECT * FROM review_assignments WHERE id=$1")
        .bind(id).fetch_optional(pool).await
}

pub async fn update_assignment_status(
    pool: &PgPool, id: Uuid, status: &ReviewAssignmentStatus,
) -> Result<ReviewAssignment, sqlx::Error> {
    sqlx::query_as::<_, ReviewAssignment>(
        r#"UPDATE review_assignments SET status=$2,
        completed_at = CASE WHEN $2='completed'::review_assignment_status THEN NOW() ELSE completed_at END
        WHERE id=$1 RETURNING *"#,
    ).bind(id).bind(status).fetch_one(pool).await
}

pub async fn recuse_assignment(
    pool: &PgPool, id: Uuid, reason: &str,
) -> Result<ReviewAssignment, sqlx::Error> {
    sqlx::query_as::<_, ReviewAssignment>(
        "UPDATE review_assignments SET status='recused'::review_assignment_status, recused_at=NOW(), recusal_reason=$2 WHERE id=$1 RETURNING *",
    ).bind(id).bind(reason).fetch_one(pool).await
}

pub async fn list_assignments_for_reviewer(
    pool: &PgPool, reviewer_id: Uuid, status: Option<&ReviewAssignmentStatus>,
    limit: i64, offset: i64,
) -> Result<Vec<ReviewAssignment>, sqlx::Error> {
    match status {
        Some(s) => sqlx::query_as::<_, ReviewAssignment>(
            "SELECT * FROM review_assignments WHERE reviewer_id=$1 AND status=$2 ORDER BY assigned_at DESC LIMIT $3 OFFSET $4",
        ).bind(reviewer_id).bind(s).bind(limit).bind(offset).fetch_all(pool).await,
        None => sqlx::query_as::<_, ReviewAssignment>(
            "SELECT * FROM review_assignments WHERE reviewer_id=$1 ORDER BY assigned_at DESC LIMIT $2 OFFSET $3",
        ).bind(reviewer_id).bind(limit).bind(offset).fetch_all(pool).await,
    }
}

pub async fn count_assignments_for_reviewer(
    pool: &PgPool, reviewer_id: Uuid, status: Option<&ReviewAssignmentStatus>,
) -> Result<i64, sqlx::Error> {
    match status {
        Some(s) => sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM review_assignments WHERE reviewer_id=$1 AND status=$2",
        ).bind(reviewer_id).bind(s).fetch_one(pool).await,
        None => sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM review_assignments WHERE reviewer_id=$1",
        ).bind(reviewer_id).fetch_one(pool).await,
    }
}

/// Get all eligible reviewers (role=Reviewer, not already assigned, no COI)
pub async fn get_eligible_reviewers(
    pool: &PgPool, target_type: &ReviewTargetType, target_id: Uuid, submitter_id: Uuid,
) -> Result<Vec<crate::models::User>, sqlx::Error> {
    sqlx::query_as::<_, crate::models::User>(
        r#"
        SELECT u.* FROM users u
        WHERE u.role = 'reviewer'::user_role
          AND u.status = 'active'::account_status
          AND u.id != $3
          -- Not already assigned to this target
          AND u.id NOT IN (
              SELECT ra.reviewer_id FROM review_assignments ra
              WHERE ra.target_type = $1 AND ra.target_id = $2
                AND ra.status NOT IN ('recused'::review_assignment_status, 'reassigned'::review_assignment_status)
          )
          -- No COI with submitter
          AND u.id NOT IN (
              SELECT coi.reviewer_id FROM conflict_of_interest coi
              WHERE coi.is_active = TRUE AND coi.target_user_id = $3
          )
          -- No department COI
          AND u.id NOT IN (
              SELECT coi.reviewer_id FROM conflict_of_interest coi
              JOIN reviewer_departments rd ON rd.user_id = $3
              WHERE coi.is_active = TRUE
                AND coi.conflict_type = 'department'
                AND coi.department = rd.department
          )
        ORDER BY (
            SELECT COUNT(*) FROM review_assignments ra2
            WHERE ra2.reviewer_id = u.id AND ra2.status IN ('pending'::review_assignment_status, 'in_progress'::review_assignment_status)
        ) ASC
        "#,
    ).bind(target_type).bind(target_id).bind(submitter_id).fetch_all(pool).await
}

// ── Reviews ─────────────────────────────────────────────────

pub async fn create_review(
    pool: &PgPool, assignment_id: Uuid, reviewer_id: Uuid, scorecard_id: Uuid,
    target_type: &ReviewTargetType, target_id: Uuid,
) -> Result<Review, sqlx::Error> {
    sqlx::query_as::<_, Review>(
        r#"INSERT INTO reviews (assignment_id, reviewer_id, scorecard_id, target_type, target_id)
        VALUES ($1,$2,$3,$4,$5) RETURNING *"#,
    )
    .bind(assignment_id).bind(reviewer_id).bind(scorecard_id)
    .bind(target_type).bind(target_id)
    .fetch_one(pool).await
}

pub async fn get_review(pool: &PgPool, id: Uuid) -> Result<Option<Review>, sqlx::Error> {
    sqlx::query_as::<_, Review>("SELECT * FROM reviews WHERE id=$1")
        .bind(id).fetch_optional(pool).await
}

pub async fn get_review_by_assignment(pool: &PgPool, assignment_id: Uuid) -> Result<Option<Review>, sqlx::Error> {
    sqlx::query_as::<_, Review>("SELECT * FROM reviews WHERE assignment_id=$1 ORDER BY created_at DESC LIMIT 1")
        .bind(assignment_id).fetch_optional(pool).await
}

pub async fn submit_review(
    pool: &PgPool, id: Uuid, overall_score: f32, overall_comment: Option<&str>,
    recommendation: &str,
) -> Result<Review, sqlx::Error> {
    sqlx::query_as::<_, Review>(
        r#"UPDATE reviews SET status='submitted'::review_status,
        overall_score=$2, overall_comment=$3, recommendation=$4,
        submitted_at=NOW(), updated_at=NOW()
        WHERE id=$1 RETURNING *"#,
    ).bind(id).bind(overall_score).bind(overall_comment).bind(recommendation).fetch_one(pool).await
}

pub async fn finalize_review(pool: &PgPool, id: Uuid) -> Result<Review, sqlx::Error> {
    sqlx::query_as::<_, Review>(
        "UPDATE reviews SET status='finalized'::review_status, finalized_at=NOW(), updated_at=NOW() WHERE id=$1 RETURNING *",
    ).bind(id).fetch_one(pool).await
}

// ── Review Scores ───────────────────────────────────────────

pub async fn upsert_score(
    pool: &PgPool, review_id: Uuid, dimension_id: Uuid, rating: i32, comment: Option<&str>,
) -> Result<ReviewScore, sqlx::Error> {
    sqlx::query_as::<_, ReviewScore>(
        r#"INSERT INTO review_scores (review_id, dimension_id, rating, comment)
        VALUES ($1,$2,$3,$4)
        ON CONFLICT (review_id, dimension_id) DO UPDATE SET rating=$3, comment=$4
        RETURNING *"#,
    ).bind(review_id).bind(dimension_id).bind(rating).bind(comment).fetch_one(pool).await
}

pub async fn get_scores(pool: &PgPool, review_id: Uuid) -> Result<Vec<ReviewScore>, sqlx::Error> {
    sqlx::query_as::<_, ReviewScore>(
        "SELECT * FROM review_scores WHERE review_id=$1 ORDER BY created_at",
    ).bind(review_id).fetch_all(pool).await
}

// ── Consistency Check Results ───────────────────────────────

pub async fn save_consistency_result(
    pool: &PgPool, review_id: Uuid, rule_id: Uuid, severity: &ConsistencySeverity, message: &str,
) -> Result<ConsistencyCheckResult, sqlx::Error> {
    sqlx::query_as::<_, ConsistencyCheckResult>(
        r#"INSERT INTO consistency_check_results (review_id, rule_id, severity, message)
        VALUES ($1,$2,$3,$4) RETURNING *"#,
    ).bind(review_id).bind(rule_id).bind(severity).bind(message).fetch_one(pool).await
}

pub async fn get_consistency_results(pool: &PgPool, review_id: Uuid) -> Result<Vec<ConsistencyCheckResult>, sqlx::Error> {
    sqlx::query_as::<_, ConsistencyCheckResult>(
        "SELECT * FROM consistency_check_results WHERE review_id=$1",
    ).bind(review_id).fetch_all(pool).await
}

pub async fn clear_consistency_results(pool: &PgPool, review_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM consistency_check_results WHERE review_id=$1")
        .bind(review_id).execute(pool).await?;
    Ok(())
}

// ── Conflict of Interest ────────────────────────────────────

pub async fn declare_coi(
    pool: &PgPool, reviewer_id: Uuid, conflict_type: &str, target_user_id: Option<Uuid>,
    department: Option<&str>, description: Option<&str>, declared_by: Option<Uuid>,
) -> Result<ConflictOfInterest, sqlx::Error> {
    sqlx::query_as::<_, ConflictOfInterest>(
        r#"INSERT INTO conflict_of_interest (reviewer_id, conflict_type, target_user_id, department, description, declared_by)
        VALUES ($1,$2,$3,$4,$5,$6) RETURNING *"#,
    )
    .bind(reviewer_id).bind(conflict_type).bind(target_user_id)
    .bind(department).bind(description).bind(declared_by)
    .fetch_one(pool).await
}

pub async fn get_coi_for_reviewer(pool: &PgPool, reviewer_id: Uuid) -> Result<Vec<ConflictOfInterest>, sqlx::Error> {
    sqlx::query_as::<_, ConflictOfInterest>(
        "SELECT * FROM conflict_of_interest WHERE reviewer_id=$1 AND is_active=TRUE ORDER BY declared_at DESC",
    ).bind(reviewer_id).fetch_all(pool).await
}

pub async fn check_coi(
    pool: &PgPool, reviewer_id: Uuid, submitter_id: Uuid,
) -> Result<Vec<ConflictOfInterest>, sqlx::Error> {
    sqlx::query_as::<_, ConflictOfInterest>(
        r#"SELECT * FROM conflict_of_interest
        WHERE reviewer_id=$1 AND is_active=TRUE
          AND (target_user_id=$2
               OR (conflict_type='department' AND department IN (
                   SELECT department FROM reviewer_departments WHERE user_id=$2
               ))
          )"#,
    ).bind(reviewer_id).bind(submitter_id).fetch_all(pool).await
}

pub async fn revoke_coi(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE conflict_of_interest SET is_active=FALSE WHERE id=$1")
        .bind(id).execute(pool).await?;
    Ok(())
}

// ── Reviewer Departments ────────────────────────────────────

pub async fn set_reviewer_department(
    pool: &PgPool, user_id: Uuid, department: &str, is_primary: bool,
) -> Result<ReviewerDepartment, sqlx::Error> {
    sqlx::query_as::<_, ReviewerDepartment>(
        r#"INSERT INTO reviewer_departments (user_id, department, is_primary)
        VALUES ($1,$2,$3)
        ON CONFLICT (user_id, department) DO UPDATE SET is_primary=$3
        RETURNING *"#,
    ).bind(user_id).bind(department).bind(is_primary).fetch_one(pool).await
}

pub async fn get_reviewer_departments(pool: &PgPool, user_id: Uuid) -> Result<Vec<ReviewerDepartment>, sqlx::Error> {
    sqlx::query_as::<_, ReviewerDepartment>(
        "SELECT * FROM reviewer_departments WHERE user_id=$1",
    ).bind(user_id).fetch_all(pool).await
}
