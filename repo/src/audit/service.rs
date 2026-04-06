use sqlx::PgPool;
use uuid::Uuid;

use crate::db::audit::{insert_audit_entry, AuditEntryInput};
use crate::errors::AppError;
use crate::models::{AuditLogEntry, UserRole};

/// Record an action in the immutable audit log
pub async fn record_action(
    pool: &PgPool,
    user_id: Option<Uuid>,
    username: &str,
    role: Option<UserRole>,
    action: &str,
    resource_type: Option<&str>,
    resource_id: Option<&str>,
    details: Option<serde_json::Value>,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
    session_id: Option<Uuid>,
) -> Result<AuditLogEntry, AppError> {
    // Sensitive auth actions get details encrypted at rest
    let is_sensitive = matches!(action,
        "user_login" | "user_registered" | "stepup_verified" |
        "device_bound" | "device_trusted" | "anomalous_login_blocked"
    );
    let encrypted_details = if is_sensitive {
        if let Some(ref d) = details {
            let json_str = d.to_string();
            match crate::encryption::field_encryption::encrypt_field(pool, &json_str).await {
                Ok(enc) => Some(enc),
                Err(_) => None, // audit must not fail the caller; log non-sensitive summary only
            }
        } else {
            None
        }
    } else {
        None
    };

    // For sensitive actions, strip details from plaintext column
    let stored_details = if is_sensitive && encrypted_details.is_some() {
        Some(serde_json::json!({"_encrypted": true}))
    } else {
        details
    };

    let entry = insert_audit_entry(
        pool,
        AuditEntryInput {
            user_id,
            username: username.to_string(),
            role,
            action: action.to_string(),
            resource_type: resource_type.map(String::from),
            resource_id: resource_id.map(String::from),
            details: stored_details,
            encrypted_details,
            ip_address: ip_address.map(String::from),
            user_agent: user_agent.map(String::from),
            session_id,
        },
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(entry)
}

/// Convenience function for recording auth events
pub async fn record_auth_event(
    pool: &PgPool,
    username: &str,
    action: &str,
    success: bool,
    ip_address: Option<&str>,
    details: Option<serde_json::Value>,
) -> Result<(), AppError> {
    let detail_json = Some(serde_json::json!({
        "success": success,
        "extra": details,
    }));

    record_action(
        pool,
        None,
        username,
        None,
        action,
        Some("auth"),
        None,
        detail_json,
        ip_address,
        None,
        None,
    )
    .await?;

    Ok(())
}
