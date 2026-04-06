use sqlx::PgPool;
use uuid::Uuid;

use crate::db::messaging as msg_db;
use crate::errors::AppError;
use crate::messaging::template_engine::{payload_to_variables, render_template};
use crate::models::{
    FireEventResult, NotificationChannel, TriggerEvent, TriggerRule,
};

/// Process an event: find matching trigger rules, render templates, create notifications
/// and queue external payloads.
pub async fn fire_event(
    pool: &PgPool,
    event: &TriggerEvent,
    payload: &serde_json::Value,
    recipient_user_id: Option<Uuid>,
    reference_type: Option<&str>,
    reference_id: Option<Uuid>,
) -> Result<FireEventResult, AppError> {
    let rules = msg_db::get_rules_for_event(pool, event)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let mut notifications_created = 0usize;
    let mut payloads_queued = 0usize;

    let variables = payload_to_variables(payload);

    for rule in &rules {
        // Check conditions
        if !evaluate_conditions(&rule.conditions, payload) {
            continue;
        }

        // Resolve recipient
        let recipients = resolve_recipients(pool, recipient_user_id, rule.target_role.as_deref()).await?;

        // Get template
        let template = msg_db::get_template(pool, rule.template_id)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let template = match template {
            Some(t) if t.is_active => t,
            _ => continue,
        };

        let var_defs = msg_db::get_template_variables(pool, template.id)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Render template
        let rendered_body = render_template(&template.body_template, &variables, &var_defs)
            .unwrap_or_else(|_| template.body_template.clone());
        let rendered_subject = template.subject_template.as_ref().map(|s| {
            render_template(s, &variables, &var_defs).unwrap_or_else(|_| s.clone())
        });

        for user_id in &recipients {
            // Create in-app notification
            let notif = msg_db::create_notification(
                pool, *user_id, Some(template.id), Some(rule.id),
                &rule.channel, rendered_subject.as_deref(), &rendered_body,
                Some(payload), Some(event), Some(payload),
                reference_type, reference_id,
            )
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
            notifications_created += 1;

            // Queue external payload for non-in_app channels
            if rule.channel != NotificationChannel::InApp {
                let channel_body = match rule.channel {
                    NotificationChannel::Sms => {
                        template.sms_template.as_ref().map(|s| {
                            render_template(s, &variables, &var_defs).unwrap_or_else(|_| s.clone())
                        }).unwrap_or_else(|| rendered_body.clone())
                    }
                    NotificationChannel::Email => {
                        template.html_template.as_ref().map(|s| {
                            render_template(s, &variables, &var_defs).unwrap_or_else(|_| s.clone())
                        }).unwrap_or_else(|| rendered_body.clone())
                    }
                    _ => rendered_body.clone(),
                };

                // Recipient address from payload or placeholder
                let recipient_addr = variables.get("recipient_address")
                    .or_else(|| variables.get("email"))
                    .or_else(|| variables.get("phone"))
                    .cloned()
                    .unwrap_or_else(|| format!("user:{}", user_id));

                msg_db::create_external_payload(
                    pool, Some(notif.id), &rule.channel,
                    &recipient_addr, rendered_subject.as_deref(),
                    &channel_body, Some(payload),
                )
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;
                payloads_queued += 1;
            }
        }
    }

    Ok(FireEventResult {
        event: event.clone(),
        rules_matched: rules.len(),
        notifications_created,
        payloads_queued,
    })
}

/// Evaluate conditions against event payload.
/// Conditions is a JSON object of key-value pairs that must all match.
fn evaluate_conditions(conditions: &Option<serde_json::Value>, payload: &serde_json::Value) -> bool {
    match conditions {
        None => true,
        Some(conds) => {
            if let Some(cond_obj) = conds.as_object() {
                for (key, expected) in cond_obj {
                    let actual = payload.get(key);
                    match actual {
                        Some(val) if val == expected => continue,
                        _ => return false,
                    }
                }
                true
            } else {
                true
            }
        }
    }
}

/// Resolve notification recipients based on rule target
async fn resolve_recipients(
    pool: &PgPool,
    explicit_user: Option<Uuid>,
    target_role: Option<&str>,
) -> Result<Vec<Uuid>, AppError> {
    match (explicit_user, target_role) {
        (Some(uid), _) => Ok(vec![uid]),
        (None, Some(role)) => {
            // Find all active users with this role
            let users: Vec<Uuid> = sqlx::query_scalar(
                "SELECT id FROM users WHERE role::text=$1 AND status='active'::account_status",
            )
            .bind(role)
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
            Ok(users)
        }
        (None, None) => Ok(Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_conditions_none() {
        assert!(evaluate_conditions(&None, &serde_json::json!({})));
    }

    #[test]
    fn test_evaluate_conditions_match() {
        let conds = Some(serde_json::json!({"region": "north", "cycle": "daily"}));
        let payload = serde_json::json!({"region": "north", "cycle": "daily", "extra": "ok"});
        assert!(evaluate_conditions(&conds, &payload));
    }

    #[test]
    fn test_evaluate_conditions_mismatch() {
        let conds = Some(serde_json::json!({"region": "north"}));
        let payload = serde_json::json!({"region": "south"});
        assert!(!evaluate_conditions(&conds, &payload));
    }

    #[test]
    fn test_evaluate_conditions_missing_key() {
        let conds = Some(serde_json::json!({"region": "north"}));
        let payload = serde_json::json!({"other": "value"});
        assert!(!evaluate_conditions(&conds, &payload));
    }
}
