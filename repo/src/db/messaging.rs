use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    DeliveryLogEntry, ExternalPayload, Notification, NotificationChannel, NotificationStatus,
    NotificationTemplate, PayloadStatus, TemplateVariable, TemplateVariableInput, TriggerEvent,
    TriggerRule,
};

// ── Templates ───────────────────────────────────────────────

pub async fn create_template(
    pool: &PgPool, name: &str, description: Option<&str>, channel: &NotificationChannel,
    subject_template: Option<&str>, body_template: &str,
    sms_template: Option<&str>, html_template: Option<&str>,
    created_by: Option<Uuid>,
) -> Result<NotificationTemplate, sqlx::Error> {
    sqlx::query_as::<_, NotificationTemplate>(
        r#"INSERT INTO notification_templates
        (name, description, channel, subject_template, body_template, sms_template, html_template, created_by)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING *"#,
    ).bind(name).bind(description).bind(channel).bind(subject_template)
    .bind(body_template).bind(sms_template).bind(html_template).bind(created_by)
    .fetch_one(pool).await
}

pub async fn get_template(pool: &PgPool, id: Uuid) -> Result<Option<NotificationTemplate>, sqlx::Error> {
    sqlx::query_as::<_, NotificationTemplate>("SELECT * FROM notification_templates WHERE id=$1")
        .bind(id).fetch_optional(pool).await
}

pub async fn get_template_by_name(pool: &PgPool, name: &str) -> Result<Option<NotificationTemplate>, sqlx::Error> {
    sqlx::query_as::<_, NotificationTemplate>(
        "SELECT * FROM notification_templates WHERE name=$1 AND is_active=TRUE",
    ).bind(name).fetch_optional(pool).await
}

pub async fn list_templates(pool: &PgPool) -> Result<Vec<NotificationTemplate>, sqlx::Error> {
    sqlx::query_as::<_, NotificationTemplate>(
        "SELECT * FROM notification_templates WHERE is_active=TRUE ORDER BY name",
    ).fetch_all(pool).await
}

pub async fn update_template(
    pool: &PgPool, id: Uuid, name: &str, description: Option<&str>,
    channel: &NotificationChannel, subject_template: Option<&str>,
    body_template: &str, sms_template: Option<&str>, html_template: Option<&str>,
) -> Result<NotificationTemplate, sqlx::Error> {
    sqlx::query_as::<_, NotificationTemplate>(
        r#"UPDATE notification_templates SET
        name=$2, description=$3, channel=$4, subject_template=$5,
        body_template=$6, sms_template=$7, html_template=$8, updated_at=NOW()
        WHERE id=$1 RETURNING *"#,
    ).bind(id).bind(name).bind(description).bind(channel).bind(subject_template)
    .bind(body_template).bind(sms_template).bind(html_template)
    .fetch_one(pool).await
}

pub async fn deactivate_template(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE notification_templates SET is_active=FALSE, updated_at=NOW() WHERE id=$1")
        .bind(id).execute(pool).await?;
    Ok(())
}

// ── Template Variables ──────────────────────────────────────

pub async fn set_template_variables(
    pool: &PgPool, template_id: Uuid, vars: &[TemplateVariableInput],
) -> Result<Vec<TemplateVariable>, sqlx::Error> {
    sqlx::query("DELETE FROM template_variables WHERE template_id=$1")
        .bind(template_id).execute(pool).await?;
    let mut result = Vec::new();
    for v in vars {
        let tv = sqlx::query_as::<_, TemplateVariable>(
            r#"INSERT INTO template_variables (template_id, var_name, var_type, description, default_value, is_required)
            VALUES ($1,$2,$3,$4,$5,$6) RETURNING *"#,
        )
        .bind(template_id).bind(&v.var_name)
        .bind(v.var_type.as_deref().unwrap_or("string"))
        .bind(v.description.as_deref()).bind(v.default_value.as_deref())
        .bind(v.is_required.unwrap_or(true))
        .fetch_one(pool).await?;
        result.push(tv);
    }
    Ok(result)
}

pub async fn get_template_variables(pool: &PgPool, template_id: Uuid) -> Result<Vec<TemplateVariable>, sqlx::Error> {
    sqlx::query_as::<_, TemplateVariable>(
        "SELECT * FROM template_variables WHERE template_id=$1 ORDER BY var_name",
    ).bind(template_id).fetch_all(pool).await
}

// ── Trigger Rules ───────────────────────────────────────────

pub async fn create_trigger_rule(
    pool: &PgPool, name: &str, event: &TriggerEvent, template_id: Uuid,
    channel: &NotificationChannel, conditions: Option<&serde_json::Value>,
    target_role: Option<&str>, priority: i32, created_by: Option<Uuid>,
) -> Result<TriggerRule, sqlx::Error> {
    sqlx::query_as::<_, TriggerRule>(
        r#"INSERT INTO trigger_rules (name, event, template_id, channel, conditions, target_role, priority, created_by)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING *"#,
    ).bind(name).bind(event).bind(template_id).bind(channel)
    .bind(conditions).bind(target_role).bind(priority).bind(created_by)
    .fetch_one(pool).await
}

pub async fn list_trigger_rules(pool: &PgPool) -> Result<Vec<TriggerRule>, sqlx::Error> {
    sqlx::query_as::<_, TriggerRule>(
        "SELECT * FROM trigger_rules WHERE is_active=TRUE ORDER BY event, priority DESC",
    ).fetch_all(pool).await
}

pub async fn get_rules_for_event(pool: &PgPool, event: &TriggerEvent) -> Result<Vec<TriggerRule>, sqlx::Error> {
    sqlx::query_as::<_, TriggerRule>(
        "SELECT * FROM trigger_rules WHERE event=$1 AND is_active=TRUE ORDER BY priority DESC",
    ).bind(event).fetch_all(pool).await
}

pub async fn deactivate_trigger_rule(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE trigger_rules SET is_active=FALSE WHERE id=$1")
        .bind(id).execute(pool).await?;
    Ok(())
}

// ── Notifications ───────────────────────────────────────────

pub async fn create_notification(
    pool: &PgPool, user_id: Uuid, template_id: Option<Uuid>, trigger_rule_id: Option<Uuid>,
    channel: &NotificationChannel, subject: Option<&str>, body: &str,
    rendered_data: Option<&serde_json::Value>, event_type: Option<&TriggerEvent>,
    event_payload: Option<&serde_json::Value>, reference_type: Option<&str>, reference_id: Option<Uuid>,
) -> Result<Notification, sqlx::Error> {
    let status = match channel {
        NotificationChannel::InApp => NotificationStatus::Delivered,
        _ => NotificationStatus::Pending,
    };
    sqlx::query_as::<_, Notification>(
        r#"INSERT INTO notifications
        (user_id, template_id, trigger_rule_id, channel, subject, body, rendered_data,
         status, event_type, event_payload, reference_type, reference_id,
         delivered_at)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,
                CASE WHEN $4='in_app'::notification_channel THEN NOW() ELSE NULL END)
        RETURNING *"#,
    )
    .bind(user_id).bind(template_id).bind(trigger_rule_id).bind(channel)
    .bind(subject).bind(body).bind(rendered_data).bind(&status)
    .bind(event_type).bind(event_payload).bind(reference_type).bind(reference_id)
    .fetch_one(pool).await
}

pub async fn get_notifications(
    pool: &PgPool, user_id: Uuid, status: Option<&NotificationStatus>,
    channel: Option<&NotificationChannel>, limit: i64, offset: i64,
) -> Result<Vec<Notification>, sqlx::Error> {
    let mut sql = "SELECT * FROM notifications WHERE user_id=$1".to_string();
    let mut idx = 2u32;
    if status.is_some() { sql.push_str(&format!(" AND status=${}", idx)); idx += 1; }
    if channel.is_some() { sql.push_str(&format!(" AND channel=${}", idx)); idx += 1; }
    sql.push_str(&format!(" ORDER BY created_at DESC LIMIT ${} OFFSET ${}", idx, idx + 1));

    let mut q = sqlx::query_as::<_, Notification>(&sql).bind(user_id);
    if let Some(s) = status { q = q.bind(s); }
    if let Some(c) = channel { q = q.bind(c); }
    q = q.bind(limit).bind(offset);
    q.fetch_all(pool).await
}

pub async fn count_unread_notifications(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE user_id=$1 AND status IN ('pending','delivered')",
    ).bind(user_id).fetch_one(pool).await
}

pub async fn mark_notification_read(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE notifications SET status='read'::notification_status, read_at=NOW() WHERE id=$1 AND user_id=$2")
        .bind(id).bind(user_id).execute(pool).await?;
    Ok(())
}

pub async fn dismiss_notification(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE notifications SET status='dismissed'::notification_status, dismissed_at=NOW() WHERE id=$1 AND user_id=$2")
        .bind(id).bind(user_id).execute(pool).await?;
    Ok(())
}

pub async fn mark_all_notifications_read(pool: &PgPool, user_id: Uuid) -> Result<u64, sqlx::Error> {
    let r = sqlx::query(
        "UPDATE notifications SET status='read'::notification_status, read_at=NOW() WHERE user_id=$1 AND status IN ('pending','delivered')",
    ).bind(user_id).execute(pool).await?;
    Ok(r.rows_affected())
}

// ── External Payloads ───────────────────────────────────────

pub async fn create_external_payload(
    pool: &PgPool, notification_id: Option<Uuid>, channel: &NotificationChannel,
    recipient: &str, subject: Option<&str>, body: &str,
    metadata: Option<&serde_json::Value>,
) -> Result<ExternalPayload, sqlx::Error> {
    sqlx::query_as::<_, ExternalPayload>(
        r#"INSERT INTO external_payloads (notification_id, channel, recipient, subject, body, metadata)
        VALUES ($1,$2,$3,$4,$5,$6) RETURNING *"#,
    ).bind(notification_id).bind(channel).bind(recipient).bind(subject).bind(body).bind(metadata)
    .fetch_one(pool).await
}

pub async fn get_payload_queue(
    pool: &PgPool, status: Option<&PayloadStatus>, channel: Option<&NotificationChannel>,
    limit: i64, offset: i64,
) -> Result<Vec<ExternalPayload>, sqlx::Error> {
    let mut sql = "SELECT * FROM external_payloads WHERE 1=1".to_string();
    let mut idx = 0u32;
    if status.is_some() { idx += 1; sql.push_str(&format!(" AND status=${}", idx)); }
    if channel.is_some() { idx += 1; sql.push_str(&format!(" AND channel=${}", idx)); }
    idx += 1; let lim_idx = idx; idx += 1;
    sql.push_str(&format!(" ORDER BY created_at DESC LIMIT ${} OFFSET ${}", lim_idx, idx));

    let mut q = sqlx::query_as::<_, ExternalPayload>(&sql);
    if let Some(s) = status { q = q.bind(s); }
    if let Some(c) = channel { q = q.bind(c); }
    q = q.bind(limit).bind(offset);
    q.fetch_all(pool).await
}

pub async fn count_payloads_by_status(pool: &PgPool) -> Result<(i64, i64), sqlx::Error> {
    let queued: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM external_payloads WHERE status='queued'::payload_status",
    ).fetch_one(pool).await?;
    let failed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM external_payloads WHERE status='failed'::payload_status",
    ).fetch_one(pool).await?;
    Ok((queued, failed))
}

pub async fn update_payload_status(
    pool: &PgPool, id: Uuid, status: &PayloadStatus, error: Option<&str>,
) -> Result<ExternalPayload, sqlx::Error> {
    let next_retry = if *status == PayloadStatus::Retrying {
        Some(Utc::now() + Duration::minutes(15))
    } else { None };

    sqlx::query_as::<_, ExternalPayload>(
        r#"UPDATE external_payloads SET
        status=$2, last_error=$3,
        retry_count = CASE WHEN $2='retrying'::payload_status THEN retry_count+1 ELSE retry_count END,
        next_retry_at=$4, exported_at = CASE WHEN $2='exported'::payload_status THEN NOW() ELSE exported_at END,
        updated_at=NOW()
        WHERE id=$1 RETURNING *"#,
    ).bind(id).bind(status).bind(error).bind(next_retry).fetch_one(pool).await
}

pub async fn set_payload_export_path(pool: &PgPool, id: Uuid, path: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE external_payloads SET export_path=$2, exported_at=NOW(), status='exported'::payload_status, updated_at=NOW() WHERE id=$1")
        .bind(id).bind(path).execute(pool).await?;
    Ok(())
}

pub async fn get_retryable_payloads(pool: &PgPool) -> Result<Vec<ExternalPayload>, sqlx::Error> {
    sqlx::query_as::<_, ExternalPayload>(
        r#"SELECT * FROM external_payloads
        WHERE status='retrying'::payload_status AND retry_count < max_retries
          AND (next_retry_at IS NULL OR next_retry_at <= NOW())
        ORDER BY created_at ASC"#,
    ).fetch_all(pool).await
}

// ── Delivery Log ────────────────────────────────────────────

pub async fn log_delivery(
    pool: &PgPool, payload_id: Uuid, action: &str,
    status_before: Option<&PayloadStatus>, status_after: &PayloadStatus,
    details: Option<&str>, performed_by: Option<Uuid>,
) -> Result<DeliveryLogEntry, sqlx::Error> {
    sqlx::query_as::<_, DeliveryLogEntry>(
        r#"INSERT INTO delivery_log (payload_id, action, status_before, status_after, details, performed_by)
        VALUES ($1,$2,$3,$4,$5,$6) RETURNING *"#,
    ).bind(payload_id).bind(action).bind(status_before).bind(status_after)
    .bind(details).bind(performed_by)
    .fetch_one(pool).await
}

pub async fn get_delivery_log(pool: &PgPool, payload_id: Uuid) -> Result<Vec<DeliveryLogEntry>, sqlx::Error> {
    sqlx::query_as::<_, DeliveryLogEntry>(
        "SELECT * FROM delivery_log WHERE payload_id=$1 ORDER BY performed_at ASC",
    ).bind(payload_id).fetch_all(pool).await
}
