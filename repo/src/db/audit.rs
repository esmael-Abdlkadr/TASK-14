use sha2::{Sha256, Digest};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{AuditLogEntry, AuditLogQuery, UserRole};

pub struct AuditEntryInput {
    pub user_id: Option<Uuid>,
    pub username: String,
    pub role: Option<UserRole>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub details: Option<serde_json::Value>,
    pub encrypted_details: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub session_id: Option<Uuid>,
}

pub async fn insert_audit_entry(
    pool: &PgPool,
    input: AuditEntryInput,
) -> Result<AuditLogEntry, sqlx::Error> {
    // Get the previous entry's hash for chain integrity
    let prev_hash: Option<String> = sqlx::query_scalar(
        "SELECT entry_hash FROM audit_log ORDER BY id DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    // Compute the hash for this entry
    let hash_input = format!(
        "{}|{}|{}|{}|{}|{}",
        input.username,
        input.action,
        input.resource_type.as_deref().unwrap_or(""),
        input.resource_id.as_deref().unwrap_or(""),
        prev_hash.as_deref().unwrap_or("GENESIS"),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
    );
    let mut hasher = Sha256::new();
    hasher.update(hash_input.as_bytes());
    let entry_hash = hex::encode(hasher.finalize());

    sqlx::query_as::<_, AuditLogEntry>(
        r#"
        INSERT INTO audit_log
            (user_id, username, role, action, resource_type, resource_id,
             details, encrypted_details, ip_address, user_agent, session_id, prev_hash, entry_hash)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        RETURNING *
        "#,
    )
    .bind(input.user_id)
    .bind(&input.username)
    .bind(&input.role)
    .bind(&input.action)
    .bind(&input.resource_type)
    .bind(&input.resource_id)
    .bind(&input.details)
    .bind(&input.encrypted_details)
    .bind(&input.ip_address)
    .bind(&input.user_agent)
    .bind(input.session_id)
    .bind(&prev_hash)
    .bind(&entry_hash)
    .fetch_one(pool)
    .await
}

pub async fn query_audit_log(
    pool: &PgPool,
    query: &AuditLogQuery,
) -> Result<(Vec<AuditLogEntry>, i64), sqlx::Error> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(50).min(200);
    let offset = (page - 1) * page_size;

    let mut where_clauses = Vec::new();
    let mut param_index = 1u32;

    if query.user_id.is_some() {
        where_clauses.push(format!("user_id = ${}", param_index));
        param_index += 1;
    }
    if query.action.is_some() {
        where_clauses.push(format!("action = ${}", param_index));
        param_index += 1;
    }
    if query.resource_type.is_some() {
        where_clauses.push(format!("resource_type = ${}", param_index));
        param_index += 1;
    }
    if query.from_date.is_some() {
        where_clauses.push(format!("created_at >= ${}", param_index));
        param_index += 1;
    }
    if query.to_date.is_some() {
        where_clauses.push(format!("created_at <= ${}", param_index));
        param_index += 1;
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let count_sql = format!("SELECT COUNT(*) FROM audit_log {}", where_sql);
    let data_sql = format!(
        "SELECT * FROM audit_log {} ORDER BY id DESC LIMIT ${} OFFSET ${}",
        where_sql, param_index, param_index + 1
    );

    // Build count query
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    if let Some(ref uid) = query.user_id {
        count_query = count_query.bind(uid);
    }
    if let Some(ref action) = query.action {
        count_query = count_query.bind(action);
    }
    if let Some(ref rt) = query.resource_type {
        count_query = count_query.bind(rt);
    }
    if let Some(ref fd) = query.from_date {
        count_query = count_query.bind(fd);
    }
    if let Some(ref td) = query.to_date {
        count_query = count_query.bind(td);
    }
    let total = count_query.fetch_one(pool).await?;

    // Build data query
    let mut data_query = sqlx::query_as::<_, AuditLogEntry>(&data_sql);
    if let Some(ref uid) = query.user_id {
        data_query = data_query.bind(uid);
    }
    if let Some(ref action) = query.action {
        data_query = data_query.bind(action);
    }
    if let Some(ref rt) = query.resource_type {
        data_query = data_query.bind(rt);
    }
    if let Some(ref fd) = query.from_date {
        data_query = data_query.bind(fd);
    }
    if let Some(ref td) = query.to_date {
        data_query = data_query.bind(td);
    }
    data_query = data_query.bind(page_size).bind(offset);
    let entries = data_query.fetch_all(pool).await?;

    Ok((entries, total))
}

pub async fn verify_chain_integrity(pool: &PgPool) -> Result<bool, sqlx::Error> {
    let entries = sqlx::query_as::<_, AuditLogEntry>(
        "SELECT * FROM audit_log ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await?;

    let mut prev_hash: Option<String> = None;
    for entry in &entries {
        if entry.prev_hash != prev_hash {
            log::warn!(
                "Audit chain broken at entry {}: expected prev_hash {:?}, got {:?}",
                entry.id,
                prev_hash,
                entry.prev_hash
            );
            return Ok(false);
        }
        prev_hash = Some(entry.entry_hash.clone());
    }

    Ok(true)
}
