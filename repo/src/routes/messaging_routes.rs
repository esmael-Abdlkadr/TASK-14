use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::messaging as msg_db;
use crate::errors::{map_sqlx_unique_violation, AppError};
use crate::messaging::{payload_export, trigger};
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
// TEMPLATES
// ═══════════════════════════════════════════════════════════

/// POST /api/messaging/templates
pub async fn create_template(
    pool: web::Data<PgPool>, body: web::Json<CreateNotifTemplateRequest>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;
    apply_rate_limit(pool.get_ref(), auth.user_id).await?;

    let tmpl = msg_db::create_template(
        pool.get_ref(), &body.name, body.description.as_deref(), &body.channel,
        body.subject_template.as_deref(), &body.body_template,
        body.sms_template.as_deref(), body.html_template.as_deref(),
        Some(auth.user_id),
    ).await.map_err(|e| map_sqlx_unique_violation(e, "Template name already exists"))?;

    let variables = if let Some(ref vars) = body.variables {
        msg_db::set_template_variables(pool.get_ref(), tmpl.id, vars)
            .await.map_err(|e| AppError::DatabaseError(e.to_string()))?
    } else { Vec::new() };

    audit_action(pool.get_ref(), &auth, "notification_template_created",
        Some("notification_template"), Some(&tmpl.id.to_string()),
        Some(serde_json::json!({"name": &body.name})),
        get_ip(&req).as_deref(), get_ua(&req).as_deref()).await?;

    Ok(HttpResponse::Created().json(TemplateWithVariables { template: tmpl, variables }))
}

/// GET /api/messaging/templates
pub async fn list_templates(
    pool: web::Data<PgPool>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;
    apply_rate_limit(pool.get_ref(), auth.user_id).await?;
    let templates = msg_db::list_templates(pool.get_ref())
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(templates))
}

/// GET /api/messaging/templates/{id}
pub async fn get_template(
    pool: web::Data<PgPool>, path: web::Path<Uuid>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;
    let id = path.into_inner();
    let tmpl = msg_db::get_template(pool.get_ref(), id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Template not found".into()))?;
    let variables = msg_db::get_template_variables(pool.get_ref(), id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(TemplateWithVariables { template: tmpl, variables }))
}

/// PUT /api/messaging/templates/{id}
pub async fn update_template(
    pool: web::Data<PgPool>, path: web::Path<Uuid>,
    body: web::Json<UpdateNotifTemplateRequest>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;

    let id = path.into_inner();
    let existing = msg_db::get_template(pool.get_ref(), id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Template not found".into()))?;

    let tmpl = msg_db::update_template(
        pool.get_ref(), id,
        body.name.as_deref().unwrap_or(&existing.name),
        body.description.as_deref().or(existing.description.as_deref()),
        body.channel.as_ref().unwrap_or(&existing.channel),
        body.subject_template.as_deref().or(existing.subject_template.as_deref()),
        body.body_template.as_deref().unwrap_or(&existing.body_template),
        body.sms_template.as_deref().or(existing.sms_template.as_deref()),
        body.html_template.as_deref().or(existing.html_template.as_deref()),
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(tmpl))
}

/// DELETE /api/messaging/templates/{id}
pub async fn deactivate_template(
    pool: web::Data<PgPool>, path: web::Path<Uuid>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;
    msg_db::deactivate_template(pool.get_ref(), path.into_inner())
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Template deactivated"})))
}

// ═══════════════════════════════════════════════════════════
// TRIGGER RULES
// ═══════════════════════════════════════════════════════════

/// POST /api/messaging/triggers
pub async fn create_trigger(
    pool: web::Data<PgPool>, body: web::Json<CreateTriggerRuleRequest>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;

    let rule = msg_db::create_trigger_rule(
        pool.get_ref(), &body.name, &body.event, body.template_id,
        body.channel.as_ref().unwrap_or(&NotificationChannel::InApp),
        body.conditions.as_ref(), body.target_role.as_deref(),
        body.priority.unwrap_or(0), Some(auth.user_id),
    ).await.map_err(|e| {
        if let Some(db) = e.as_database_error() {
            if db.code().is_some_and(|c| c == "23503") {
                return AppError::BadRequest("Referenced template does not exist".to_string());
            }
        }
        map_sqlx_unique_violation(e, "Trigger rule name already exists")
    })?;

    audit_action(pool.get_ref(), &auth, "trigger_rule_created",
        Some("trigger_rule"), Some(&rule.id.to_string()),
        Some(serde_json::json!({"event": &body.event})),
        get_ip(&req).as_deref(), get_ua(&req).as_deref()).await?;

    Ok(HttpResponse::Created().json(rule))
}

/// GET /api/messaging/triggers
pub async fn list_triggers(
    pool: web::Data<PgPool>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;
    let rules = msg_db::list_trigger_rules(pool.get_ref())
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(rules))
}

/// DELETE /api/messaging/triggers/{id}
pub async fn deactivate_trigger(
    pool: web::Data<PgPool>, path: web::Path<Uuid>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;
    msg_db::deactivate_trigger_rule(pool.get_ref(), path.into_inner())
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Trigger deactivated"})))
}

// ═══════════════════════════════════════════════════════════
// FIRE EVENTS
// ═══════════════════════════════════════════════════════════

/// POST /api/messaging/fire
pub async fn fire_event(
    pool: web::Data<PgPool>, body: web::Json<FireEventRequest>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;
    apply_rate_limit(pool.get_ref(), auth.user_id).await?;
    crate::risk::antibot::check_antibot(pool.get_ref(), auth.user_id, "fire_event").await?;

    let result = trigger::fire_event(
        pool.get_ref(), &body.event, &body.payload,
        body.recipient_user_id, body.reference_type.as_deref(), body.reference_id,
    ).await?;

    audit_action(pool.get_ref(), &auth, "fire_event", Some("messaging"),
        None, Some(serde_json::json!({"event": &body.event})),
        get_ip(&req).as_deref(), get_ua(&req).as_deref()).await?;

    Ok(HttpResponse::Ok().json(result))
}

// ═══════════════════════════════════════════════════════════
// NOTIFICATIONS (User inbox)
// ═══════════════════════════════════════════════════════════

/// GET /api/messaging/notifications
pub async fn get_notifications(
    pool: web::Data<PgPool>, query: web::Query<NotificationQuery>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    apply_rate_limit(pool.get_ref(), auth.user_id).await?;

    let page = query.page.unwrap_or(1).max(1);
    let ps = query.page_size.unwrap_or(20).min(100);

    let notifications = msg_db::get_notifications(
        pool.get_ref(), auth.user_id, query.status.as_ref(),
        query.channel.as_ref(), ps, (page - 1) * ps,
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let unread = msg_db::count_unread_notifications(pool.get_ref(), auth.user_id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let total = notifications.len() as i64 + (page - 1) * ps;

    Ok(HttpResponse::Ok().json(NotificationInbox {
        unread_count: unread, notifications, total, page, page_size: ps,
    }))
}

/// POST /api/messaging/notifications/{id}/read
pub async fn mark_read(
    pool: web::Data<PgPool>, path: web::Path<Uuid>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    msg_db::mark_notification_read(pool.get_ref(), path.into_inner(), auth.user_id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Marked as read"})))
}

/// POST /api/messaging/notifications/{id}/dismiss
pub async fn dismiss_notification(
    pool: web::Data<PgPool>, path: web::Path<Uuid>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    msg_db::dismiss_notification(pool.get_ref(), path.into_inner(), auth.user_id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Dismissed"})))
}

/// POST /api/messaging/notifications/read-all
pub async fn mark_all_read(
    pool: web::Data<PgPool>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    let count = msg_db::mark_all_notifications_read(pool.get_ref(), auth.user_id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"marked_read": count})))
}

// ═══════════════════════════════════════════════════════════
// PAYLOAD QUEUE
// ═══════════════════════════════════════════════════════════

/// GET /api/messaging/payloads
pub async fn get_payload_queue(
    pool: web::Data<PgPool>, query: web::Query<PayloadQueueQuery>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;

    let page = query.page.unwrap_or(1).max(1);
    let ps = query.page_size.unwrap_or(20).min(100);

    let payloads = msg_db::get_payload_queue(
        pool.get_ref(), query.status.as_ref(), query.channel.as_ref(), ps, (page - 1) * ps,
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let (queued, failed) = msg_db::count_payloads_by_status(pool.get_ref())
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let total = payloads.len() as i64 + (page - 1) * ps;

    Ok(HttpResponse::Ok().json(PayloadQueueResponse {
        payloads, total, page, page_size: ps, queued_count: queued, failed_count: failed,
    }))
}

/// POST /api/messaging/payloads/export
pub async fn export_payloads(
    pool: web::Data<PgPool>, body: web::Json<serde_json::Value>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;

    let channel_str = body.get("channel").and_then(|v| v.as_str()).unwrap_or("sms");
    let channel = match channel_str {
        "sms" => NotificationChannel::Sms,
        "email" => NotificationChannel::Email,
        "push" => NotificationChannel::Push,
        _ => return Err(AppError::BadRequest(format!("Invalid channel: {}", channel_str))),
    };

    let result = payload_export::export_queued_payloads(pool.get_ref(), &channel, Some(auth.user_id)).await?;

    audit_action(pool.get_ref(), &auth, "payloads_exported",
        Some("external_payload"), None,
        Some(serde_json::json!({"channel": channel_str, "count": result.count})),
        get_ip(&req).as_deref(), get_ua(&req).as_deref()).await?;

    Ok(HttpResponse::Ok().json(result))
}

/// POST /api/messaging/payloads/mark-delivered
pub async fn mark_delivered(
    pool: web::Data<PgPool>, body: web::Json<MarkDeliveredRequest>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;

    let count = payload_export::mark_batch_delivered(
        pool.get_ref(), &body.payload_ids, Some(auth.user_id),
    ).await?;

    audit_action(pool.get_ref(), &auth, "payloads_delivered",
        Some("external_payload"), None,
        Some(serde_json::json!({"count": count})),
        get_ip(&req).as_deref(), get_ua(&req).as_deref()).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"delivered": count})))
}

/// POST /api/messaging/payloads/mark-failed
pub async fn mark_failed(
    pool: web::Data<PgPool>, body: web::Json<MarkFailedRequest>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;

    let payload = payload_export::mark_payload_failed(
        pool.get_ref(), body.payload_id, &body.error, Some(auth.user_id),
    ).await?;

    audit_action(pool.get_ref(), &auth, "payload_marked_failed",
        Some("external_payload"), Some(&body.payload_id.to_string()),
        Some(serde_json::json!({"error": &body.error, "status": &payload.status})),
        get_ip(&req).as_deref(), get_ua(&req).as_deref()).await?;

    Ok(HttpResponse::Ok().json(payload))
}

/// GET /api/messaging/payloads/{id}/log
pub async fn get_delivery_log(
    pool: web::Data<PgPool>, path: web::Path<Uuid>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;

    let log = msg_db::get_delivery_log(pool.get_ref(), path.into_inner())
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(log))
}

// ═══════════════════════════════════════════════════════════
// ROUTE CONFIG
// ═══════════════════════════════════════════════════════════

pub fn messaging_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/messaging")
            // Templates
            .route("/templates", web::post().to(create_template))
            .route("/templates", web::get().to(list_templates))
            .route("/templates/{id}", web::get().to(get_template))
            .route("/templates/{id}", web::put().to(update_template))
            .route("/templates/{id}", web::delete().to(deactivate_template))
            // Trigger rules
            .route("/triggers", web::post().to(create_trigger))
            .route("/triggers", web::get().to(list_triggers))
            .route("/triggers/{id}", web::delete().to(deactivate_trigger))
            // Fire events
            .route("/fire", web::post().to(fire_event))
            // Notifications
            .route("/notifications", web::get().to(get_notifications))
            .route("/notifications/read-all", web::post().to(mark_all_read))
            .route("/notifications/{id}/read", web::post().to(mark_read))
            .route("/notifications/{id}/dismiss", web::post().to(dismiss_notification))
            // Payload queue
            .route("/payloads", web::get().to(get_payload_queue))
            .route("/payloads/export", web::post().to(export_payloads))
            .route("/payloads/mark-delivered", web::post().to(mark_delivered))
            .route("/payloads/mark-failed", web::post().to(mark_failed))
            .route("/payloads/{id}/log", web::get().to(get_delivery_log)),
    );
}
