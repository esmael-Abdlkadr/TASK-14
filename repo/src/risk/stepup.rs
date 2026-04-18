use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::password::verify_password;
use crate::db;
use crate::errors::AppError;
use crate::models::StepUpVerification;

/// Actions that require step-up verification (re-enter password)
const CRITICAL_ACTIONS: &[&str] = &[
    "export_csv",
    "export_pdf",
    "rule_rollback",
    "result_publication",
    "user_role_change",
    "system_config_change",
];

const STEPUP_VALIDITY_MINUTES: i64 = 5;

/// Check if an action requires step-up verification
pub fn requires_stepup(action: &str) -> bool {
    CRITICAL_ACTIONS.contains(&action)
}

/// Verify step-up: user re-enters password for critical action
pub async fn perform_stepup(
    pool: &PgPool,
    session_id: Uuid,
    user_id: Uuid,
    password: &str,
    action_type: &str,
) -> Result<StepUpVerification, AppError> {
    // Verify the user's password
    let user = db::users::find_by_id(pool, user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let valid = verify_password(password, &user.password_hash)?;
    if !valid {
        return Err(AppError::StepUpFailed);
    }

    let expires_at = Utc::now() + Duration::minutes(STEPUP_VALIDITY_MINUTES);

    let verification = sqlx::query_as::<_, StepUpVerification>(
        r#"
        INSERT INTO stepup_verifications (session_id, user_id, action_type, expires_at)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(session_id)
    .bind(user_id)
    .bind(action_type)
    .bind(expires_at)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(verification)
}

/// Check if a valid step-up verification exists for this session and action
pub async fn check_stepup(
    pool: &PgPool,
    session_id: Uuid,
    action_type: &str,
) -> Result<bool, AppError> {
    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM stepup_verifications
            WHERE session_id = $1
              AND action_type = $2
              AND expires_at > NOW()
        )
        "#,
    )
    .bind(session_id)
    .bind(action_type)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(exists)
}

#[cfg(test)]
mod requires_stepup_tests {
    use super::requires_stepup;

    #[test]
    fn critical_actions_require_stepup() {
        for action in [
            "export_csv",
            "export_pdf",
            "rule_rollback",
            "result_publication",
            "user_role_change",
            "system_config_change",
        ] {
            assert!(requires_stepup(action), "{}", action);
        }
    }

    #[test]
    fn benign_actions_skip_stepup() {
        assert!(!requires_stepup("read_dashboard"));
        assert!(!requires_stepup(""));
    }
}
