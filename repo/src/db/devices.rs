use sqlx::PgPool;
use uuid::Uuid;

use crate::models::DeviceBinding;

/// Find device by canonical (user_id, fingerprint_hash).
/// Falls back to legacy plaintext match for pre-migration rows without hash.
pub async fn find_device_binding(
    pool: &PgPool,
    user_id: Uuid,
    fingerprint_hash: &str,
    legacy_plaintext: &str,
) -> Result<Option<DeviceBinding>, sqlx::Error> {
    sqlx::query_as::<_, DeviceBinding>(
        r#"
        SELECT * FROM device_bindings
        WHERE user_id = $1
          AND (fingerprint_hash = $2
               OR (fingerprint_hash IS NULL AND device_fingerprint = $3))
        "#,
    )
    .bind(user_id)
    .bind(fingerprint_hash)
    .bind(legacy_plaintext)
    .fetch_optional(pool)
    .await
}

/// Bind device using encrypted fingerprint and hash only.
/// `device_fingerprint` column receives a non-sensitive placeholder for legacy
/// NOT NULL constraint compatibility; actual sensitive data is in `encrypted_fingerprint`.
/// Canonical dedup is via `(user_id, fingerprint_hash)` unique index.
pub async fn bind_device(
    pool: &PgPool,
    user_id: Uuid,
    fingerprint_hash: &str,
    encrypted_fingerprint: &str,
    device_name: Option<&str>,
) -> Result<DeviceBinding, sqlx::Error> {
    let placeholder = format!("sha256:{}", &fingerprint_hash[..16]);

    sqlx::query_as::<_, DeviceBinding>(
        r#"
        INSERT INTO device_bindings
            (user_id, device_fingerprint, device_name, encrypted_fingerprint, fingerprint_hash)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (user_id, fingerprint_hash) WHERE fingerprint_hash IS NOT NULL
        DO UPDATE SET last_seen_at = NOW(),
                      encrypted_fingerprint = EXCLUDED.encrypted_fingerprint
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(&placeholder)
    .bind(device_name)
    .bind(encrypted_fingerprint)
    .bind(fingerprint_hash)
    .fetch_one(pool)
    .await
}

pub async fn trust_device(pool: &PgPool, device_id: Uuid) -> Result<DeviceBinding, sqlx::Error> {
    sqlx::query_as::<_, DeviceBinding>(
        r#"
        UPDATE device_bindings
        SET is_trusted = TRUE, last_seen_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(device_id)
    .fetch_one(pool)
    .await
}

pub async fn update_last_seen(pool: &PgPool, device_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE device_bindings SET last_seen_at = NOW() WHERE id = $1")
        .bind(device_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_user_devices(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<DeviceBinding>, sqlx::Error> {
    sqlx::query_as::<_, DeviceBinding>(
        "SELECT * FROM device_bindings WHERE user_id = $1 ORDER BY last_seen_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn remove_device(pool: &PgPool, device_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM device_bindings WHERE id = $1 AND user_id = $2",
    )
    .bind(device_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}
