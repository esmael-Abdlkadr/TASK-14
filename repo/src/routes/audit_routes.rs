use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;

use crate::audit::export::export_audit_log;
use crate::db::audit::{query_audit_log, verify_chain_integrity};
use crate::errors::AppError;
use crate::middleware::auth_middleware::{authenticate_request, require_role};
use crate::middleware::audit_middleware::audit_action;
use crate::middleware::rate_limit_middleware::apply_rate_limit;
use crate::models::{AuditExportQuery, AuditLogPage, AuditLogQuery, UserRole};
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

/// GET /api/audit
pub async fn query_audit(
    pool: web::Data<PgPool>,
    query: web::Query<AuditLogQuery>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    require_role(
        &auth_user,
        &[UserRole::OperationsAdmin, UserRole::DepartmentManager],
    )?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    let (entries, total) = query_audit_log(pool.get_ref(), &query)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(50);

    Ok(HttpResponse::Ok().json(AuditLogPage {
        entries,
        total,
        page,
        page_size,
    }))
}

/// GET /api/audit/export  (requires step-up)
pub async fn export_audit(
    pool: web::Data<PgPool>,
    query: web::Query<AuditExportQuery>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    require_role(
        &auth_user,
        &[UserRole::OperationsAdmin, UserRole::DepartmentManager],
    )?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    // Determine step-up action based on format
    let stepup_action = match query.format {
        crate::models::ExportFormat::Csv => "export_csv",
        crate::models::ExportFormat::Pdf => "export_pdf",
    };

    let has_stepup =
        stepup::check_stepup(pool.get_ref(), auth_user.session_id, stepup_action).await?;
    if !has_stepup {
        return Err(AppError::StepUpRequired(stepup_action.to_string()));
    }

    let (data, content_type, filename) = export_audit_log(pool.get_ref(), &query).await?;

    audit_action(
        pool.get_ref(),
        &auth_user,
        &format!("audit_export_{}", stepup_action),
        Some("audit_log"),
        None,
        Some(serde_json::json!({"format": stepup_action})),
        get_ip(&req).as_deref(),
        get_user_agent(&req).as_deref(),
    )
    .await?;

    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", content_type))
        .insert_header((
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", filename),
        ))
        .body(data))
}

/// GET /api/audit/integrity
pub async fn check_integrity(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth_user, &[UserRole::OperationsAdmin])?;

    let is_valid = verify_chain_integrity(pool.get_ref())
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "chain_valid": is_valid,
        "checked_at": chrono::Utc::now()
    })))
}

pub fn audit_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/audit")
            .route("", web::get().to(query_audit))
            .route("/export", web::get().to(export_audit))
            .route("/integrity", web::get().to(check_integrity)),
    );
}
