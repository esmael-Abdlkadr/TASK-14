use sqlx::PgPool;
use uuid::Uuid;

use crate::db::rate_limit;
use crate::errors::AppError;

/// Check if a user has exceeded their rate limit (60 requests/minute)
pub async fn check_user_rate_limit(pool: &PgPool, user_id: Uuid) -> Result<i32, AppError> {
    let result = rate_limit::check_rate_limit(pool, user_id, "global")
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    if !result.allowed {
        return Err(AppError::RateLimitExceeded {
            retry_after_secs: result.retry_after_secs,
        });
    }

    Ok(result.remaining)
}

/// Check rate limit for a specific action bucket (e.g., "login", "export")
pub async fn check_action_rate_limit(
    pool: &PgPool,
    user_id: Uuid,
    action: &str,
    max_per_minute: i32,
) -> Result<i32, AppError> {
    let result = rate_limit::check_rate_limit(pool, user_id, action)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Use custom limit for specific actions
    if result.remaining < (60 - max_per_minute) {
        return Err(AppError::RateLimitExceeded {
            retry_after_secs: result.retry_after_secs,
        });
    }

    Ok(result.remaining)
}
