use sqlx::PgPool;
use uuid::Uuid;

use crate::db::bulk_data as bulk_db;
use crate::dedup::{entity_resolution, fingerprint};
use crate::errors::AppError;
use crate::models::{
    ChangeOperation, ImportJob, ImportJobStatus, ImportRow, ImportRowStatus,
    ImportValidationResult,
};

/// Process a bulk import: validate rows, detect duplicates, flag conflicts.
/// Does NOT commit data — returns validation results for user review.
pub async fn validate_import(
    pool: &PgPool,
    job: &ImportJob,
    rows: &[ImportRow],
) -> Result<ImportValidationResult, AppError> {
    let mut duplicates_found = 0usize;
    let mut errors_found = 0usize;

    for row in rows {
        let (status, parsed, duplicate_of, error_msg, validation_errors) =
            validate_row(pool, &job.entity_type, &row.raw_data).await;

        if status == ImportRowStatus::Duplicate {
            duplicates_found += 1;
        }
        if status == ImportRowStatus::Error {
            errors_found += 1;
        }

        bulk_db::update_import_row(
            pool, row.id, &status, parsed.as_ref(),
            None, duplicate_of, error_msg.as_deref(),
            validation_errors.as_ref(),
        ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    // Update job status
    let updated_job = bulk_db::update_import_job_status(
        pool, job.id, &ImportJobStatus::Validated,
        rows.len() as i32, 0, duplicates_found as i32, errors_found as i32, None,
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let updated_rows = bulk_db::get_import_rows(pool, job.id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(ImportValidationResult {
        job: updated_job,
        rows: updated_rows,
        duplicates_found,
        errors_found,
    })
}

/// Validate a single import row based on entity type
async fn validate_row(
    pool: &PgPool,
    entity_type: &str,
    raw_data: &serde_json::Value,
) -> (ImportRowStatus, Option<serde_json::Value>, Option<Uuid>, Option<String>, Option<serde_json::Value>) {
    match entity_type {
        "kb_entry" => validate_kb_entry_row(pool, raw_data).await,
        "user" => validate_user_row(pool, raw_data).await,
        _ => (
            ImportRowStatus::Error,
            None, None,
            Some(format!("Unknown entity type: {}", entity_type)),
            None,
        ),
    }
}

async fn validate_kb_entry_row(
    pool: &PgPool,
    data: &serde_json::Value,
) -> (ImportRowStatus, Option<serde_json::Value>, Option<Uuid>, Option<String>, Option<serde_json::Value>) {
    let mut errors = Vec::new();

    let item_name = data.get("item_name").and_then(|v| v.as_str()).unwrap_or("");
    let disposal_category = data.get("disposal_category").and_then(|v| v.as_str()).unwrap_or("");
    let disposal_instructions = data.get("disposal_instructions").and_then(|v| v.as_str()).unwrap_or("");

    if item_name.is_empty() { errors.push("item_name is required"); }
    if disposal_category.is_empty() { errors.push("disposal_category is required"); }
    if disposal_instructions.is_empty() { errors.push("disposal_instructions is required"); }

    if !errors.is_empty() {
        return (
            ImportRowStatus::Error, Some(data.clone()), None,
            Some(errors.join("; ")),
            Some(serde_json::json!(errors)),
        );
    }

    // Check for duplicates via content hash
    let name_hash = fingerprint::compute_content_hash(item_name);
    if let Ok(matches) = bulk_db::find_matching_fingerprints(pool, "kb_entry", "content_hash", &name_hash).await {
        if !matches.is_empty() {
            return (
                ImportRowStatus::Duplicate, Some(data.clone()),
                Some(matches[0].entity_id), None, None,
            );
        }
    }

    (ImportRowStatus::Valid, Some(data.clone()), None, None, None)
}

async fn validate_user_row(
    pool: &PgPool,
    data: &serde_json::Value,
) -> (ImportRowStatus, Option<serde_json::Value>, Option<Uuid>, Option<String>, Option<serde_json::Value>) {
    let mut errors = Vec::new();

    let username = data.get("username").and_then(|v| v.as_str()).unwrap_or("");
    let role = data.get("role").and_then(|v| v.as_str()).unwrap_or("");

    if username.is_empty() { errors.push("username is required"); }
    if role.is_empty() { errors.push("role is required"); }

    if !errors.is_empty() {
        return (
            ImportRowStatus::Error, Some(data.clone()), None,
            Some(errors.join("; ")),
            Some(serde_json::json!(errors)),
        );
    }

    // Check username uniqueness
    let key_hash = fingerprint::compute_key_fields_hash(&[("username", username)]);
    if let Ok(matches) = bulk_db::find_matching_fingerprints(pool, "user", "key_fields", &key_hash).await {
        if !matches.is_empty() {
            return (
                ImportRowStatus::Duplicate, Some(data.clone()),
                Some(matches[0].entity_id), None, None,
            );
        }
    }

    (ImportRowStatus::Valid, Some(data.clone()), None, None, None)
}

/// Execute confirmed import rows (only valid/non-duplicate rows)
pub async fn execute_import(
    pool: &PgPool,
    job_id: Uuid,
    imported_by: Uuid,
) -> Result<ImportJob, AppError> {
    let rows = bulk_db::get_import_rows(pool, job_id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let job = bulk_db::get_import_job(pool, job_id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Import job not found".into()))?;

    let mut imported = 0i32;
    let change_set_id = Uuid::new_v4();

    for row in &rows {
        if row.status != ImportRowStatus::Valid {
            continue;
        }

        let parsed = row.parsed_data.as_ref().unwrap_or(&row.raw_data);

        match job.entity_type.as_str() {
            "kb_entry" => {
                let item_name = parsed.get("item_name").and_then(|v| v.as_str()).unwrap_or("");
                let region = parsed.get("region").and_then(|v| v.as_str()).unwrap_or("default");
                let disposal_category = parsed.get("disposal_category").and_then(|v| v.as_str()).unwrap_or("");
                let disposal_instructions = parsed.get("disposal_instructions").and_then(|v| v.as_str()).unwrap_or("");

                let entry = crate::db::knowledge_base::create_entry(
                    pool, item_name, None, region, Some(imported_by),
                ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

                let today = chrono::Utc::now().date_naive();
                crate::db::knowledge_base::create_version(
                    pool, entry.id, 1, item_name, disposal_category,
                    disposal_instructions, None, None, region, None,
                    today, Some("Bulk import"), Some(imported_by),
                ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

                // Record change
                bulk_db::record_change(
                    pool, "kb_entry", entry.id, &ChangeOperation::Import,
                    None, None, Some(parsed), Some(change_set_id),
                    Some(job_id), None, imported_by,
                ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

                // Store fingerprint
                let name_hash = fingerprint::compute_content_hash(item_name);
                let _ = bulk_db::upsert_fingerprint(pool, "kb_entry", entry.id, "content_hash", &name_hash, Some(item_name)).await;

                bulk_db::update_import_row(
                    pool, row.id, &ImportRowStatus::Imported, None,
                    Some(entry.id), None, None, None,
                ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

                imported += 1;
            }
            _ => {}
        }
    }

    let updated = bulk_db::update_import_job_status(
        pool, job_id, &ImportJobStatus::Completed,
        job.total_rows, imported, job.duplicate_rows, job.error_rows, None,
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(updated)
}
