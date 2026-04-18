use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::{PgPool, Row};

use crate::audit::service::record_auth_event;
use crate::auth::login::login;
use crate::auth::password::{hash_password, validate_password};
use crate::db;
use crate::errors::{map_sqlx_unique_violation, AppError};
use crate::middleware::auth_middleware::{authenticate_request, AuthenticatedUser};
use crate::models::{CreateUserRequest, LoginRequest, StepUpRequest};
use crate::risk::anomaly::check_login_anomaly;
use crate::risk::antibot::check_antibot;
use crate::risk::stepup;

fn get_ip(req: &HttpRequest) -> Option<String> {
    req.peer_addr().map(|a| a.ip().to_string())
}

fn get_user_agent(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("User-Agent")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
}

/// POST /api/auth/register
/// Requires OperationsAdmin role. On first-boot (no users exist), allows one
/// bootstrap admin if CIVICSORT_BOOTSTRAP_ADMIN=1 is set.
pub async fn register(
    pool: web::Data<PgPool>,
    body: web::Json<CreateUserRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    // Check if this is bootstrap context (empty DB + env flag)
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool.get_ref())
        .await
        .unwrap_or(1); // default to non-zero so bootstrap is denied on DB error

    let is_bootstrap = user_count == 0
        && std::env::var("CIVICSORT_BOOTSTRAP_ADMIN").unwrap_or_default() == "1";

    if !is_bootstrap {
        // Normal path: require authenticated admin
        let auth_user = crate::middleware::auth_middleware::authenticate_request(
            pool.get_ref(), &req,
        ).await?;
        crate::middleware::auth_middleware::require_role(
            &auth_user, &[crate::models::UserRole::OperationsAdmin],
        )?;
    }

    validate_password(&body.password)?;

    let password_hash = hash_password(&body.password)?;
    let user = db::users::create_user(pool.get_ref(), &body, &password_hash).await
        .map_err(|e| map_sqlx_unique_violation(e, "Username already taken"))?;

    record_auth_event(
        pool.get_ref(),
        &body.username,
        "user_registered",
        true,
        get_ip(&req).as_deref(),
        Some(serde_json::json!({"role": &body.role, "bootstrap": is_bootstrap})),
    )
    .await?;

    Ok(HttpResponse::Created().json(crate::models::UserResponse::from(user)))
}

/// POST /api/auth/login
pub async fn login_handler(
    pool: web::Data<PgPool>,
    body: web::Json<LoginRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let ip = get_ip(&req);
    let ua = get_user_agent(&req);

    // Anti-bot throttling on login attempts
    // Use a deterministic UUID from username to track per-user login rate without auth
    let login_track_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, body.username.as_bytes());
    check_antibot(pool.get_ref(), login_track_id, "login").await?;

    // Anomaly detection
    let anomaly = check_login_anomaly(pool.get_ref(), &body.username, ip.as_deref()).await?;
    if anomaly.is_anomalous {
        log::warn!(
            "Anomalous login detected for {}: {:?} (score: {})",
            body.username,
            anomaly.reasons,
            anomaly.risk_score
        );
        record_auth_event(
            pool.get_ref(),
            &body.username,
            "anomalous_login_blocked",
            false,
            ip.as_deref(),
            Some(serde_json::json!({
                "reasons": anomaly.reasons,
                "risk_score": anomaly.risk_score
            })),
        )
        .await?;
    }

    let result = login(pool.get_ref(), &body, ip.as_deref(), ua.as_deref()).await?;

    record_auth_event(
        pool.get_ref(),
        &body.username,
        "user_login",
        true,
        ip.as_deref(),
        None,
    )
    .await?;

    Ok(HttpResponse::Ok().json(result))
}

/// POST /api/auth/logout
pub async fn logout_handler(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;

    crate::auth::session::invalidate_session(pool.get_ref(), auth_user.session_id).await?;

    record_auth_event(
        pool.get_ref(),
        &auth_user.username,
        "user_logout",
        true,
        get_ip(&req).as_deref(),
        None,
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Logged out successfully"})))
}

/// POST /api/auth/stepup
pub async fn stepup_handler(
    pool: web::Data<PgPool>,
    body: web::Json<StepUpRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;

    // Anti-bot check on step-up
    check_antibot(pool.get_ref(), auth_user.user_id, "stepup").await?;

    if !stepup::requires_stepup(&body.action_type) {
        return Err(AppError::BadRequest(format!(
            "Action '{}' does not require step-up verification",
            body.action_type
        )));
    }

    let verification = stepup::perform_stepup(
        pool.get_ref(),
        auth_user.session_id,
        auth_user.user_id,
        &body.password,
        &body.action_type,
    )
    .await?;

    crate::middleware::audit_middleware::audit_action(
        pool.get_ref(),
        &auth_user,
        "stepup_verified",
        Some("stepup"),
        Some(&body.action_type),
        None,
        get_ip(&req).as_deref(),
        get_user_agent(&req).as_deref(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Step-up verification successful",
        "action_type": body.action_type,
        "expires_at": verification.expires_at
    })))
}

/// GET /api/auth/session
pub async fn session_info_handler(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "user_id": auth_user.user_id,
        "username": auth_user.username,
        "role": auth_user.role,
        "session_id": auth_user.session_id,
    })))
}

pub fn auth_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/auth")
            .route("/register", web::post().to(register))
            .route("/login", web::post().to(login_handler))
            .route("/logout", web::post().to(logout_handler))
            .route("/stepup", web::post().to(stepup_handler))
            .route("/session", web::get().to(session_info_handler)),
    );
}
