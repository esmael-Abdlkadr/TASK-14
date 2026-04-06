use actix_web::HttpResponse;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::risk::rate_limiter::check_user_rate_limit;

/// Apply rate limiting for an authenticated user.
/// Returns remaining request count on success, or error if limit exceeded.
pub async fn apply_rate_limit(pool: &PgPool, user_id: Uuid) -> Result<i32, AppError> {
    check_user_rate_limit(pool, user_id).await
}
