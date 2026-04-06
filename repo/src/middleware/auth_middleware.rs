use actix_web::{
    dev::ServiceRequest, Error, HttpMessage, HttpResponse,
    web,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::session::validate_session;
use crate::errors::AppError;
use crate::models::User;

/// Authenticated user context injected into request extensions
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub username: String,
    pub role: crate::models::UserRole,
    pub session_id: Uuid,
}

/// Extract the session token from the Authorization header
pub fn extract_token(req: &actix_web::HttpRequest) -> Option<String> {
    req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(String::from)
}

/// Validate session and load user info. Returns AuthenticatedUser on success.
pub async fn authenticate_request(
    pool: &PgPool,
    req: &actix_web::HttpRequest,
) -> Result<AuthenticatedUser, AppError> {
    let token = extract_token(req).ok_or(AppError::Unauthorized)?;

    let session = validate_session(pool, &token).await?;

    let user = crate::db::users::find_by_id(pool, session.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if user.status != crate::models::AccountStatus::Active {
        return Err(AppError::Unauthorized);
    }

    Ok(AuthenticatedUser {
        user_id: user.id,
        username: user.username,
        role: user.role,
        session_id: session.id,
    })
}

/// Helper to require specific roles
pub fn require_role(
    auth_user: &AuthenticatedUser,
    allowed_roles: &[crate::models::UserRole],
) -> Result<(), AppError> {
    if allowed_roles.contains(&auth_user.role) {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}
