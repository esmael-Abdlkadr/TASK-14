use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{AccountStatus, CreateUserRequest, User, UserRole};

pub async fn create_user(
    pool: &PgPool,
    req: &CreateUserRequest,
    password_hash: &str,
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (username, password_hash, role)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(&req.username)
    .bind(password_hash)
    .bind(&req.role)
    .fetch_one(pool)
    .await
}

pub async fn find_by_username(pool: &PgPool, username: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(pool)
        .await
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn increment_failed_attempts(pool: &PgPool, user_id: Uuid) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET failed_attempts = failed_attempts + 1,
            last_failed_at = NOW(),
            status = CASE
                WHEN failed_attempts + 1 >= 5 THEN 'locked'::account_status
                ELSE status
            END,
            locked_until = CASE
                WHEN failed_attempts + 1 >= 5 THEN NOW() + INTERVAL '15 minutes'
                ELSE locked_until
            END,
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
}

pub async fn reset_failed_attempts(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE users
        SET failed_attempts = 0, last_failed_at = NULL, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn unlock_if_expired(pool: &PgPool, user_id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET status = 'active'::account_status,
            failed_attempts = 0,
            locked_until = NULL,
            last_failed_at = NULL,
            updated_at = NOW()
        WHERE id = $1
          AND status = 'locked'::account_status
          AND locked_until IS NOT NULL
          AND locked_until <= NOW()
        RETURNING *
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn update_role(
    pool: &PgPool,
    user_id: Uuid,
    role: &UserRole,
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        UPDATE users SET role = $2, updated_at = NOW()
        WHERE id = $1 RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(role)
    .fetch_one(pool)
    .await
}

pub async fn list_users(pool: &PgPool) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
}
