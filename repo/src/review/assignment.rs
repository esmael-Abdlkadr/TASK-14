use sqlx::PgPool;
use uuid::Uuid;

use crate::db::review as review_db;
use crate::errors::AppError;
use crate::models::{
    AssignmentMethod, ReviewAssignment, ReviewTargetType,
};

/// Auto-assign a review to the most available eligible reviewer.
/// Filters out reviewers with COI (department or declared) and already-assigned.
pub async fn auto_assign(
    pool: &PgPool,
    target_type: &ReviewTargetType,
    target_id: Uuid,
    scorecard_id: Uuid,
    submitter_id: Uuid,
    is_blind: bool,
    assigned_by: Option<Uuid>,
    due_date: Option<chrono::NaiveDate>,
) -> Result<ReviewAssignment, AppError> {
    let eligible = review_db::get_eligible_reviewers(pool, target_type, target_id, submitter_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    if eligible.is_empty() {
        return Err(AppError::BadRequest(
            "No eligible reviewers available (all reviewers are conflicted, already assigned, or unavailable)".to_string(),
        ));
    }

    // Pick the reviewer with the fewest pending assignments (already sorted by the query)
    let reviewer = &eligible[0];

    let assignment = review_db::create_assignment(
        pool,
        reviewer.id,
        target_type,
        target_id,
        scorecard_id,
        &AssignmentMethod::Automatic,
        is_blind,
        assigned_by,
        due_date,
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    log::info!(
        "Auto-assigned review for {:?}/{} to reviewer {} ({})",
        target_type, target_id, reviewer.id, reviewer.username
    );

    Ok(assignment)
}

/// Manual assignment with COI check
pub async fn manual_assign(
    pool: &PgPool,
    reviewer_id: Uuid,
    target_type: &ReviewTargetType,
    target_id: Uuid,
    scorecard_id: Uuid,
    submitter_id: Uuid,
    is_blind: bool,
    assigned_by: Option<Uuid>,
    due_date: Option<chrono::NaiveDate>,
) -> Result<ReviewAssignment, AppError> {
    // Check COI
    let conflicts = review_db::check_coi(pool, reviewer_id, submitter_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    if !conflicts.is_empty() {
        let reasons: Vec<String> = conflicts
            .iter()
            .map(|c| format!("{}: {}", c.conflict_type, c.description.as_deref().unwrap_or("N/A")))
            .collect();
        return Err(AppError::BadRequest(format!(
            "Reviewer has conflict of interest: {}",
            reasons.join("; ")
        )));
    }

    // Verify reviewer is not the submitter
    if reviewer_id == submitter_id {
        return Err(AppError::BadRequest(
            "Cannot assign review to the submitter".to_string(),
        ));
    }

    let assignment = review_db::create_assignment(
        pool,
        reviewer_id,
        target_type,
        target_id,
        scorecard_id,
        &AssignmentMethod::Manual,
        is_blind,
        assigned_by,
        due_date,
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(assignment)
}

/// Reassign after recusal: auto-pick a new reviewer
pub async fn reassign_after_recusal(
    pool: &PgPool,
    original_assignment: &ReviewAssignment,
    submitter_id: Uuid,
) -> Result<ReviewAssignment, AppError> {
    let new_assignment = auto_assign(
        pool,
        &original_assignment.target_type,
        original_assignment.target_id,
        original_assignment.scorecard_id,
        submitter_id,
        original_assignment.is_blind,
        None,
        original_assignment.due_date,
    )
    .await?;

    Ok(new_assignment)
}

/// Build anonymized target summary for blind reviews
pub fn anonymize_submitter_name(name: &str) -> String {
    "Anonymous Submitter".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anonymize() {
        assert_eq!(anonymize_submitter_name("John Doe"), "Anonymous Submitter");
    }
}
