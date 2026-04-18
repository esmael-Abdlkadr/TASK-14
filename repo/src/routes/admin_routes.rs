use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{Duration, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::admin::reports;
use crate::db::admin as admin_db;
use crate::errors::{map_sqlx_unique_violation, AppError};
use crate::middleware::auth_middleware::{authenticate_request, require_role};
use crate::middleware::audit_middleware::audit_action;
use crate::middleware::rate_limit_middleware::apply_rate_limit;
use crate::models::*;

fn get_ip(req: &HttpRequest) -> Option<String> {
    req.peer_addr().map(|a| a.ip().to_string())
}
fn get_ua(req: &HttpRequest) -> Option<String> {
    req.headers().get("User-Agent").and_then(|v| v.to_str().ok()).map(String::from)
}

// ═══════════════════════════════════════════════════════════
// DASHBOARD KPIs
// ═══════════════════════════════════════════════════════════

/// GET /api/admin/dashboard
pub async fn get_dashboard(
    pool: web::Data<PgPool>, query: web::Query<KpiQuery>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;
    apply_rate_limit(pool.get_ref(), auth.user_id).await?;

    let to = query.to_date.unwrap_or_else(|| Utc::now().date_naive());
    let from = query.from_date.unwrap_or_else(|| to - Duration::days(30));

    let kpis = admin_db::get_dashboard_kpis(pool.get_ref(), from, to)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(kpis))
}

/// GET /api/admin/kpi/trend?metric=...&limit=...
pub async fn get_kpi_trend(
    pool: web::Data<PgPool>, query: web::Query<serde_json::Value>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;

    let metric = query.get("metric").and_then(|v| v.as_str()).unwrap_or("sorting_conversion_rate");
    let limit = query.get("limit").and_then(|v| v.as_i64()).unwrap_or(12);

    let trend = admin_db::get_kpi_trend(pool.get_ref(), metric, limit)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(trend))
}

// ═══════════════════════════════════════════════════════════
// OVERVIEWS
// ═══════════════════════════════════════════════════════════

/// GET /api/admin/overview/users
pub async fn user_overview(
    pool: web::Data<PgPool>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;
    let overview = admin_db::get_user_overview(pool.get_ref())
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(overview))
}

/// GET /api/admin/overview/items
pub async fn item_overview(
    pool: web::Data<PgPool>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;
    let overview = admin_db::get_item_overview(pool.get_ref())
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(overview))
}

/// GET /api/admin/overview/workorders
pub async fn workorder_overview(
    pool: web::Data<PgPool>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;
    let overview = admin_db::get_work_order_overview(pool.get_ref())
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(overview))
}

// ═══════════════════════════════════════════════════════════
// CAMPAIGNS
// ═══════════════════════════════════════════════════════════

/// POST /api/admin/campaigns
pub async fn create_campaign(
    pool: web::Data<PgPool>, body: web::Json<CreateCampaignRequest>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;
    apply_rate_limit(pool.get_ref(), auth.user_id).await?;

    if body.end_date <= body.start_date {
        return Err(AppError::BadRequest("End date must be after start date".into()));
    }

    let campaign = admin_db::create_campaign(
        pool.get_ref(), &body.name, body.description.as_deref(),
        body.start_date, body.end_date, body.target_region.as_deref(),
        body.target_audience.as_deref(), body.goals.as_ref(), Some(auth.user_id),
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    if let Some(ref tags) = body.tag_ids {
        admin_db::set_campaign_tags(pool.get_ref(), campaign.id, tags)
            .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    let tags = admin_db::get_campaign_tags(pool.get_ref(), campaign.id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    audit_action(pool.get_ref(), &auth, "campaign_created", Some("campaign"),
        Some(&campaign.id.to_string()), Some(serde_json::json!({"name": &body.name})),
        get_ip(&req).as_deref(), get_ua(&req).as_deref()).await?;

    Ok(HttpResponse::Created().json(CampaignDetail { campaign, tags }))
}

/// GET /api/admin/campaigns
pub async fn list_campaigns(
    pool: web::Data<PgPool>, query: web::Query<CampaignQuery>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;

    let page = query.page.unwrap_or(1).max(1);
    let ps = query.page_size.unwrap_or(20).min(100);

    let campaigns = admin_db::list_campaigns(
        pool.get_ref(), query.status.as_ref(), ps, (page - 1) * ps,
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(campaigns))
}

/// GET /api/admin/campaigns/{id}
pub async fn get_campaign(
    pool: web::Data<PgPool>, path: web::Path<Uuid>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;

    let id = path.into_inner();
    let campaign = admin_db::get_campaign(pool.get_ref(), id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Campaign not found".into()))?;
    let tags = admin_db::get_campaign_tags(pool.get_ref(), id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(CampaignDetail { campaign, tags }))
}

/// PUT /api/admin/campaigns/{id}
pub async fn update_campaign(
    pool: web::Data<PgPool>, path: web::Path<Uuid>,
    body: web::Json<UpdateCampaignRequest>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;

    let id = path.into_inner();
    let existing = admin_db::get_campaign(pool.get_ref(), id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Campaign not found".into()))?;

    let campaign = admin_db::update_campaign(
        pool.get_ref(), id,
        body.name.as_deref().unwrap_or(&existing.name),
        body.description.as_deref().or(existing.description.as_deref()),
        body.status.as_ref().unwrap_or(&existing.status),
        body.start_date.unwrap_or(existing.start_date),
        body.end_date.unwrap_or(existing.end_date),
        body.target_region.as_deref().or(existing.target_region.as_deref()),
        body.target_audience.as_deref().or(existing.target_audience.as_deref()),
        body.goals.as_ref().or(existing.goals.as_ref()),
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    if let Some(ref tags) = body.tag_ids {
        admin_db::set_campaign_tags(pool.get_ref(), id, tags)
            .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    let tags = admin_db::get_campaign_tags(pool.get_ref(), id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    audit_action(pool.get_ref(), &auth, "campaign_updated", Some("campaign"),
        Some(&id.to_string()), None, get_ip(&req).as_deref(), get_ua(&req).as_deref()).await?;

    Ok(HttpResponse::Ok().json(CampaignDetail { campaign, tags }))
}

// ═══════════════════════════════════════════════════════════
// TAGS
// ═══════════════════════════════════════════════════════════

/// POST /api/admin/tags
pub async fn create_tag(
    pool: web::Data<PgPool>, body: web::Json<CreateTagRequest>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;
    let tag = admin_db::create_tag(pool.get_ref(), &body.name, body.color.as_deref())
        .await
        .map_err(|e| map_sqlx_unique_violation(e, "Tag name already exists"))?;
    Ok(HttpResponse::Created().json(tag))
}

/// GET /api/admin/tags
pub async fn list_tags(
    pool: web::Data<PgPool>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    let tags = admin_db::list_tags(pool.get_ref())
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(tags))
}

/// DELETE /api/admin/tags/{id}
pub async fn delete_tag(
    pool: web::Data<PgPool>, path: web::Path<Uuid>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;
    admin_db::delete_tag(pool.get_ref(), path.into_inner())
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Tag deleted"})))
}

/// PUT /api/admin/categories/{id}/tags
pub async fn set_category_tags(
    pool: web::Data<PgPool>, path: web::Path<Uuid>,
    body: web::Json<Vec<Uuid>>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;
    admin_db::set_category_tags(pool.get_ref(), path.into_inner(), &body)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Category tags updated"})))
}

// ═══════════════════════════════════════════════════════════
// REPORTS
// ═══════════════════════════════════════════════════════════

/// POST /api/admin/reports/generate
pub async fn generate_report(
    pool: web::Data<PgPool>, body: web::Json<GenerateReportRequest>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;

    // Step-up required for exports
    let stepup_action = match body.format {
        ReportFormat::Csv => "export_csv",
        ReportFormat::Pdf => "export_pdf",
    };
    let has_stepup = crate::risk::stepup::check_stepup(pool.get_ref(), auth.session_id, stepup_action).await?;
    if !has_stepup {
        return Err(AppError::StepUpRequired(stepup_action.to_string()));
    }

    let to = body.to_date.unwrap_or_else(|| Utc::now().date_naive());
    let from = body.from_date.unwrap_or_else(|| to - Duration::days(30));

    let (data, content_type, filename) = match body.report_type.as_str() {
        "kpi_summary" => reports::generate_kpi_report(pool.get_ref(), from, to, &body.format).await?,
        "user_overview" => reports::generate_user_report(pool.get_ref(), &body.format).await?,
        "task_overview" => reports::generate_workorder_report(pool.get_ref(), &body.format).await?,
        "campaign_report" => reports::generate_campaign_report(pool.get_ref(), &body.format).await?,
        "audit_report" => {
            let audit_query = crate::models::AuditExportQuery {
                format: match body.format {
                    ReportFormat::Csv => ExportFormat::Csv,
                    ReportFormat::Pdf => ExportFormat::Pdf,
                },
                from_date: Some(from.and_hms_opt(0, 0, 0).unwrap().and_utc()),
                to_date: Some(to.and_hms_opt(23, 59, 59).unwrap().and_utc()),
                action: None,
            };
            let (d, ct, fn_) = crate::audit::export::export_audit_log(pool.get_ref(), &audit_query).await?;
            (d, ct, fn_.to_string())
        }
        _ => return Err(AppError::BadRequest(format!("Unknown report type: {}", body.report_type))),
    };

    audit_action(pool.get_ref(), &auth, "report_generated", Some("report"),
        None, Some(serde_json::json!({"type": &body.report_type, "format": &body.format})),
        get_ip(&req).as_deref(), get_ua(&req).as_deref()).await?;

    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", content_type))
        .insert_header(("Content-Disposition", format!("attachment; filename=\"{}\"", filename)))
        .body(data))
}

/// POST /api/admin/reports/configs
pub async fn save_report_config(
    pool: web::Data<PgPool>, body: web::Json<CreateReportConfigRequest>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;
    let params = body.parameters.as_ref().cloned().unwrap_or(serde_json::json!({}));
    let fmt = body.format.as_ref().unwrap_or(&ReportFormat::Csv);
    let config = admin_db::create_report_config(
        pool.get_ref(), &body.name, &body.report_type, &params, fmt, Some(auth.user_id),
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Created().json(config))
}

/// GET /api/admin/reports/configs
pub async fn list_report_configs(
    pool: web::Data<PgPool>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;
    let configs = admin_db::list_report_configs(pool.get_ref())
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(configs))
}

// ═══════════════════════════════════════════════════════════
// ROUTE CONFIG
// ═══════════════════════════════════════════════════════════

pub fn admin_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/admin")
            // Dashboard
            .route("/dashboard", web::get().to(get_dashboard))
            .route("/kpi/trend", web::get().to(get_kpi_trend))
            // Overviews
            .route("/overview/users", web::get().to(user_overview))
            .route("/overview/items", web::get().to(item_overview))
            .route("/overview/workorders", web::get().to(workorder_overview))
            // Campaigns
            .route("/campaigns", web::post().to(create_campaign))
            .route("/campaigns", web::get().to(list_campaigns))
            .route("/campaigns/{id}", web::get().to(get_campaign))
            .route("/campaigns/{id}", web::put().to(update_campaign))
            // Tags
            .route("/tags", web::post().to(create_tag))
            .route("/tags", web::get().to(list_tags))
            .route("/tags/{id}", web::delete().to(delete_tag))
            .route("/categories/{id}/tags", web::put().to(set_category_tags))
            // Reports
            .route("/reports/generate", web::post().to(generate_report))
            .route("/reports/configs", web::post().to(save_report_config))
            .route("/reports/configs", web::get().to(list_report_configs)),
    );
}
