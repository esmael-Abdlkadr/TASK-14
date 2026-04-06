use sqlx::PgPool;
use uuid::Uuid;

use crate::db;
use crate::errors::AppError;
use crate::models::{AccountStatus, LoginRequest, LoginResponse, UserResponse};

use super::password::verify_password;
use super::session::create_session;

/// Records a login attempt in the database for anomaly detection
async fn record_login_attempt(
    pool: &PgPool,
    user_id: Option<Uuid>,
    username: &str,
    success: bool,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
    failure_reason: Option<&str>,
) {
    let _ = sqlx::query(
        r#"
        INSERT INTO login_attempts (user_id, username, success, ip_address, user_agent, failure_reason)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(user_id)
    .bind(username)
    .bind(success)
    .bind(ip_address)
    .bind(user_agent)
    .bind(failure_reason)
    .execute(pool)
    .await;
}

pub async fn login(
    pool: &PgPool,
    req: &LoginRequest,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) -> Result<LoginResponse, AppError> {
    // Find the user
    let user = db::users::find_by_username(pool, &req.username)
        .await?
        .ok_or_else(|| {
            // Don't reveal whether the username exists
            AppError::InvalidCredentials
        })?;

    // Check if account is locked
    if user.status == AccountStatus::Locked {
        // Try to unlock if lockout period has expired
        if let Some(_unlocked) = db::users::unlock_if_expired(pool, user.id).await? {
            // Account was unlocked, proceed with login
        } else {
            let minutes_remaining = user
                .locked_until
                .map(|lt| {
                    let remaining = lt - chrono::Utc::now();
                    remaining.num_minutes().max(1)
                })
                .unwrap_or(15);

            record_login_attempt(
                pool,
                Some(user.id),
                &req.username,
                false,
                ip_address,
                user_agent,
                Some("account_locked"),
            )
            .await;

            return Err(AppError::AccountLocked { minutes_remaining });
        }
    }

    if user.status == AccountStatus::Disabled {
        record_login_attempt(
            pool,
            Some(user.id),
            &req.username,
            false,
            ip_address,
            user_agent,
            Some("account_disabled"),
        )
        .await;
        return Err(AppError::InvalidCredentials);
    }

    // Verify password
    let valid = verify_password(&req.password, &user.password_hash)?;
    if !valid {
        // Increment failed attempts (may trigger lockout)
        let updated_user = db::users::increment_failed_attempts(pool, user.id).await?;

        let reason = if updated_user.status == AccountStatus::Locked {
            "locked_after_max_attempts"
        } else {
            "invalid_password"
        };

        record_login_attempt(
            pool,
            Some(user.id),
            &req.username,
            false,
            ip_address,
            user_agent,
            Some(reason),
        )
        .await;

        if updated_user.status == AccountStatus::Locked {
            return Err(AppError::AccountLocked {
                minutes_remaining: 15,
            });
        }
        return Err(AppError::InvalidCredentials);
    }

    // Successful login — reset failed attempts
    db::users::reset_failed_attempts(pool, user.id).await?;

    // Handle device binding with encrypted fingerprint + hash for secure lookup
    let (device_trusted, requires_device_binding) =
        if let Some(ref fingerprint) = req.device_fingerprint {
            let fp_hash = {
                use sha2::{Sha256, Digest};
                let mut h = Sha256::new();
                h.update(fingerprint.as_bytes());
                hex::encode(h.finalize())
            };
            match db::devices::find_device_binding(pool, user.id, &fp_hash, fingerprint).await? {
                Some(device) => {
                    db::devices::update_last_seen(pool, device.id).await?;
                    (device.is_trusted, false)
                }
                None => {
                    // Encrypt fingerprint — if encryption fails, skip device bind
                    // entirely rather than persisting unencrypted data
                    match crate::encryption::field_encryption::encrypt_field(pool, fingerprint).await {
                        Ok(encrypted_fp) => {
                            db::devices::bind_device(pool, user.id, &fp_hash, &encrypted_fp, None).await?;
                            (false, true)
                        }
                        Err(e) => {
                            log::warn!("Device bind skipped: encryption unavailable ({})", e);
                            (false, true) // login succeeds, device not bound
                        }
                    }
                }
            }
        } else {
            (false, false)
        };

    // Create session
    let (token, _session) = create_session(pool, user.id, ip_address, user_agent).await?;

    record_login_attempt(
        pool,
        Some(user.id),
        &req.username,
        true,
        ip_address,
        user_agent,
        None,
    )
    .await;

    Ok(LoginResponse {
        session_token: token,
        user: UserResponse::from(user),
        device_trusted,
        requires_device_binding,
    })
}
