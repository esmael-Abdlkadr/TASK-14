use chrono::Utc;
use sqlx::PgPool;
use std::path::PathBuf;
use uuid::Uuid;

use crate::db::messaging as msg_db;
use crate::errors::AppError;
use crate::models::{ExternalPayload, NotificationChannel, PayloadStatus};

/// Export all queued payloads for a given channel to files on disk.
/// Returns the directory path and number of files written.
pub async fn export_queued_payloads(
    pool: &PgPool,
    channel: &NotificationChannel,
    performed_by: Option<Uuid>,
) -> Result<ExportBatchResult, AppError> {
    let payloads = msg_db::get_payload_queue(pool, Some(&PayloadStatus::Queued), Some(channel), 1000, 0)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    if payloads.is_empty() {
        return Ok(ExportBatchResult {
            channel: channel.clone(),
            count: 0,
            export_dir: String::new(),
            files: Vec::new(),
        });
    }

    let base_dir = std::env::var("CIVICSORT_EXPORT_DIR")
        .unwrap_or_else(|_| "./data/exports".to_string());
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let channel_name = match channel {
        NotificationChannel::Sms => "sms",
        NotificationChannel::Email => "email",
        NotificationChannel::Push => "push",
        NotificationChannel::InApp => "in_app",
    };
    let export_dir = PathBuf::from(&base_dir).join(format!("{}_{}", channel_name, timestamp));

    tokio::fs::create_dir_all(&export_dir)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to create export dir: {}", e)))?;

    let mut files = Vec::new();

    for payload in &payloads {
        let filename = format!("{}_{}.json", channel_name, payload.id);
        let file_path = export_dir.join(&filename);

        let content = build_payload_file(payload, channel);
        tokio::fs::write(&file_path, &content)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to write payload file: {}", e)))?;

        let path_str = file_path.to_string_lossy().to_string();

        // Update payload status
        msg_db::set_payload_export_path(pool, payload.id, &path_str)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Log delivery action
        let _ = msg_db::log_delivery(
            pool, payload.id, "exported",
            Some(&PayloadStatus::Queued), &PayloadStatus::Exported,
            Some(&format!("Exported to {}", path_str)), performed_by,
        ).await;

        files.push(path_str);
    }

    // Also write a batch manifest
    let manifest = serde_json::json!({
        "channel": channel_name,
        "timestamp": Utc::now().to_rfc3339(),
        "count": payloads.len(),
        "files": files,
    });
    let manifest_path = export_dir.join("manifest.json");
    tokio::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap_or_default())
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to write manifest: {}", e)))?;

    Ok(ExportBatchResult {
        channel: channel.clone(),
        count: payloads.len(),
        export_dir: export_dir.to_string_lossy().to_string(),
        files,
    })
}

/// Build a JSON payload file for external transfer
fn build_payload_file(payload: &ExternalPayload, channel: &NotificationChannel) -> String {
    let content = match channel {
        NotificationChannel::Sms => serde_json::json!({
            "type": "sms",
            "id": payload.id,
            "to": payload.recipient,
            "body": payload.body,
            "metadata": payload.metadata,
            "created_at": payload.created_at,
        }),
        NotificationChannel::Email => serde_json::json!({
            "type": "email",
            "id": payload.id,
            "to": payload.recipient,
            "subject": payload.subject,
            "body": payload.body,
            "metadata": payload.metadata,
            "created_at": payload.created_at,
        }),
        NotificationChannel::Push => serde_json::json!({
            "type": "push",
            "id": payload.id,
            "device_token": payload.recipient,
            "title": payload.subject,
            "body": payload.body,
            "metadata": payload.metadata,
            "created_at": payload.created_at,
        }),
        NotificationChannel::InApp => serde_json::json!({
            "type": "in_app",
            "id": payload.id,
            "body": payload.body,
        }),
    };

    serde_json::to_string_pretty(&content).unwrap_or_default()
}

/// Mark payloads as delivered (after manual transfer confirmation)
pub async fn mark_batch_delivered(
    pool: &PgPool,
    payload_ids: &[Uuid],
    performed_by: Option<Uuid>,
) -> Result<usize, AppError> {
    let mut count = 0;
    for id in payload_ids {
        let payload = msg_db::update_payload_status(pool, *id, &PayloadStatus::Delivered, None)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        msg_db::log_delivery(
            pool, *id, "delivered",
            Some(&PayloadStatus::Exported), &PayloadStatus::Delivered,
            Some("Manually confirmed delivery"), performed_by,
        ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        count += 1;
    }
    Ok(count)
}

/// Mark a payload as failed and optionally schedule retry
pub async fn mark_payload_failed(
    pool: &PgPool,
    payload_id: Uuid,
    error: &str,
    performed_by: Option<Uuid>,
) -> Result<ExternalPayload, AppError> {
    // Check if retries are available
    let current = msg_db::get_payload_queue(pool, None, None, 1, 0)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Default: schedule retry if under max
    let new_status = PayloadStatus::Retrying;

    let updated = msg_db::update_payload_status(pool, payload_id, &new_status, Some(error))
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    if updated.retry_count >= updated.max_retries {
        // Exceeded max retries, mark as permanently failed
        let final_update = msg_db::update_payload_status(pool, payload_id, &PayloadStatus::Failed, Some(error))
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        msg_db::log_delivery(
            pool, payload_id, "failed",
            Some(&PayloadStatus::Retrying), &PayloadStatus::Failed,
            Some(&format!("Max retries exceeded: {}", error)), performed_by,
        ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        return Ok(final_update);
    }

    msg_db::log_delivery(
        pool, payload_id, "retry_scheduled",
        Some(&PayloadStatus::Exported), &PayloadStatus::Retrying,
        Some(&format!("Retry scheduled: {}", error)), performed_by,
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(updated)
}

#[derive(Debug, serde::Serialize)]
pub struct ExportBatchResult {
    pub channel: NotificationChannel,
    pub count: usize,
    pub export_dir: String,
    pub files: Vec<String>,
}
