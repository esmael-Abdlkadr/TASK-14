use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::bulk_data as bulk_db;
use crate::dedup::{entity_resolution, import_processor};
use crate::errors::AppError;
use crate::middleware::auth_middleware::{authenticate_request, require_role};
use crate::middleware::audit_middleware::audit_action;
use crate::middleware::rate_limit_middleware::apply_rate_limit;
use crate::models::*;

fn get_ip(req: &HttpRequest) -> Option<String> { req.peer_addr().map(|a| a.ip().to_string()) }
fn get_ua(req: &HttpRequest) -> Option<String> {
    req.headers().get("User-Agent").and_then(|v| v.to_str().ok()).map(String::from)
}

// ═══════════════════════════════════════════════════════════
// IMPORT
// ═══════════════════════════════════════════════════════════

/// POST /api/bulk/import — Start an import job and validate rows
pub async fn start_import(
    pool: web::Data<PgPool>, body: web::Json<StartImportRequest>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;
    apply_rate_limit(pool.get_ref(), auth.user_id).await?;

    if body.rows.is_empty() {
        return Err(AppError::BadRequest("No rows to import".into()));
    }

    let job = bulk_db::create_import_job(
        pool.get_ref(), &body.name, &body.entity_type, None,
        body.rows.len() as i32, auth.user_id,
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Create import rows
    let mut import_rows = Vec::new();
    for (i, row_data) in body.rows.iter().enumerate() {
        let row = bulk_db::create_import_row(pool.get_ref(), job.id, i as i32 + 1, row_data)
            .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        import_rows.push(row);
    }

    // Validate
    let result = import_processor::validate_import(pool.get_ref(), &job, &import_rows).await?;

    audit_action(pool.get_ref(), &auth, "import_started", Some("import_job"),
        Some(&job.id.to_string()),
        Some(serde_json::json!({"entity_type": &body.entity_type, "rows": body.rows.len()})),
        get_ip(&req).as_deref(), get_ua(&req).as_deref()).await?;

    Ok(HttpResponse::Created().json(result))
}

/// POST /api/bulk/import/{id}/execute — Execute validated import
pub async fn execute_import(
    pool: web::Data<PgPool>, path: web::Path<Uuid>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;

    let job_id = path.into_inner();
    let job = bulk_db::get_import_job(pool.get_ref(), job_id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Import job not found".into()))?;

    if job.status != ImportJobStatus::Validated {
        return Err(AppError::BadRequest("Import must be validated before execution".into()));
    }

    let result = import_processor::execute_import(pool.get_ref(), job_id, auth.user_id).await?;

    audit_action(pool.get_ref(), &auth, "import_executed", Some("import_job"),
        Some(&job_id.to_string()),
        Some(serde_json::json!({"imported": result.imported_rows})),
        get_ip(&req).as_deref(), get_ua(&req).as_deref()).await?;

    Ok(HttpResponse::Ok().json(result))
}

/// GET /api/bulk/import — List import jobs
pub async fn list_imports(
    pool: web::Data<PgPool>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;
    let jobs = bulk_db::list_import_jobs(pool.get_ref(), 50, 0)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(jobs))
}

/// GET /api/bulk/import/{id} — Get import job detail with rows
pub async fn get_import(
    pool: web::Data<PgPool>, path: web::Path<Uuid>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;
    let id = path.into_inner();
    let job = bulk_db::get_import_job(pool.get_ref(), id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Import job not found".into()))?;
    let rows = bulk_db::get_import_rows(pool.get_ref(), id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"job": job, "rows": rows})))
}

// ═══════════════════════════════════════════════════════════
// EXPORT
// ═══════════════════════════════════════════════════════════

/// POST /api/bulk/export — Export entity data as CSV
pub async fn export_data(
    pool: web::Data<PgPool>, body: web::Json<ExportDataRequest>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;

    // Step-up for exports
    let stepup_action = "export_csv";
    let has_stepup = crate::risk::stepup::check_stepup(pool.get_ref(), auth.session_id, stepup_action).await?;
    if !has_stepup {
        return Err(AppError::StepUpRequired(stepup_action.into()));
    }

    let (data, content_type, filename) = match body.entity_type.as_str() {
        "kb_entry" => export_kb_entries(pool.get_ref(), &body.format).await?,
        _ => return Err(AppError::BadRequest(format!("Unsupported entity type: {}", body.entity_type))),
    };

    audit_action(pool.get_ref(), &auth, "data_exported", Some("bulk_export"),
        None, Some(serde_json::json!({"entity_type": &body.entity_type, "format": &body.format})),
        get_ip(&req).as_deref(), get_ua(&req).as_deref()).await?;

    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", content_type))
        .insert_header(("Content-Disposition", format!("attachment; filename=\"{}\"", filename)))
        .body(data))
}

async fn export_kb_entries(pool: &PgPool, format: &str) -> Result<(Vec<u8>, &'static str, String), AppError> {
    let entries: Vec<crate::models::KbEntry> = sqlx::query_as(
        "SELECT * FROM kb_entries WHERE is_active=TRUE ORDER BY item_name",
    ).fetch_all(pool).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let mut w = csv::Writer::from_writer(Vec::new());
    w.write_record(&["ID", "Item Name", "Region", "Version", "Created At"])
        .map_err(|e| AppError::InternalError(e.to_string()))?;
    for e in &entries {
        w.write_record(&[
            &e.id.to_string(), &e.item_name, &e.region,
            &e.current_version.to_string(), &e.created_at.to_rfc3339(),
        ]).map_err(|e| AppError::InternalError(e.to_string()))?;
    }
    let data = w.into_inner().map_err(|e| AppError::InternalError(e.to_string()))?;
    Ok((data, "text/csv", "kb_entries_export.csv".into()))
}

// ═══════════════════════════════════════════════════════════
// CHANGE HISTORY
// ═══════════════════════════════════════════════════════════

/// GET /api/bulk/changes
pub async fn get_change_history(
    pool: web::Data<PgPool>, query: web::Query<ChangeHistoryQuery>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;
    apply_rate_limit(pool.get_ref(), auth.user_id).await?;

    let page = query.page.unwrap_or(1).max(1);
    let ps = query.page_size.unwrap_or(50).min(200);

    let changes = bulk_db::get_change_history(
        pool.get_ref(), query.entity_type.as_deref(), query.entity_id,
        ps, (page - 1) * ps,
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let total = bulk_db::count_changes(pool.get_ref(), query.entity_type.as_deref(), query.entity_id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ChangeHistoryResponse { changes, total, page, page_size: ps }))
}

/// POST /api/bulk/changes/{id}/revert
pub async fn revert_change(
    pool: web::Data<PgPool>, path: web::Path<Uuid>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;

    let change = bulk_db::revert_change(pool.get_ref(), path.into_inner(), auth.user_id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    audit_action(pool.get_ref(), &auth, "change_reverted", Some("data_change"),
        Some(&change.id.to_string()), None,
        get_ip(&req).as_deref(), get_ua(&req).as_deref()).await?;

    Ok(HttpResponse::Ok().json(change))
}

// ═══════════════════════════════════════════════════════════
// DUPLICATES
// ═══════════════════════════════════════════════════════════

/// GET /api/bulk/duplicates
pub async fn list_duplicates(
    pool: web::Data<PgPool>, query: web::Query<DuplicateQuery>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;
    apply_rate_limit(pool.get_ref(), auth.user_id).await?;

    let page = query.page.unwrap_or(1).max(1);
    let ps = query.page_size.unwrap_or(20).min(100);

    let flags = bulk_db::list_duplicate_flags(
        pool.get_ref(), query.entity_type.as_deref(), query.status.as_ref(),
        ps, (page - 1) * ps,
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(flags))
}

/// PUT /api/bulk/duplicates/{id}/resolve
pub async fn resolve_duplicate(
    pool: web::Data<PgPool>, path: web::Path<Uuid>,
    body: web::Json<ResolveDuplicateRequest>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;
    let dup_id = path.into_inner();
    let flag = bulk_db::resolve_duplicate(pool.get_ref(), dup_id, &body.status, auth.user_id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    audit_action(pool.get_ref(), &auth, "duplicate_resolved", Some("duplicate_flag"),
        Some(&dup_id.to_string()), Some(serde_json::json!({"status": &body.status})),
        get_ip(&req).as_deref(), get_ua(&req).as_deref()).await?;

    Ok(HttpResponse::Ok().json(flag))
}

// ═══════════════════════════════════════════════════════════
// MERGE REQUESTS
// ═══════════════════════════════════════════════════════════

/// POST /api/bulk/merges
pub async fn create_merge_request(
    pool: web::Data<PgPool>, body: web::Json<CreateMergeRequest>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;
    apply_rate_limit(pool.get_ref(), auth.user_id).await?;

    let mr = bulk_db::create_merge_request(
        pool.get_ref(), &body.entity_type, body.source_id, body.target_id,
        body.duplicate_flag_id, auth.user_id,
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Auto-detect conflicts
    let source_data = fetch_entity_data(pool.get_ref(), &body.entity_type, body.source_id).await?;
    let target_data = fetch_entity_data(pool.get_ref(), &body.entity_type, body.target_id).await?;
    let conflicts = entity_resolution::detect_conflicts(&source_data, &target_data);

    for (field, sv, tv) in &conflicts {
        bulk_db::create_merge_conflict(pool.get_ref(), mr.id, field, Some(sv), Some(tv))
            .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    let merge_conflicts = bulk_db::get_merge_conflicts(pool.get_ref(), mr.id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    audit_action(pool.get_ref(), &auth, "merge_requested", Some("merge_request"),
        Some(&mr.id.to_string()),
        Some(serde_json::json!({"conflicts": conflicts.len()})),
        get_ip(&req).as_deref(), get_ua(&req).as_deref()).await?;

    Ok(HttpResponse::Created().json(MergeRequestDetail { request: mr, conflicts: merge_conflicts }))
}

/// GET /api/bulk/merges
pub async fn list_merge_requests(
    pool: web::Data<PgPool>, query: web::Query<serde_json::Value>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;
    let merges = bulk_db::list_merge_requests(pool.get_ref(), None, 50, 0)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(merges))
}

/// GET /api/bulk/merges/{id}
pub async fn get_merge_request(
    pool: web::Data<PgPool>, path: web::Path<Uuid>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;
    let id = path.into_inner();
    let mr = bulk_db::get_merge_request(pool.get_ref(), id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Merge request not found".into()))?;
    let conflicts = bulk_db::get_merge_conflicts(pool.get_ref(), id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(MergeRequestDetail { request: mr, conflicts }))
}

/// PUT /api/bulk/merges/{id}/conflicts/{cid}
pub async fn resolve_conflict(
    pool: web::Data<PgPool>, path: web::Path<(Uuid, Uuid)>,
    body: web::Json<ResolveConflictRequest>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;
    let (_merge_id, conflict_id) = path.into_inner();
    let conflict = bulk_db::resolve_conflict(
        pool.get_ref(), conflict_id, &body.resolution,
        body.custom_value.as_ref(), auth.user_id,
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    audit_action(pool.get_ref(), &auth, "conflict_resolved", Some("merge_conflict"),
        Some(&conflict_id.to_string()), Some(serde_json::json!({"resolution": &body.resolution})),
        get_ip(&req).as_deref(), get_ua(&req).as_deref()).await?;

    Ok(HttpResponse::Ok().json(conflict))
}

/// PUT /api/bulk/merges/{id}/review — Manager approval
pub async fn review_merge(
    pool: web::Data<PgPool>, path: web::Path<Uuid>,
    body: web::Json<ReviewMergeRequest>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::DepartmentManager, UserRole::OperationsAdmin])?;

    let id = path.into_inner();

    // Verify all conflicts resolved before approval
    if body.status == MergeRequestStatus::Approved || body.status == MergeRequestStatus::Applied {
        let conflicts = bulk_db::get_merge_conflicts(pool.get_ref(), id)
            .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let unresolved = conflicts.iter().filter(|c| c.resolution.is_none()).count();
        if unresolved > 0 {
            return Err(AppError::BadRequest(format!(
                "{} conflicts must be resolved before approval", unresolved
            )));
        }

        // Build provenance
        let mr = bulk_db::get_merge_request(pool.get_ref(), id)
            .await.map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or(AppError::NotFound("Merge request not found".into()))?;

        let provenance = entity_resolution::build_provenance(
            &[], &conflicts, mr.source_id, mr.target_id,
        );

        let updated = bulk_db::review_merge_request(
            pool.get_ref(), id, &body.status, auth.user_id,
            body.review_notes.as_deref(), None, Some(&provenance),
        ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        audit_action(pool.get_ref(), &auth, "merge_reviewed", Some("merge_request"),
            Some(&id.to_string()),
            Some(serde_json::json!({"status": &body.status})),
            get_ip(&req).as_deref(), get_ua(&req).as_deref()).await?;

        return Ok(HttpResponse::Ok().json(updated));
    }

    let updated = bulk_db::review_merge_request(
        pool.get_ref(), id, &body.status, auth.user_id,
        body.review_notes.as_deref(), None, None,
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    audit_action(pool.get_ref(), &auth, "merge_reviewed", Some("merge_request"),
        Some(&id.to_string()),
        Some(serde_json::json!({"status": &body.status, "review_notes": &body.review_notes})),
        get_ip(&req).as_deref(), get_ua(&req).as_deref()).await?;

    Ok(HttpResponse::Ok().json(updated))
}

// ═══════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════

async fn fetch_entity_data(pool: &PgPool, entity_type: &str, entity_id: Uuid) -> Result<serde_json::Value, AppError> {
    match entity_type {
        "kb_entry" => {
            let entry = crate::db::knowledge_base::get_entry(pool, entity_id)
                .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
            match entry {
                Some(e) => Ok(serde_json::json!({
                    "item_name": e.item_name, "region": e.region,
                    "category_id": e.category_id, "current_version": e.current_version,
                })),
                None => Ok(serde_json::json!({})),
            }
        }
        _ => Ok(serde_json::json!({})),
    }
}

// ═══════════════════════════════════════════════════════════
// ROUTE CONFIG
// ═══════════════════════════════════════════════════════════

pub fn bulk_data_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/bulk")
            // Import
            .route("/import", web::post().to(start_import))
            .route("/import", web::get().to(list_imports))
            .route("/import/{id}", web::get().to(get_import))
            .route("/import/{id}/execute", web::post().to(execute_import))
            // Export
            .route("/export", web::post().to(export_data))
            // Change history
            .route("/changes", web::get().to(get_change_history))
            .route("/changes/{id}/revert", web::post().to(revert_change))
            // Duplicates
            .route("/duplicates", web::get().to(list_duplicates))
            .route("/duplicates/{id}/resolve", web::put().to(resolve_duplicate))
            // Merge requests
            .route("/merges", web::post().to(create_merge_request))
            .route("/merges", web::get().to(list_merge_requests))
            .route("/merges/{id}", web::get().to(get_merge_request))
            .route("/merges/{id}/conflicts/{cid}", web::put().to(resolve_conflict))
            .route("/merges/{id}/review", web::put().to(review_merge)),
    );
}
