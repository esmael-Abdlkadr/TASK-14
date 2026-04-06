use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Session;

const SESSION_DURATION_MINUTES: i64 = 30;

pub async fn create_session(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &str,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) -> Result<Session, sqlx::Error> {
    let expires_at = Utc::now() + Duration::minutes(SESSION_DURATION_MINUTES);

    sqlx::query_as::<_, Session>(
        r#"
        INSERT INTO sessions (user_id, token_hash, expires_at, ip_address, user_agent)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .bind(ip_address)
    .bind(user_agent)
    .fetch_one(pool)
    .await
}

pub async fn find_valid_session(pool: &PgPool, token_hash: &str) -> Result<Option<Session>, sqlx::Error> {
    sqlx::query_as::<_, Session>(
        r#"
        SELECT * FROM sessions
        WHERE token_hash = $1
          AND is_valid = TRUE
          AND expires_at > NOW()
        "#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
}

pub async fn touch_session(pool: &PgPool, session_id: Uuid) -> Result<(), sqlx::Error> {
    let new_expires = Utc::now() + Duration::minutes(SESSION_DURATION_MINUTES);
    sqlx::query(
        r#"
        UPDATE sessions
        SET last_active_at = NOW(), expires_at = $2
        WHERE id = $1 AND is_valid = TRUE
        "#,
    )
    .bind(session_id)
    .bind(new_expires)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn invalidate_session(pool: &PgPool, session_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE sessions SET is_valid = FALSE WHERE id = $1")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn invalidate_all_user_sessions(pool: &PgPool, user_id: Uuid) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sessions SET is_valid = FALSE WHERE user_id = $1 AND is_valid = TRUE",
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn cleanup_expired_sessions(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sessions SET is_valid = FALSE WHERE expires_at <= NOW() AND is_valid = TRUE",
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn check_idle_timeout(pool: &PgPool, session_id: Uuid) -> Result<bool, sqlx::Error> {
    let row = sqlx::query_as::<_, Session>(
        r#"
        SELECT * FROM sessions
        WHERE id = $1 AND is_valid = TRUE
        "#,
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(session) => {
            let idle_limit = session.last_active_at + Duration::minutes(SESSION_DURATION_MINUTES);
            if Utc::now() > idle_limit {
                invalidate_session(pool, session_id).await?;
                Ok(false)
            } else {
                Ok(true)
            }
        }
        None => Ok(false),
    }
}
