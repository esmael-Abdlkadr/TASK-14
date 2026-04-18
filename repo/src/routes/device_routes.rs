use actix_web::{web, HttpRequest, HttpResponse};
use sha2::{Sha256, Digest};
use sqlx::PgPool;

use crate::auth::password::verify_password;
use crate::db;
use crate::encryption::field_encryption::mask_field;
use crate::errors::AppError;
use crate::middleware::auth_middleware::authenticate_request;
use crate::middleware::audit_middleware::audit_action;
use crate::middleware::rate_limit_middleware::apply_rate_limit;
use crate::models::{BindDeviceRequest, TrustDeviceRequest};

fn get_ip(req: &HttpRequest) -> Option<String> {
    req.peer_addr().map(|a| a.ip().to_string())
}

fn get_user_agent(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("User-Agent")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
}

/// Compute deterministic SHA-256 hash of fingerprint for secure DB lookups
fn hash_fingerprint(fp: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(fp.as_bytes());
    hex::encode(hasher.finalize())
}

/// Build masked device JSON response (never expose raw fingerprint or ciphertext)
fn masked_device_json(d: &crate::models::DeviceBinding) -> serde_json::Value {
    // Show truncated hash as device identifier — never raw fingerprint
    let display_id = d.fingerprint_hash.as_deref()
        .map(|h| format!("{}...{}", &h[..8], &h[h.len()-4..]))
        .unwrap_or_else(|| mask_field(&d.device_fingerprint, 4, 4));

    serde_json::json!({
        "id": d.id,
        "user_id": d.user_id,
        "device_identifier": display_id,
        "device_name": d.device_name,
        "bound_at": d.bound_at,
        "last_seen_at": d.last_seen_at,
        "is_trusted": d.is_trusted,
    })
}

/// GET /api/devices
pub async fn list_devices(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    let devices = db::devices::list_user_devices(pool.get_ref(), auth_user.user_id).await?;
    let masked: Vec<serde_json::Value> = devices.iter().map(masked_device_json).collect();

    Ok(HttpResponse::Ok().json(masked))
}

/// POST /api/devices/bind
pub async fn bind_device(
    pool: web::Data<PgPool>,
    body: web::Json<BindDeviceRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    let fp_hash = hash_fingerprint(&body.device_fingerprint);

    // Encrypt fingerprint at rest — fail closed if encryption unavailable
    let encrypted_fp = crate::encryption::field_encryption::encrypt_field(
        pool.get_ref(), &body.device_fingerprint,
    ).await.map_err(|e| {
        log::error!("Encryption failed for device fingerprint: {}", e);
        AppError::InternalError("Cannot store sensitive data: encryption unavailable".to_string())
    })?;

    // No raw fingerprint stored — only hash + ciphertext
    let device = db::devices::bind_device(
        pool.get_ref(),
        auth_user.user_id,
        &fp_hash,
        &encrypted_fp,
        body.device_name.as_deref(),
    )
    .await?;

    audit_action(
        pool.get_ref(), &auth_user, "device_bound", Some("device"),
        Some(&device.id.to_string()), None,
        get_ip(&req).as_deref(), get_user_agent(&req).as_deref(),
    ).await?;

    Ok(HttpResponse::Created().json(masked_device_json(&device)))
}

/// POST /api/devices/trust (requires password re-entry)
pub async fn trust_device(
    pool: web::Data<PgPool>,
    body: web::Json<TrustDeviceRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    let user = db::users::find_by_id(pool.get_ref(), auth_user.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let valid = verify_password(&body.password, &user.password_hash)?;
    if !valid {
        return Err(AppError::StepUpFailed);
    }

    let device = db::devices::trust_device(pool.get_ref(), body.device_id, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Device not found".to_string()))?;

    audit_action(
        pool.get_ref(), &auth_user, "device_trusted", Some("device"),
        Some(&device.id.to_string()), None,
        get_ip(&req).as_deref(), get_user_agent(&req).as_deref(),
    ).await?;

    Ok(HttpResponse::Ok().json(masked_device_json(&device)))
}

/// DELETE /api/devices/{id}
pub async fn remove_device(
    pool: web::Data<PgPool>,
    path: web::Path<uuid::Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    let device_id = path.into_inner();
    let removed = db::devices::remove_device(pool.get_ref(), device_id, auth_user.user_id).await?;

    if !removed {
        return Err(AppError::NotFound("Device not found".to_string()));
    }

    audit_action(
        pool.get_ref(), &auth_user, "device_removed", Some("device"),
        Some(&device_id.to_string()), None,
        get_ip(&req).as_deref(), get_user_agent(&req).as_deref(),
    ).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Device removed"})))
}

pub fn device_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/devices")
            .route("", web::get().to(list_devices))
            .route("/bind", web::post().to(bind_device))
            .route("/trust", web::post().to(trust_device))
            .route("/{id}", web::delete().to(remove_device)),
    );
}
