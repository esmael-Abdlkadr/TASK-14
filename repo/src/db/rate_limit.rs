use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

const RATE_LIMIT_WINDOW_SECS: i64 = 60;
const MAX_REQUESTS_PER_WINDOW: i32 = 60;

pub struct RateLimitResult {
    pub allowed: bool,
    pub remaining: i32,
    pub retry_after_secs: u64,
}

pub async fn check_rate_limit(
    pool: &PgPool,
    user_id: Uuid,
    bucket_key: &str,
) -> Result<RateLimitResult, sqlx::Error> {
    let window_start_threshold = Utc::now() - Duration::seconds(RATE_LIMIT_WINDOW_SECS);

    // Upsert the bucket and increment, resetting if window expired
    let row = sqlx::query_as::<_, (i32, chrono::DateTime<Utc>)>(
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
        RETURNING request_count, window_start
        "#,
    )
    .bind(user_id)
    .bind(bucket_key)
    .bind(window_start_threshold)
    .fetch_one(pool)
    .await?;

    let (count, window_start) = row;
    let remaining = (MAX_REQUESTS_PER_WINDOW - count).max(0);
    let window_end = window_start + Duration::seconds(RATE_LIMIT_WINDOW_SECS);
    let retry_after = if count > MAX_REQUESTS_PER_WINDOW {
        (window_end - Utc::now()).num_seconds().max(0) as u64
    } else {
        0
    };

    Ok(RateLimitResult {
        allowed: count <= MAX_REQUESTS_PER_WINDOW,
        remaining,
        retry_after_secs: retry_after,
    })
}

pub async fn cleanup_stale_buckets(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let threshold = Utc::now() - Duration::seconds(RATE_LIMIT_WINDOW_SECS * 2);
    let result = sqlx::query("DELETE FROM rate_limit_buckets WHERE window_start < $1")
        .bind(threshold)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}
