use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    ChangeOperation, ContentFingerprint, DataChange, DuplicateFlag, DuplicateStatus,
    ImportJob, ImportJobStatus, ImportRow, ImportRowStatus, MergeConflict, MergeRequest,
    MergeRequestStatus,
};

// ── Import Jobs ─────────────────────────────────────────────

pub async fn create_import_job(
    pool: &PgPool, name: &str, entity_type: &str, file_name: Option<&str>,
    total_rows: i32, imported_by: Uuid,
) -> Result<ImportJob, sqlx::Error> {
    sqlx::query_as::<_, ImportJob>(
        "INSERT INTO import_jobs (name, entity_type, file_name, total_rows, imported_by) VALUES ($1,$2,$3,$4,$5) RETURNING *",
    ).bind(name).bind(entity_type).bind(file_name).bind(total_rows).bind(imported_by)
    .fetch_one(pool).await
}

pub async fn get_import_job(pool: &PgPool, id: Uuid) -> Result<Option<ImportJob>, sqlx::Error> {
    sqlx::query_as::<_, ImportJob>("SELECT * FROM import_jobs WHERE id=$1")
        .bind(id).fetch_optional(pool).await
}

pub async fn update_import_job_status(
    pool: &PgPool, id: Uuid, status: &ImportJobStatus,
    processed: i32, imported: i32, duplicates: i32, errors: i32, error_msg: Option<&str>,
) -> Result<ImportJob, sqlx::Error> {
    sqlx::query_as::<_, ImportJob>(
        r#"UPDATE import_jobs SET status=$2, processed_rows=$3, imported_rows=$4,
        duplicate_rows=$5, error_rows=$6, error_message=$7,
        completed_at = CASE WHEN $2 IN ('completed','failed','cancelled') THEN NOW() ELSE completed_at END
        WHERE id=$1 RETURNING *"#,
    ).bind(id).bind(status).bind(processed).bind(imported).bind(duplicates).bind(errors).bind(error_msg)
    .fetch_one(pool).await
}

pub async fn list_import_jobs(pool: &PgPool, limit: i64, offset: i64) -> Result<Vec<ImportJob>, sqlx::Error> {
    sqlx::query_as::<_, ImportJob>(
        "SELECT * FROM import_jobs ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    ).bind(limit).bind(offset).fetch_all(pool).await
}

// ── Import Rows ─────────────────────────────────────────────

pub async fn create_import_row(
    pool: &PgPool, job_id: Uuid, row_number: i32, raw_data: &serde_json::Value,
) -> Result<ImportRow, sqlx::Error> {
    sqlx::query_as::<_, ImportRow>(
        "INSERT INTO import_rows (job_id, row_number, raw_data) VALUES ($1,$2,$3) RETURNING *",
    ).bind(job_id).bind(row_number).bind(raw_data).fetch_one(pool).await
}

pub async fn update_import_row(
    pool: &PgPool, id: Uuid, status: &ImportRowStatus, parsed_data: Option<&serde_json::Value>,
    entity_id: Option<Uuid>, duplicate_of: Option<Uuid>, error_msg: Option<&str>,
    validation_errors: Option<&serde_json::Value>,
) -> Result<ImportRow, sqlx::Error> {
    sqlx::query_as::<_, ImportRow>(
        r#"UPDATE import_rows SET status=$2, parsed_data=$3, entity_id=$4,
        duplicate_of=$5, error_message=$6, validation_errors=$7
        WHERE id=$1 RETURNING *"#,
    ).bind(id).bind(status).bind(parsed_data).bind(entity_id)
    .bind(duplicate_of).bind(error_msg).bind(validation_errors)
    .fetch_one(pool).await
}

pub async fn get_import_rows(pool: &PgPool, job_id: Uuid) -> Result<Vec<ImportRow>, sqlx::Error> {
    sqlx::query_as::<_, ImportRow>(
        "SELECT * FROM import_rows WHERE job_id=$1 ORDER BY row_number",
    ).bind(job_id).fetch_all(pool).await
}

// ── Data Changes ────────────────────────────────────────────

pub async fn record_change(
    pool: &PgPool, entity_type: &str, entity_id: Uuid, operation: &ChangeOperation,
    field_name: Option<&str>, old_value: Option<&serde_json::Value>,
    new_value: Option<&serde_json::Value>, change_set_id: Option<Uuid>,
    import_job_id: Option<Uuid>, merge_request_id: Option<Uuid>, changed_by: Uuid,
) -> Result<DataChange, sqlx::Error> {
    sqlx::query_as::<_, DataChange>(
        r#"INSERT INTO data_changes
        (entity_type, entity_id, operation, field_name, old_value, new_value,
         change_set_id, import_job_id, merge_request_id, changed_by)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING *"#,
    ).bind(entity_type).bind(entity_id).bind(operation).bind(field_name)
    .bind(old_value).bind(new_value).bind(change_set_id)
    .bind(import_job_id).bind(merge_request_id).bind(changed_by)
    .fetch_one(pool).await
}

pub async fn get_change_history(
    pool: &PgPool, entity_type: Option<&str>, entity_id: Option<Uuid>,
    limit: i64, offset: i64,
) -> Result<Vec<DataChange>, sqlx::Error> {
    match (entity_type, entity_id) {
        (Some(et), Some(eid)) => sqlx::query_as::<_, DataChange>(
            "SELECT * FROM data_changes WHERE entity_type=$1 AND entity_id=$2 ORDER BY changed_at DESC LIMIT $3 OFFSET $4",
        ).bind(et).bind(eid).bind(limit).bind(offset).fetch_all(pool).await,
        (Some(et), None) => sqlx::query_as::<_, DataChange>(
            "SELECT * FROM data_changes WHERE entity_type=$1 ORDER BY changed_at DESC LIMIT $2 OFFSET $3",
        ).bind(et).bind(limit).bind(offset).fetch_all(pool).await,
        _ => sqlx::query_as::<_, DataChange>(
            "SELECT * FROM data_changes ORDER BY changed_at DESC LIMIT $1 OFFSET $2",
        ).bind(limit).bind(offset).fetch_all(pool).await,
    }
}

pub async fn count_changes(
    pool: &PgPool, entity_type: Option<&str>, entity_id: Option<Uuid>,
) -> Result<i64, sqlx::Error> {
    match (entity_type, entity_id) {
        (Some(et), Some(eid)) => sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM data_changes WHERE entity_type=$1 AND entity_id=$2",
        ).bind(et).bind(eid).fetch_one(pool).await,
        (Some(et), None) => sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM data_changes WHERE entity_type=$1",
        ).bind(et).fetch_one(pool).await,
        _ => sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM data_changes",
        ).fetch_one(pool).await,
    }
}

pub async fn revert_change(pool: &PgPool, id: Uuid, reverted_by: Uuid) -> Result<DataChange, sqlx::Error> {
    sqlx::query_as::<_, DataChange>(
        "UPDATE data_changes SET reverted_at=NOW(), reverted_by=$2 WHERE id=$1 RETURNING *",
    ).bind(id).bind(reverted_by).fetch_one(pool).await
}

// ── Content Fingerprints ────────────────────────────────────

pub async fn upsert_fingerprint(
    pool: &PgPool, entity_type: &str, entity_id: Uuid, fp_type: &str,
    fingerprint: &str, source_text: Option<&str>,
) -> Result<ContentFingerprint, sqlx::Error> {
    sqlx::query_as::<_, ContentFingerprint>(
        r#"INSERT INTO content_fingerprints (entity_type, entity_id, fingerprint_type, fingerprint, source_text)
        VALUES ($1,$2,$3,$4,$5)
        ON CONFLICT (id) DO UPDATE SET fingerprint=$4, source_text=$5, updated_at=NOW()
        RETURNING *"#,
    ).bind(entity_type).bind(entity_id).bind(fp_type).bind(fingerprint).bind(source_text)
    .fetch_one(pool).await
}

pub async fn find_matching_fingerprints(
    pool: &PgPool, entity_type: &str, fp_type: &str, fingerprint: &str,
) -> Result<Vec<ContentFingerprint>, sqlx::Error> {
    sqlx::query_as::<_, ContentFingerprint>(
        "SELECT * FROM content_fingerprints WHERE entity_type=$1 AND fingerprint_type=$2 AND fingerprint=$3",
    ).bind(entity_type).bind(fp_type).bind(fingerprint).fetch_all(pool).await
}

// ── Duplicate Flags ─────────────────────────────────────────

pub async fn create_duplicate_flag(
    pool: &PgPool, entity_type: &str, source_id: Uuid, target_id: Uuid,
    match_type: &str, confidence: f32, details: Option<&serde_json::Value>,
) -> Result<DuplicateFlag, sqlx::Error> {
    sqlx::query_as::<_, DuplicateFlag>(
        r#"INSERT INTO duplicate_flags (entity_type, source_id, target_id, match_type, confidence, details)
        VALUES ($1,$2,$3,$4,$5,$6) RETURNING *"#,
    ).bind(entity_type).bind(source_id).bind(target_id).bind(match_type).bind(confidence).bind(details)
    .fetch_one(pool).await
}

pub async fn list_duplicate_flags(
    pool: &PgPool, entity_type: Option<&str>, status: Option<&DuplicateStatus>,
    limit: i64, offset: i64,
) -> Result<Vec<DuplicateFlag>, sqlx::Error> {
    match (entity_type, status) {
        (Some(et), Some(s)) => sqlx::query_as::<_, DuplicateFlag>(
            "SELECT * FROM duplicate_flags WHERE entity_type=$1 AND status=$2 ORDER BY detected_at DESC LIMIT $3 OFFSET $4",
        ).bind(et).bind(s).bind(limit).bind(offset).fetch_all(pool).await,
        (Some(et), None) => sqlx::query_as::<_, DuplicateFlag>(
            "SELECT * FROM duplicate_flags WHERE entity_type=$1 ORDER BY detected_at DESC LIMIT $2 OFFSET $3",
        ).bind(et).bind(limit).bind(offset).fetch_all(pool).await,
        (None, Some(s)) => sqlx::query_as::<_, DuplicateFlag>(
            "SELECT * FROM duplicate_flags WHERE status=$1 ORDER BY detected_at DESC LIMIT $2 OFFSET $3",
        ).bind(s).bind(limit).bind(offset).fetch_all(pool).await,
        _ => sqlx::query_as::<_, DuplicateFlag>(
            "SELECT * FROM duplicate_flags ORDER BY detected_at DESC LIMIT $1 OFFSET $2",
        ).bind(limit).bind(offset).fetch_all(pool).await,
    }
}

pub async fn resolve_duplicate(
    pool: &PgPool, id: Uuid, status: &DuplicateStatus, resolved_by: Uuid,
) -> Result<DuplicateFlag, sqlx::Error> {
    sqlx::query_as::<_, DuplicateFlag>(
        "UPDATE duplicate_flags SET status=$2, resolved_by=$3, resolved_at=NOW() WHERE id=$1 RETURNING *",
    ).bind(id).bind(status).bind(resolved_by).fetch_one(pool).await
}

// ── Merge Requests ──────────────────────────────────────────

pub async fn create_merge_request(
    pool: &PgPool, entity_type: &str, source_id: Uuid, target_id: Uuid,
    duplicate_flag_id: Option<Uuid>, requested_by: Uuid,
) -> Result<MergeRequest, sqlx::Error> {
    sqlx::query_as::<_, MergeRequest>(
        r#"INSERT INTO merge_requests (entity_type, source_id, target_id, duplicate_flag_id, requested_by)
        VALUES ($1,$2,$3,$4,$5) RETURNING *"#,
    ).bind(entity_type).bind(source_id).bind(target_id).bind(duplicate_flag_id).bind(requested_by)
    .fetch_one(pool).await
}

pub async fn get_merge_request(pool: &PgPool, id: Uuid) -> Result<Option<MergeRequest>, sqlx::Error> {
    sqlx::query_as::<_, MergeRequest>("SELECT * FROM merge_requests WHERE id=$1")
        .bind(id).fetch_optional(pool).await
}

pub async fn list_merge_requests(
    pool: &PgPool, status: Option<&MergeRequestStatus>, limit: i64, offset: i64,
) -> Result<Vec<MergeRequest>, sqlx::Error> {
    match status {
        Some(s) => sqlx::query_as::<_, MergeRequest>(
            "SELECT * FROM merge_requests WHERE status=$1 ORDER BY requested_at DESC LIMIT $2 OFFSET $3",
        ).bind(s).bind(limit).bind(offset).fetch_all(pool).await,
        None => sqlx::query_as::<_, MergeRequest>(
            "SELECT * FROM merge_requests ORDER BY requested_at DESC LIMIT $1 OFFSET $2",
        ).bind(limit).bind(offset).fetch_all(pool).await,
    }
}

pub async fn review_merge_request(
    pool: &PgPool, id: Uuid, status: &MergeRequestStatus,
    reviewed_by: Uuid, review_notes: Option<&str>,
    resolution: Option<&serde_json::Value>, provenance: Option<&serde_json::Value>,
) -> Result<MergeRequest, sqlx::Error> {
    sqlx::query_as::<_, MergeRequest>(
        r#"UPDATE merge_requests SET status=$2, reviewed_by=$3, review_notes=$4,
        resolution=$5, provenance=$6, reviewed_at=NOW(),
        applied_at = CASE WHEN $2='applied'::merge_request_status THEN NOW() ELSE applied_at END
        WHERE id=$1 RETURNING *"#,
    ).bind(id).bind(status).bind(reviewed_by).bind(review_notes).bind(resolution).bind(provenance)
    .fetch_one(pool).await
}

// ── Merge Conflicts ─────────────────────────────────────────

pub async fn create_merge_conflict(
    pool: &PgPool, merge_request_id: Uuid, field_name: &str,
    source_value: Option<&serde_json::Value>, target_value: Option<&serde_json::Value>,
) -> Result<MergeConflict, sqlx::Error> {
    sqlx::query_as::<_, MergeConflict>(
        r#"INSERT INTO merge_conflicts (merge_request_id, field_name, source_value, target_value)
        VALUES ($1,$2,$3,$4) RETURNING *"#,
    ).bind(merge_request_id).bind(field_name).bind(source_value).bind(target_value)
    .fetch_one(pool).await
}

pub async fn get_merge_conflicts(pool: &PgPool, merge_request_id: Uuid) -> Result<Vec<MergeConflict>, sqlx::Error> {
    sqlx::query_as::<_, MergeConflict>(
        "SELECT * FROM merge_conflicts WHERE merge_request_id=$1 ORDER BY field_name",
    ).bind(merge_request_id).fetch_all(pool).await
}

pub async fn resolve_conflict(
    pool: &PgPool, id: Uuid, resolution: &str,
    custom_value: Option<&serde_json::Value>, resolved_by: Uuid,
) -> Result<MergeConflict, sqlx::Error> {
    sqlx::query_as::<_, MergeConflict>(
        "UPDATE merge_conflicts SET resolution=$2, custom_value=$3, resolved_by=$4, resolved_at=NOW() WHERE id=$1 RETURNING *",
    ).bind(id).bind(resolution).bind(custom_value).bind(resolved_by)
    .fetch_one(pool).await
}
