use sqlx::PgPool;
use uuid::Uuid;

use crate::models::dispute::{ClassificationDispute, DisputeStatus};

pub async fn create_dispute(
    pool: &PgPool,
    kb_entry_id: Uuid,
    disputed_by: Uuid,
    reason: &str,
    proposed_category: Option<&str>,
    proposed_instructions: Option<&str>,
) -> Result<ClassificationDispute, sqlx::Error> {
    sqlx::query_as::<_, ClassificationDispute>(
        r#"INSERT INTO classification_disputes
           (kb_entry_id, disputed_by, reason, proposed_category, proposed_instructions)
           VALUES ($1, $2, $3, $4, $5) RETURNING *"#,
    )
    .bind(kb_entry_id)
    .bind(disputed_by)
    .bind(reason)
    .bind(proposed_category)
    .bind(proposed_instructions)
    .fetch_one(pool)
    .await
}

pub async fn get_dispute(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<ClassificationDispute>, sqlx::Error> {
    sqlx::query_as::<_, ClassificationDispute>(
        "SELECT * FROM classification_disputes WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn list_disputes(
    pool: &PgPool,
    status: Option<&DisputeStatus>,
    limit: i64,
    offset: i64,
) -> Result<Vec<ClassificationDispute>, sqlx::Error> {
    match status {
        Some(s) => {
            sqlx::query_as::<_, ClassificationDispute>(
                r#"SELECT * FROM classification_disputes
                   WHERE status = $1
                   ORDER BY created_at DESC
                   LIMIT $2 OFFSET $3"#,
            )
            .bind(s)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as::<_, ClassificationDispute>(
                r#"SELECT * FROM classification_disputes
                   ORDER BY created_at DESC
                   LIMIT $1 OFFSET $2"#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
        }
    }
}

pub async fn update_dispute_status(
    pool: &PgPool,
    id: Uuid,
    status: &DisputeStatus,
    resolution_notes: Option<&str>,
    resolved_by: Uuid,
) -> Result<ClassificationDispute, sqlx::Error> {
    sqlx::query_as::<_, ClassificationDispute>(
        r#"UPDATE classification_disputes
           SET status = $2, resolution_notes = $3, resolved_by = $4, resolved_at = NOW()
           WHERE id = $1
           RETURNING *"#,
    )
    .bind(id)
    .bind(status)
    .bind(resolution_notes)
    .bind(resolved_by)
    .fetch_one(pool)
    .await
}
