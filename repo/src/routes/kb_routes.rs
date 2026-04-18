use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::knowledge_base as kb_db;
use crate::errors::{map_sqlx_unique_violation, AppError};
use crate::images::storage;
use crate::middleware::auth_middleware::{authenticate_request, require_role};
use crate::middleware::audit_middleware::audit_action;
use crate::middleware::rate_limit_middleware::apply_rate_limit;
use crate::models::*;

fn get_ip(req: &HttpRequest) -> Option<String> {
    req.peer_addr().map(|a| a.ip().to_string())
}

fn get_user_agent(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("User-Agent")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
}

/// Build image URLs for search results
fn build_image_list(images: &[kb_db::VersionImageRow]) -> Vec<KbSearchResultImage> {
    images
        .iter()
        .map(|img| KbSearchResultImage {
            image_id: img.id,
            file_name: img.file_name.clone(),
            url: format!("/api/kb/images/{}", img.id),
            caption: img.caption.clone(),
        })
        .collect()
}

// ── Search ──────────────────────────────────────────────────

/// GET /api/kb/search?q=...&region=...&category_id=...
pub async fn search(
    pool: web::Data<PgPool>,
    query: web::Query<KbSearchQuery>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    if query.q.trim().is_empty() {
        return Err(AppError::BadRequest("Search query cannot be empty".to_string()));
    }

    let config = kb_db::get_search_config(pool.get_ref())
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let (rows, total) = kb_db::fuzzy_search(pool.get_ref(), &query, &config)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Enrich results with images
    let mut results = Vec::new();
    for row in rows {
        // Get current version's images
        let version = kb_db::get_version_by_number(
            pool.get_ref(),
            row.entry_id,
            row.current_version,
        )
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let images = if let Some(ref v) = version {
            let img_rows = kb_db::get_version_images(pool.get_ref(), v.id)
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;
            build_image_list(&img_rows)
        } else {
            Vec::new()
        };

        results.push(KbSearchResult {
            entry_id: row.entry_id,
            item_name: row.item_name,
            matched_alias: row.matched_alias,
            match_type: row.match_type,
            score: row.score as f64,
            region: row.region,
            category_name: row.category_name,
            current_version: row.current_version,
            disposal_category: row.disposal_category,
            disposal_instructions: row.disposal_instructions,
            special_handling: row.special_handling,
            contamination_notes: row.contamination_notes,
            rule_source: row.rule_source,
            effective_date: row.effective_date,
            images,
        });
    }

    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(config.max_results as i64);

    Ok(HttpResponse::Ok().json(KbSearchResponse {
        results,
        total,
        page,
        page_size,
        query: query.q.clone(),
    }))
}

// ── Entry CRUD ──────────────────────────────────────────────

/// POST /api/kb/entries
pub async fn create_entry(
    pool: web::Data<PgPool>,
    body: web::Json<CreateEntryRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth_user, &[UserRole::OperationsAdmin, UserRole::Reviewer])?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    let region = body.region.as_deref().unwrap_or("default");
    let effective_date = body.effective_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

    // Create entry
    let entry = kb_db::create_entry(
        pool.get_ref(),
        &body.item_name,
        body.category_id,
        region,
        Some(auth_user.user_id),
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Create version 1
    let version = kb_db::create_version(
        pool.get_ref(),
        entry.id,
        1,
        &body.item_name,
        &body.disposal_category,
        &body.disposal_instructions,
        body.special_handling.as_deref(),
        body.contamination_notes.as_deref(),
        region,
        body.rule_source.as_deref(),
        effective_date,
        Some("Initial version"),
        Some(auth_user.user_id),
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Set aliases
    if let Some(ref aliases) = body.aliases {
        kb_db::set_aliases(pool.get_ref(), entry.id, aliases)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    // Link images
    if let Some(ref image_ids) = body.image_ids {
        kb_db::link_images_to_version(pool.get_ref(), version.id, image_ids)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    audit_action(
        pool.get_ref(),
        &auth_user,
        "kb_entry_created",
        Some("kb_entry"),
        Some(&entry.id.to_string()),
        Some(serde_json::json!({"item_name": body.item_name})),
        get_ip(&req).as_deref(),
        get_user_agent(&req).as_deref(),
    )
    .await?;

    Ok(HttpResponse::Created().json(serde_json::json!({
        "entry": entry,
        "version": version,
    })))
}

/// GET /api/kb/entries/{id}
pub async fn get_entry(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    let entry_id = path.into_inner();
    let entry = kb_db::get_entry(pool.get_ref(), entry_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Knowledge base entry not found".to_string()))?;

    let version = kb_db::get_current_version(pool.get_ref(), entry_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let aliases = kb_db::get_aliases(pool.get_ref(), entry_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let images = if let Some(ref v) = version {
        let img_rows = kb_db::get_version_images(pool.get_ref(), v.id)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        build_image_list(&img_rows)
    } else {
        Vec::new()
    };

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "entry": entry,
        "version": version,
        "aliases": aliases,
        "images": images,
    })))
}

/// PUT /api/kb/entries/{id} — creates a new version
pub async fn update_entry(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    body: web::Json<UpdateEntryRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth_user, &[UserRole::OperationsAdmin, UserRole::Reviewer])?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    let entry_id = path.into_inner();
    let entry = kb_db::get_entry(pool.get_ref(), entry_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Knowledge base entry not found".to_string()))?;

    let new_version_number = entry.current_version + 1;
    let item_name = body.item_name.as_deref().unwrap_or(&entry.item_name);
    let region = body.region.as_deref().unwrap_or(&entry.region);
    let effective_date = body.effective_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

    // Create new version
    let version = kb_db::create_version(
        pool.get_ref(),
        entry_id,
        new_version_number,
        item_name,
        &body.disposal_category,
        &body.disposal_instructions,
        body.special_handling.as_deref(),
        body.contamination_notes.as_deref(),
        region,
        body.rule_source.as_deref(),
        effective_date,
        body.change_summary.as_deref(),
        Some(auth_user.user_id),
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Update head pointer
    let updated_entry = kb_db::update_entry_head(pool.get_ref(), entry_id, item_name, region, new_version_number)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Update aliases if provided
    if let Some(ref aliases) = body.aliases {
        kb_db::set_aliases(pool.get_ref(), entry_id, aliases)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    // Link images to new version
    if let Some(ref image_ids) = body.image_ids {
        kb_db::link_images_to_version(pool.get_ref(), version.id, image_ids)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    audit_action(
        pool.get_ref(),
        &auth_user,
        "kb_entry_updated",
        Some("kb_entry"),
        Some(&entry_id.to_string()),
        Some(serde_json::json!({
            "new_version": new_version_number,
            "change_summary": body.change_summary,
        })),
        get_ip(&req).as_deref(),
        get_user_agent(&req).as_deref(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "entry": updated_entry,
        "version": version,
    })))
}

/// DELETE /api/kb/entries/{id} — soft-deactivate
pub async fn deactivate_entry(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth_user, &[UserRole::OperationsAdmin])?;

    let entry_id = path.into_inner();
    kb_db::deactivate_entry(pool.get_ref(), entry_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    audit_action(
        pool.get_ref(),
        &auth_user,
        "kb_entry_deactivated",
        Some("kb_entry"),
        Some(&entry_id.to_string()),
        None,
        get_ip(&req).as_deref(),
        get_user_agent(&req).as_deref(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Entry deactivated"})))
}

// ── Version history ─────────────────────────────────────────

/// GET /api/kb/entries/{id}/versions
pub async fn get_versions(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    let entry_id = path.into_inner();
    let entry = kb_db::get_entry(pool.get_ref(), entry_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Entry not found".to_string()))?;

    let versions = kb_db::get_version_history(pool.get_ref(), entry_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let mut version_details = Vec::new();
    for v in versions {
        let img_rows = kb_db::get_version_images(pool.get_ref(), v.id)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let images = build_image_list(&img_rows);

        let aliases = kb_db::get_aliases(pool.get_ref(), entry_id)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        version_details.push(KbVersionDetail {
            version: v,
            images,
            aliases,
        });
    }

    Ok(HttpResponse::Ok().json(KbVersionHistoryResponse {
        entry_id,
        item_name: entry.item_name,
        versions: version_details,
    }))
}

// ── Categories ──────────────────────────────────────────────

/// GET /api/kb/categories
pub async fn list_categories(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    let categories = kb_db::list_categories(pool.get_ref())
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(categories))
}

/// POST /api/kb/categories
pub async fn create_category(
    pool: web::Data<PgPool>,
    body: web::Json<CreateCategoryRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth_user, &[UserRole::OperationsAdmin])?;

    let category = kb_db::create_category(
        pool.get_ref(),
        &body.name,
        body.description.as_deref(),
        body.parent_id,
        body.sort_order.unwrap_or(0),
    )
    .await
    .map_err(|e| map_sqlx_unique_violation(e, "Category name already exists"))?;

    audit_action(
        pool.get_ref(),
        &auth_user,
        "kb_category_created",
        Some("kb_category"),
        Some(&category.id.to_string()),
        Some(serde_json::json!({"name": body.name})),
        get_ip(&req).as_deref(),
        get_user_agent(&req).as_deref(),
    )
    .await?;

    Ok(HttpResponse::Created().json(category))
}

// ── Image upload & serve ────────────────────────────────────

/// POST /api/kb/images  (multipart form: file field)
pub async fn upload_image(
    pool: web::Data<PgPool>,
    mut payload: actix_web::web::Payload,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth_user, &[UserRole::OperationsAdmin, UserRole::Reviewer])?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    // Read the raw body
    use actix_web::web::BytesMut;
    use futures::StreamExt;

    let mut body = BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk.map_err(|e| AppError::BadRequest(format!("Payload error: {}", e)))?;
        if body.len() + chunk.len() > 5 * 1024 * 1024 + 4096 {
            return Err(AppError::BadRequest("Upload exceeds 5 MB limit".to_string()));
        }
        body.extend_from_slice(&chunk);
    }

    let content_type = req
        .headers()
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream");

    // For multipart, extract the image data; for direct upload, use body as-is
    let (file_name, mime_type, data) = if content_type.starts_with("multipart/") {
        // Simple boundary-based extraction for the first file part
        parse_multipart_image(&body, content_type)?
    } else {
        // Direct binary upload
        let mime = if content_type == "application/octet-stream" {
            // Detect from magic bytes
            if body.starts_with(&[0xFF, 0xD8, 0xFF]) {
                "image/jpeg"
            } else if body.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
                "image/png"
            } else {
                return Err(AppError::BadRequest("Cannot detect image type".to_string()));
            }
        } else {
            content_type
        };
        ("upload".to_string(), mime.to_string(), body.to_vec())
    };

    let image = storage::store_image(
        pool.get_ref(),
        &file_name,
        &data,
        &mime_type,
        Some(auth_user.user_id),
    )
    .await?;

    audit_action(
        pool.get_ref(),
        &auth_user,
        "kb_image_uploaded",
        Some("kb_image"),
        Some(&image.id.to_string()),
        Some(serde_json::json!({
            "file_name": file_name,
            "size": data.len(),
            "deduplicated": image.file_name != file_name,
        })),
        get_ip(&req).as_deref(),
        get_user_agent(&req).as_deref(),
    )
    .await?;

    Ok(HttpResponse::Created().json(image))
}

/// GET /api/kb/images/{id}
pub async fn serve_image(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    // Auth check - any authenticated user can view images
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    let image_id = path.into_inner();
    let (data, mime_type) = storage::read_image(pool.get_ref(), image_id).await?;

    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", mime_type))
        .insert_header(("Cache-Control", "max-age=86400"))
        .body(data))
}

// ── Search config ───────────────────────────────────────────

/// GET /api/kb/search-config
pub async fn get_search_config(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth_user, &[UserRole::OperationsAdmin])?;

    let config = kb_db::get_search_config(pool.get_ref())
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(config))
}

/// PUT /api/kb/search-config
pub async fn update_search_config(
    pool: web::Data<PgPool>,
    body: web::Json<UpdateSearchConfigRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth_user, &[UserRole::OperationsAdmin])?;

    let current = kb_db::get_search_config(pool.get_ref())
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let updated = kb_db::update_search_config(
        pool.get_ref(),
        current.id,
        body.name_exact_weight.unwrap_or(current.name_exact_weight),
        body.name_prefix_weight.unwrap_or(current.name_prefix_weight),
        body.name_fuzzy_weight.unwrap_or(current.name_fuzzy_weight),
        body.alias_exact_weight.unwrap_or(current.alias_exact_weight),
        body.alias_fuzzy_weight.unwrap_or(current.alias_fuzzy_weight),
        body.category_boost.unwrap_or(current.category_boost),
        body.region_boost.unwrap_or(current.region_boost),
        body.recency_boost.unwrap_or(current.recency_boost),
        body.fuzzy_threshold.unwrap_or(current.fuzzy_threshold),
        body.max_results.unwrap_or(current.max_results),
        Some(auth_user.user_id),
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    audit_action(
        pool.get_ref(),
        &auth_user,
        "kb_search_config_updated",
        Some("kb_search_config"),
        Some(&current.id.to_string()),
        Some(serde_json::json!(body.into_inner())),
        get_ip(&req).as_deref(),
        get_user_agent(&req).as_deref(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(updated))
}

// ── Helpers ─────────────────────────────────────────────────

/// Simple multipart parser to extract the first file part
fn parse_multipart_image(
    body: &[u8],
    content_type: &str,
) -> Result<(String, String, Vec<u8>), AppError> {
    // Extract boundary
    let boundary = content_type
        .split("boundary=")
        .nth(1)
        .ok_or(AppError::BadRequest("Missing multipart boundary".to_string()))?
        .trim_matches('"');

    let boundary_marker = format!("--{}", boundary);
    let body_str = String::from_utf8_lossy(body);

    // Find the first part with a filename
    let parts: Vec<&str> = body_str.split(&boundary_marker).collect();

    for part in parts {
        if part.contains("filename=") {
            // Extract filename
            let file_name = part
                .split("filename=")
                .nth(1)
                .and_then(|s| s.split('"').nth(1))
                .unwrap_or("upload")
                .to_string();

            // Extract content type of the part
            let part_mime = part
                .split("Content-Type:")
                .nth(1)
                .and_then(|s| s.split('\r').next())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "application/octet-stream".to_string());

            // Find the blank line separating headers from content
            if let Some(header_end) = part.find("\r\n\r\n") {
                let content_start = header_end + 4;
                let content = &part.as_bytes()[content_start..];
                // Trim trailing \r\n
                let content = if content.ends_with(b"\r\n") {
                    &content[..content.len() - 2]
                } else {
                    content
                };

                return Ok((file_name, part_mime, content.to_vec()));
            }
        }
    }

    Err(AppError::BadRequest("No file found in multipart upload".to_string()))
}

// ── Route config ────────────────────────────────────────────

pub fn kb_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/kb")
            .route("/search", web::get().to(search))
            .route("/entries", web::post().to(create_entry))
            .route("/entries/{id}", web::get().to(get_entry))
            .route("/entries/{id}", web::put().to(update_entry))
            .route("/entries/{id}", web::delete().to(deactivate_entry))
            .route("/entries/{id}/versions", web::get().to(get_versions))
            .route("/categories", web::get().to(list_categories))
            .route("/categories", web::post().to(create_category))
            .route("/images", web::post().to(upload_image))
            .route("/images/{id}", web::get().to(serve_image))
            .route("/search-config", web::get().to(get_search_config))
            .route("/search-config", web::put().to(update_search_config)),
    );
}
