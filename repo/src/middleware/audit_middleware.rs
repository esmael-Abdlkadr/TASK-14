use sqlx::PgPool;

use crate::audit::service::record_action;
use crate::errors::AppError;
use crate::middleware::auth_middleware::AuthenticatedUser;
use crate::models::AuditLogEntry;

/// Record an audited action by an authenticated user
pub async fn audit_action(
    pool: &PgPool,
    user: &AuthenticatedUser,
    action: &str,
    resource_type: Option<&str>,
    resource_id: Option<&str>,
    details: Option<serde_json::Value>,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) -> Result<AuditLogEntry, AppError> {
    record_action(
        pool,
        Some(user.user_id),
        &user.username,
        Some(user.role.clone()),
        action,
        resource_type,
        resource_id,
        details,
        ip_address,
        user_agent,
        Some(user.session_id),
    )
    .await
}
