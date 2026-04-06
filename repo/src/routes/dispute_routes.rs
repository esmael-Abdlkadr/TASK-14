use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::dispute as dispute_db;
use crate::db::knowledge_base as kb_db;
use crate::errors::AppError;
use crate::middleware::auth_middleware::{authenticate_request, require_role};
use crate::middleware::audit_middleware::audit_action;
use crate::middleware::rate_limit_middleware::apply_rate_limit;
use crate::models::*;

fn get_ip(req: &HttpRequest) -> Option<String> {
    req.peer_addr().map(|a| a.ip().to_string())
}
fn get_user_agent(req: &HttpRequest) -> Option<String> {
    req.headers().get("User-Agent").and_then(|v| v.to_str().ok()).map(String::from)
}

#[derive(Debug, serde::Deserialize)]
pub struct ListDisputesQuery {
    pub status: Option<DisputeStatus>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// POST /api/disputes
pub async fn create_dispute(
    pool: web::Data<PgPool>,
    body: web::Json<CreateDisputeRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    apply_rate_limit(pool.get_ref(), auth.user_id).await?;

    let dispute = dispute_db::create_dispute(
        pool.get_ref(),
        body.kb_entry_id,
        auth.user_id,
        &body.reason,
        body.proposed_category.as_deref(),
        body.proposed_instructions.as_deref(),
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    audit_action(
        pool.get_ref(), &auth, "dispute_created", Some("classification_dispute"),
        Some(&dispute.id.to_string()),
        Some(serde_json::json!({"kb_entry_id": body.kb_entry_id, "reason": &body.reason})),
        get_ip(&req).as_deref(), get_user_agent(&req).as_deref(),
    ).await?;

    Ok(HttpResponse::Created().json(dispute))
}

/// GET /api/disputes
pub async fn list_disputes(
    pool: web::Data<PgPool>,
    query: web::Query<ListDisputesQuery>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin, UserRole::Reviewer, UserRole::DepartmentManager])?;
    apply_rate_limit(pool.get_ref(), auth.user_id).await?;

    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    let disputes = dispute_db::list_disputes(
        pool.get_ref(),
        query.status.as_ref(),
        limit,
        offset,
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(disputes))
}

/// GET /api/disputes/{id}
pub async fn get_dispute(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    apply_rate_limit(pool.get_ref(), auth.user_id).await?;

    let id = path.into_inner();
    let dispute = dispute_db::get_dispute(pool.get_ref(), id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Dispute not found".into()))?;

    // Object-level auth: owner or privileged role
    if dispute.disputed_by != auth.user_id {
        require_role(&auth, &[UserRole::OperationsAdmin, UserRole::Reviewer, UserRole::DepartmentManager])?;
    }

    let entry = kb_db::get_entry(pool.get_ref(), dispute.kb_entry_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let kb_entry_name = entry.map(|e| e.item_name).unwrap_or_else(|| "Unknown".into());

    Ok(HttpResponse::Ok().json(DisputeDetail { dispute, kb_entry_name }))
}

/// PUT /api/disputes/{id}/resolve
pub async fn resolve_dispute(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    body: web::Json<ResolveDisputeRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin, UserRole::Reviewer])?;
    apply_rate_limit(pool.get_ref(), auth.user_id).await?;

    let id = path.into_inner();
    let dispute = dispute_db::update_dispute_status(
        pool.get_ref(),
        id,
        &body.status,
        body.resolution_notes.as_deref(),
        auth.user_id,
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    audit_action(
        pool.get_ref(), &auth, "dispute_resolved", Some("classification_dispute"),
        Some(&id.to_string()),
        Some(serde_json::json!({"status": &body.status, "resolution_notes": &body.resolution_notes})),
        get_ip(&req).as_deref(), get_user_agent(&req).as_deref(),
    ).await?;

    Ok(HttpResponse::Ok().json(dispute))
}

pub fn dispute_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/disputes")
            .route("", web::post().to(create_dispute))
            .route("", web::get().to(list_disputes))
            .route("/{id}", web::get().to(get_dispute))
            .route("/{id}/resolve", web::put().to(resolve_dispute)),
    );
}
