use chrono::{NaiveDate, NaiveTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    CreateSubtaskInput, ReminderStatus, ReminderType, SubmissionStatus, SubtaskResponse,
    SubmissionValidation, TaskCycle, TaskInstance, TaskInstanceStatus, TaskReminder,
    TaskSchedule, TaskSubmission, TaskTemplate, TemplateSubtask,
};

// ── Templates ───────────────────────────────────────────────

pub async fn create_template(
    pool: &PgPool,
    name: &str,
    description: Option<&str>,
    group_name: Option<&str>,
    cycle: &TaskCycle,
    window_start: NaiveTime,
    window_end: NaiveTime,
    allowed_misses: i32,
    miss_window_days: i32,
    makeup_allowed: bool,
    makeup_deadline_hours: i32,
    created_by: Option<Uuid>,
) -> Result<TaskTemplate, sqlx::Error> {
    sqlx::query_as::<_, TaskTemplate>(
        r#"
        INSERT INTO task_templates (
            name, description, group_name, cycle,
            time_window_start, time_window_end,
            allowed_misses, miss_window_days,
            makeup_allowed, makeup_deadline_hours, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING *
        "#,
    )
    .bind(name)
    .bind(description)
    .bind(group_name)
    .bind(cycle)
    .bind(window_start)
    .bind(window_end)
    .bind(allowed_misses)
    .bind(miss_window_days)
    .bind(makeup_allowed)
    .bind(makeup_deadline_hours)
    .bind(created_by)
    .fetch_one(pool)
    .await
}

pub async fn get_template(pool: &PgPool, id: Uuid) -> Result<Option<TaskTemplate>, sqlx::Error> {
    sqlx::query_as::<_, TaskTemplate>("SELECT * FROM task_templates WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list_templates(pool: &PgPool, active_only: bool) -> Result<Vec<TaskTemplate>, sqlx::Error> {
    if active_only {
        sqlx::query_as::<_, TaskTemplate>(
            "SELECT * FROM task_templates WHERE is_active = TRUE ORDER BY group_name, name",
        )
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, TaskTemplate>(
            "SELECT * FROM task_templates ORDER BY group_name, name",
        )
        .fetch_all(pool)
        .await
    }
}

pub async fn update_template(
    pool: &PgPool,
    id: Uuid,
    name: &str,
    description: Option<&str>,
    group_name: Option<&str>,
    cycle: &TaskCycle,
    window_start: NaiveTime,
    window_end: NaiveTime,
    allowed_misses: i32,
    miss_window_days: i32,
    makeup_allowed: bool,
    makeup_deadline_hours: i32,
) -> Result<TaskTemplate, sqlx::Error> {
    sqlx::query_as::<_, TaskTemplate>(
        r#"
        UPDATE task_templates SET
            name = $2, description = $3, group_name = $4, cycle = $5,
            time_window_start = $6, time_window_end = $7,
            allowed_misses = $8, miss_window_days = $9,
            makeup_allowed = $10, makeup_deadline_hours = $11,
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(group_name)
    .bind(cycle)
    .bind(window_start)
    .bind(window_end)
    .bind(allowed_misses)
    .bind(miss_window_days)
    .bind(makeup_allowed)
    .bind(makeup_deadline_hours)
    .fetch_one(pool)
    .await
}

pub async fn deactivate_template(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE task_templates SET is_active = FALSE, updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Subtasks ────────────────────────────────────────────────

pub async fn create_subtask(
    pool: &PgPool,
    template_id: Uuid,
    title: &str,
    description: Option<&str>,
    sort_order: i32,
    is_required: bool,
    expected_type: &str,
    options: Option<&serde_json::Value>,
) -> Result<TemplateSubtask, sqlx::Error> {
    sqlx::query_as::<_, TemplateSubtask>(
        r#"
        INSERT INTO template_subtasks (template_id, title, description, sort_order, is_required, expected_type, options)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(template_id)
    .bind(title)
    .bind(description)
    .bind(sort_order)
    .bind(is_required)
    .bind(expected_type)
    .bind(options)
    .fetch_one(pool)
    .await
}

pub async fn get_subtasks(pool: &PgPool, template_id: Uuid) -> Result<Vec<TemplateSubtask>, sqlx::Error> {
    sqlx::query_as::<_, TemplateSubtask>(
        "SELECT * FROM template_subtasks WHERE template_id = $1 ORDER BY sort_order",
    )
    .bind(template_id)
    .fetch_all(pool)
    .await
}

pub async fn delete_subtasks(pool: &PgPool, template_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM template_subtasks WHERE template_id = $1")
        .bind(template_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_subtasks(
    pool: &PgPool,
    template_id: Uuid,
    subtasks: &[CreateSubtaskInput],
) -> Result<Vec<TemplateSubtask>, sqlx::Error> {
    delete_subtasks(pool, template_id).await?;
    let mut results = Vec::new();
    for (i, s) in subtasks.iter().enumerate() {
        let st = create_subtask(
            pool,
            template_id,
            &s.title,
            s.description.as_deref(),
            s.sort_order.unwrap_or(i as i32),
            s.is_required.unwrap_or(true),
            s.expected_type.as_deref().unwrap_or("checkbox"),
            s.options.as_ref(),
        )
        .await?;
        results.push(st);
    }
    Ok(results)
}

// ── Schedules ───────────────────────────────────────────────

pub async fn create_schedule(
    pool: &PgPool,
    template_id: Uuid,
    assigned_to: Uuid,
    start_date: NaiveDate,
    end_date: Option<NaiveDate>,
    notes: Option<&str>,
    created_by: Option<Uuid>,
) -> Result<TaskSchedule, sqlx::Error> {
    sqlx::query_as::<_, TaskSchedule>(
        r#"
        INSERT INTO task_schedules (template_id, assigned_to, start_date, end_date, notes, created_by)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(template_id)
    .bind(assigned_to)
    .bind(start_date)
    .bind(end_date)
    .bind(notes)
    .bind(created_by)
    .fetch_one(pool)
    .await
}

pub async fn get_schedule(pool: &PgPool, id: Uuid) -> Result<Option<TaskSchedule>, sqlx::Error> {
    sqlx::query_as::<_, TaskSchedule>("SELECT * FROM task_schedules WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list_schedules_for_user(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<TaskSchedule>, sqlx::Error> {
    sqlx::query_as::<_, TaskSchedule>(
        "SELECT * FROM task_schedules WHERE assigned_to = $1 AND is_active = TRUE ORDER BY start_date",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn deactivate_schedule(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE task_schedules SET is_active = FALSE, updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Task Instances ──────────────────────────────────────────

pub async fn create_instance(
    pool: &PgPool,
    schedule_id: Uuid,
    template_id: Uuid,
    assigned_to: Uuid,
    due_date: NaiveDate,
    window_start: NaiveTime,
    window_end: NaiveTime,
    is_makeup: bool,
    original_instance_id: Option<Uuid>,
    makeup_deadline: Option<chrono::DateTime<Utc>>,
) -> Result<TaskInstance, sqlx::Error> {
    sqlx::query_as::<_, TaskInstance>(
        r#"
        INSERT INTO task_instances (
            schedule_id, template_id, assigned_to, due_date,
            window_start, window_end, is_makeup,
            original_instance_id, makeup_deadline
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(schedule_id)
    .bind(template_id)
    .bind(assigned_to)
    .bind(due_date)
    .bind(window_start)
    .bind(window_end)
    .bind(is_makeup)
    .bind(original_instance_id)
    .bind(makeup_deadline)
    .fetch_one(pool)
    .await
}

pub async fn get_instance(pool: &PgPool, id: Uuid) -> Result<Option<TaskInstance>, sqlx::Error> {
    sqlx::query_as::<_, TaskInstance>("SELECT * FROM task_instances WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn update_instance_status(
    pool: &PgPool,
    id: Uuid,
    status: &TaskInstanceStatus,
) -> Result<TaskInstance, sqlx::Error> {
    sqlx::query_as::<_, TaskInstance>(
        r#"
        UPDATE task_instances SET
            status = $2,
            started_at = CASE WHEN $2 = 'in_progress'::task_instance_status AND started_at IS NULL THEN NOW() ELSE started_at END,
            completed_at = CASE WHEN $2 IN ('completed'::task_instance_status, 'submitted'::task_instance_status) THEN NOW() ELSE completed_at END,
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(status)
    .fetch_one(pool)
    .await
}

pub async fn list_instances_for_user(
    pool: &PgPool,
    user_id: Uuid,
    status: Option<&TaskInstanceStatus>,
    from_date: Option<NaiveDate>,
    to_date: Option<NaiveDate>,
    limit: i64,
    offset: i64,
) -> Result<Vec<TaskInstance>, sqlx::Error> {
    let mut sql = String::from("SELECT * FROM task_instances WHERE assigned_to = $1");
    let mut param_idx = 2u32;

    if status.is_some() {
        sql.push_str(&format!(" AND status = ${}", param_idx));
        param_idx += 1;
    }
    if from_date.is_some() {
        sql.push_str(&format!(" AND due_date >= ${}", param_idx));
        param_idx += 1;
    }
    if to_date.is_some() {
        sql.push_str(&format!(" AND due_date <= ${}", param_idx));
        param_idx += 1;
    }

    sql.push_str(&format!(" ORDER BY due_date ASC, window_start ASC LIMIT ${} OFFSET ${}", param_idx, param_idx + 1));

    let mut query = sqlx::query_as::<_, TaskInstance>(&sql).bind(user_id);
    if let Some(s) = status {
        query = query.bind(s);
    }
    if let Some(fd) = from_date {
        query = query.bind(fd);
    }
    if let Some(td) = to_date {
        query = query.bind(td);
    }
    query = query.bind(limit).bind(offset);
    query.fetch_all(pool).await
}

pub async fn count_instances_for_user(
    pool: &PgPool,
    user_id: Uuid,
    status: Option<&TaskInstanceStatus>,
    from_date: Option<NaiveDate>,
    to_date: Option<NaiveDate>,
) -> Result<i64, sqlx::Error> {
    let mut sql = String::from("SELECT COUNT(*) FROM task_instances WHERE assigned_to = $1");
    let mut param_idx = 2u32;

    if status.is_some() {
        sql.push_str(&format!(" AND status = ${}", param_idx));
        param_idx += 1;
    }
    if from_date.is_some() {
        sql.push_str(&format!(" AND due_date >= ${}", param_idx));
        param_idx += 1;
    }
    if to_date.is_some() {
        sql.push_str(&format!(" AND due_date <= ${}", param_idx));
    }

    let mut query = sqlx::query_scalar::<_, i64>(&sql).bind(user_id);
    if let Some(s) = status {
        query = query.bind(s);
    }
    if let Some(fd) = from_date {
        query = query.bind(fd);
    }
    if let Some(td) = to_date {
        query = query.bind(td);
    }
    query.fetch_one(pool).await
}

/// Find overdue scheduled/in_progress instances
pub async fn find_overdue_instances(pool: &PgPool) -> Result<Vec<TaskInstance>, sqlx::Error> {
    sqlx::query_as::<_, TaskInstance>(
        r#"
        SELECT * FROM task_instances
        WHERE status IN ('scheduled'::task_instance_status, 'in_progress'::task_instance_status)
          AND due_date < CURRENT_DATE
        ORDER BY due_date ASC
        "#,
    )
    .fetch_all(pool)
    .await
}

/// Count misses within the fault-tolerance window for a schedule
pub async fn count_misses_in_window(
    pool: &PgPool,
    schedule_id: Uuid,
    window_days: i32,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM task_instances
        WHERE schedule_id = $1
          AND status = 'missed'::task_instance_status
          AND due_date >= CURRENT_DATE - $2::integer
        "#,
    )
    .bind(schedule_id)
    .bind(window_days)
    .fetch_one(pool)
    .await
}

/// Check if a makeup instance already exists for the original
pub async fn has_makeup_instance(pool: &PgPool, original_id: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM task_instances WHERE original_instance_id = $1)",
    )
    .bind(original_id)
    .fetch_one(pool)
    .await
}

/// Check if instance for this schedule+due_date already exists
pub async fn instance_exists(
    pool: &PgPool,
    schedule_id: Uuid,
    due_date: NaiveDate,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM task_instances
            WHERE schedule_id = $1 AND due_date = $2 AND is_makeup = FALSE
        )
        "#,
    )
    .bind(schedule_id)
    .bind(due_date)
    .fetch_one(pool)
    .await
}

// ── Submissions ─────────────────────────────────────────────

/// Create submission with encrypted notes. The `encrypted_notes` param holds
/// the already-encrypted ciphertext. Plaintext `notes` is set to NULL for new
/// writes to prevent sensitive data from being stored unencrypted.
pub async fn create_submission(
    pool: &PgPool,
    instance_id: Uuid,
    submitted_by: Uuid,
    encrypted_notes: Option<&str>,
) -> Result<TaskSubmission, sqlx::Error> {
    sqlx::query_as::<_, TaskSubmission>(
        r#"
        INSERT INTO task_submissions (instance_id, submitted_by, notes, encrypted_notes)
        VALUES ($1, $2, NULL, $3)
        RETURNING *
        "#,
    )
    .bind(instance_id)
    .bind(submitted_by)
    .bind(encrypted_notes)
    .fetch_one(pool)
    .await
}

pub async fn get_submission(pool: &PgPool, id: Uuid) -> Result<Option<TaskSubmission>, sqlx::Error> {
    sqlx::query_as::<_, TaskSubmission>("SELECT * FROM task_submissions WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_submission_for_instance(
    pool: &PgPool,
    instance_id: Uuid,
) -> Result<Option<TaskSubmission>, sqlx::Error> {
    sqlx::query_as::<_, TaskSubmission>(
        "SELECT * FROM task_submissions WHERE instance_id = $1 ORDER BY submitted_at DESC LIMIT 1",
    )
    .bind(instance_id)
    .fetch_optional(pool)
    .await
}

pub async fn review_submission(
    pool: &PgPool,
    id: Uuid,
    status: &SubmissionStatus,
    reviewed_by: Uuid,
    review_notes: Option<&str>,
) -> Result<TaskSubmission, sqlx::Error> {
    sqlx::query_as::<_, TaskSubmission>(
        r#"
        UPDATE task_submissions SET
            status = $2, reviewed_by = $3, reviewed_at = NOW(), review_notes = $4
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(status)
    .bind(reviewed_by)
    .bind(review_notes)
    .fetch_one(pool)
    .await
}

// ── Subtask Responses ───────────────────────────────────────

pub async fn create_subtask_response(
    pool: &PgPool,
    submission_id: Uuid,
    subtask_id: Uuid,
    response_value: &serde_json::Value,
    is_valid: bool,
    validation_msg: Option<&str>,
) -> Result<SubtaskResponse, sqlx::Error> {
    sqlx::query_as::<_, SubtaskResponse>(
        r#"
        INSERT INTO subtask_responses (submission_id, subtask_id, response_value, is_valid, validation_msg)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(submission_id)
    .bind(subtask_id)
    .bind(response_value)
    .bind(is_valid)
    .bind(validation_msg)
    .fetch_one(pool)
    .await
}

pub async fn get_responses_for_submission(
    pool: &PgPool,
    submission_id: Uuid,
) -> Result<Vec<SubtaskResponse>, sqlx::Error> {
    sqlx::query_as::<_, SubtaskResponse>(
        "SELECT * FROM subtask_responses WHERE submission_id = $1 ORDER BY responded_at",
    )
    .bind(submission_id)
    .fetch_all(pool)
    .await
}

// ── Validations ─────────────────────────────────────────────

pub async fn create_validation(
    pool: &PgPool,
    submission_id: Uuid,
    field_name: &str,
    is_valid: bool,
    message: Option<&str>,
    severity: &str,
) -> Result<SubmissionValidation, sqlx::Error> {
    sqlx::query_as::<_, SubmissionValidation>(
        r#"
        INSERT INTO submission_validations (submission_id, field_name, is_valid, message, severity)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(submission_id)
    .bind(field_name)
    .bind(is_valid)
    .bind(message)
    .bind(severity)
    .fetch_one(pool)
    .await
}

pub async fn get_validations_for_submission(
    pool: &PgPool,
    submission_id: Uuid,
) -> Result<Vec<SubmissionValidation>, sqlx::Error> {
    sqlx::query_as::<_, SubmissionValidation>(
        "SELECT * FROM submission_validations WHERE submission_id = $1 ORDER BY validated_at",
    )
    .bind(submission_id)
    .fetch_all(pool)
    .await
}

// ── Reminders ───────────────────────────────────────────────

pub async fn create_reminder(
    pool: &PgPool,
    user_id: Uuid,
    instance_id: Option<Uuid>,
    reminder_type: &ReminderType,
    title: &str,
    message: &str,
    due_date: Option<NaiveDate>,
) -> Result<TaskReminder, sqlx::Error> {
    sqlx::query_as::<_, TaskReminder>(
        r#"
        INSERT INTO task_reminders (user_id, instance_id, reminder_type, title, message, due_date)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(instance_id)
    .bind(reminder_type)
    .bind(title)
    .bind(message)
    .bind(due_date)
    .fetch_one(pool)
    .await
}

pub async fn get_reminders(
    pool: &PgPool,
    user_id: Uuid,
    status: Option<&ReminderStatus>,
    reminder_type: Option<&ReminderType>,
    limit: i64,
    offset: i64,
) -> Result<Vec<TaskReminder>, sqlx::Error> {
    let mut sql = String::from("SELECT * FROM task_reminders WHERE user_id = $1");
    let mut param_idx = 2u32;

    if status.is_some() {
        sql.push_str(&format!(" AND status = ${}", param_idx));
        param_idx += 1;
    }
    if reminder_type.is_some() {
        sql.push_str(&format!(" AND reminder_type = ${}", param_idx));
        param_idx += 1;
    }

    sql.push_str(&format!(" ORDER BY created_at DESC LIMIT ${} OFFSET ${}", param_idx, param_idx + 1));

    let mut query = sqlx::query_as::<_, TaskReminder>(&sql).bind(user_id);
    if let Some(s) = status {
        query = query.bind(s);
    }
    if let Some(rt) = reminder_type {
        query = query.bind(rt);
    }
    query = query.bind(limit).bind(offset);
    query.fetch_all(pool).await
}

pub async fn count_unread_reminders(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM task_reminders WHERE user_id = $1 AND status = 'unread'::reminder_status",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
}

pub async fn mark_reminder_read(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE task_reminders SET status = 'read'::reminder_status, read_at = NOW() WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn dismiss_reminder(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE task_reminders SET status = 'dismissed'::reminder_status, dismissed_at = NOW() WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_all_read(pool: &PgPool, user_id: Uuid) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE task_reminders SET status = 'read'::reminder_status, read_at = NOW() WHERE user_id = $1 AND status = 'unread'::reminder_status",
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}
