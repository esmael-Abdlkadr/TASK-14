use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;

/// Anti-bot throttling: detect and block high-frequency automated requests.
/// Checks request patterns that indicate bot behavior on specific endpoints.
pub async fn check_antibot(
    pool: &PgPool,
    user_id: Uuid,
    action: &str,
) -> Result<(), AppError> {
    let window = Utc::now() - Duration::seconds(10);

    // Check for burst requests (>10 of the same action in 10 seconds)
    let burst_count: i64 = sqlx::query_scalar(
        r#"
        SELECT request_count FROM rate_limit_buckets
        WHERE user_id = $1 AND bucket_key = $2 AND window_start > $3
        "#,
    )
    .bind(user_id)
    .bind(format!("antibot:{}", action))
    .bind(window)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?
    .unwrap_or(0);

    if burst_count > 10 {
        log::warn!(
            "Anti-bot triggered for user {} on action {}: {} requests in 10s",
            user_id,
            action,
            burst_count
        );
        return Err(AppError::BotDetected);
    }

    // Track this request
    let _ = sqlx::query(
        r#"
        INSERT INTO rate_limit_buckets (user_id, bucket_key, request_count, window_start)
        VALUES ($1, $2, 1, NOW())
        ON CONFLICT (user_id, bucket_key) DO UPDATE SET
            request_count = CASE
                WHEN rate_limit_buckets.window_start < $3 THEN 1
                ELSE rate_limit_buckets.request_count + 1
            END,
            window_start = CASE
                WHEN rate_limit_buckets.window_start < $3 THEN NOW()
                ELSE rate_limit_buckets.window_start
            END
        "#,
    )
    .bind(user_id)
    .bind(format!("antibot:{}", action))
    .bind(window)
    .execute(pool)
    .await;

    Ok(())
}
