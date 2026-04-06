use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db;
use crate::errors::AppError;
use crate::middleware::auth_middleware::{authenticate_request, require_role, AuthenticatedUser};
use crate::middleware::audit_middleware::audit_action;
use crate::middleware::rate_limit_middleware::apply_rate_limit;
use crate::models::{UserResponse, UserRole};

fn get_ip(req: &HttpRequest) -> Option<String> {
    req.peer_addr().map(|a| a.ip().to_string())
}

fn get_user_agent(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("User-Agent")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
}

/// GET /api/users
pub async fn list_users(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth_user, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    let users = db::users::list_users(pool.get_ref()).await?;
    let responses: Vec<UserResponse> = users.into_iter().map(UserResponse::from).collect();

    Ok(HttpResponse::Ok().json(responses))
}

/// GET /api/users/{id}
pub async fn get_user(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    let user_id = path.into_inner();

    // Users can view themselves; admins/managers can view anyone
    if auth_user.user_id != user_id {
        require_role(&auth_user, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;
    }

    let user = db::users::find_by_id(pool.get_ref(), user_id)
        .await?
        .ok_or(AppError::NotFound("User not found".to_string()))?;

    Ok(HttpResponse::Ok().json(UserResponse::from(user)))
}

/// PUT /api/users/{id}/role  (requires step-up)
pub async fn update_user_role(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    body: web::Json<serde_json::Value>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth_user, &[UserRole::OperationsAdmin])?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    // Require step-up verification for role changes
    let has_stepup =
        crate::risk::stepup::check_stepup(pool.get_ref(), auth_user.session_id, "user_role_change")
            .await?;
    if !has_stepup {
        return Err(AppError::StepUpRequired("user_role_change".to_string()));
    }

    let user_id = path.into_inner();
    let role: UserRole = serde_json::from_value(
        body.get("role")
            .cloned()
            .ok_or(AppError::BadRequest("Missing 'role' field".to_string()))?,
    )
    .map_err(|e| AppError::BadRequest(format!("Invalid role: {}", e)))?;

    let updated = db::users::update_role(pool.get_ref(), user_id, &role).await?;

    audit_action(
        pool.get_ref(),
        &auth_user,
        "user_role_changed",
        Some("user"),
        Some(&user_id.to_string()),
        Some(serde_json::json!({"new_role": role})),
        get_ip(&req).as_deref(),
        get_user_agent(&req).as_deref(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(UserResponse::from(updated)))
}

pub fn user_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/users")
            .route("", web::get().to(list_users))
            .route("/{id}", web::get().to(get_user))
            .route("/{id}/role", web::put().to(update_user_role)),
    );
}
