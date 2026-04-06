use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::inspection as insp_db;
use crate::errors::AppError;
use crate::middleware::auth_middleware::{authenticate_request, require_role};
use crate::middleware::audit_middleware::audit_action;
use crate::middleware::rate_limit_middleware::apply_rate_limit;
use crate::models::*;
use crate::scheduling::{engine, validation};

fn get_ip(req: &HttpRequest) -> Option<String> {
    req.peer_addr().map(|a| a.ip().to_string())
}

fn get_user_agent(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("User-Agent")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
}

fn parse_time(s: &str) -> Result<chrono::NaiveTime, AppError> {
    chrono::NaiveTime::parse_from_str(s, "%H:%M")
        .or_else(|_| chrono::NaiveTime::parse_from_str(s, "%H:%M:%S"))
        .map_err(|_| AppError::BadRequest(format!("Invalid time format '{}', expected HH:MM", s)))
}

// ═══════════════════════════════════════════════════════════
// TEMPLATES
// ═══════════════════════════════════════════════════════════

/// POST /api/inspection/templates
pub async fn create_template(
    pool: web::Data<PgPool>,
    body: web::Json<CreateTemplateRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth_user, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    let ws = parse_time(body.time_window_start.as_deref().unwrap_or("08:00"))?;
    let we = parse_time(body.time_window_end.as_deref().unwrap_or("18:00"))?;

    let template = insp_db::create_template(
        pool.get_ref(),
        &body.name,
        body.description.as_deref(),
        body.group_name.as_deref(),
        &body.cycle,
        ws,
        we,
        body.allowed_misses.unwrap_or(1),
        body.miss_window_days.unwrap_or(30),
        body.makeup_allowed.unwrap_or(true),
        body.makeup_deadline_hours.unwrap_or(48),
        Some(auth_user.user_id),
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Create subtasks if provided
    let subtasks = if let Some(ref subs) = body.subtasks {
        insp_db::set_subtasks(pool.get_ref(), template.id, subs)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
    } else {
        Vec::new()
    };

    audit_action(
        pool.get_ref(),
        &auth_user,
        "template_created",
        Some("task_template"),
        Some(&template.id.to_string()),
        Some(serde_json::json!({"name": &body.name, "cycle": &body.cycle})),
        get_ip(&req).as_deref(),
        get_user_agent(&req).as_deref(),
    )
    .await?;

    Ok(HttpResponse::Created().json(TemplateWithSubtasks { template, subtasks }))
}

/// GET /api/inspection/templates
pub async fn list_templates(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    let templates = insp_db::list_templates(pool.get_ref(), true)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(templates))
}

/// GET /api/inspection/templates/{id}
pub async fn get_template(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    let id = path.into_inner();
    let template = insp_db::get_template(pool.get_ref(), id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Template not found".to_string()))?;

    let subtasks = insp_db::get_subtasks(pool.get_ref(), id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(TemplateWithSubtasks { template, subtasks }))
}

/// PUT /api/inspection/templates/{id}
pub async fn update_template(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    body: web::Json<UpdateTemplateRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth_user, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;

    let id = path.into_inner();
    let existing = insp_db::get_template(pool.get_ref(), id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Template not found".to_string()))?;

    let name = body.name.as_deref().unwrap_or(&existing.name);
    let cycle = body.cycle.as_ref().unwrap_or(&existing.cycle);
    let ws = match &body.time_window_start {
        Some(s) => parse_time(s)?,
        None => existing.time_window_start,
    };
    let we = match &body.time_window_end {
        Some(s) => parse_time(s)?,
        None => existing.time_window_end,
    };

    let template = insp_db::update_template(
        pool.get_ref(),
        id,
        name,
        body.description.as_deref().or(existing.description.as_deref()),
        body.group_name.as_deref().or(existing.group_name.as_deref()),
        cycle,
        ws,
        we,
        body.allowed_misses.unwrap_or(existing.allowed_misses),
        body.miss_window_days.unwrap_or(existing.miss_window_days),
        body.makeup_allowed.unwrap_or(existing.makeup_allowed),
        body.makeup_deadline_hours.unwrap_or(existing.makeup_deadline_hours),
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    audit_action(
        pool.get_ref(),
        &auth_user,
        "template_updated",
        Some("task_template"),
        Some(&id.to_string()),
        None,
        get_ip(&req).as_deref(),
        get_user_agent(&req).as_deref(),
    )
    .await?;

    let subtasks = insp_db::get_subtasks(pool.get_ref(), id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(TemplateWithSubtasks { template, subtasks }))
}

/// PUT /api/inspection/templates/{id}/subtasks
pub async fn set_subtasks(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    body: web::Json<Vec<CreateSubtaskInput>>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth_user, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;

    let template_id = path.into_inner();

    // Verify template exists
    insp_db::get_template(pool.get_ref(), template_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Template not found".to_string()))?;

    let subtasks = insp_db::set_subtasks(pool.get_ref(), template_id, &body)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    audit_action(
        pool.get_ref(),
        &auth_user,
        "template_subtasks_updated",
        Some("task_template"),
        Some(&template_id.to_string()),
        Some(serde_json::json!({"subtask_count": subtasks.len()})),
        get_ip(&req).as_deref(),
        get_user_agent(&req).as_deref(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(subtasks))
}

/// DELETE /api/inspection/templates/{id}
pub async fn deactivate_template(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth_user, &[UserRole::OperationsAdmin])?;

    let id = path.into_inner();
    insp_db::deactivate_template(pool.get_ref(), id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    audit_action(
        pool.get_ref(),
        &auth_user,
        "template_deactivated",
        Some("task_template"),
        Some(&id.to_string()),
        None,
        get_ip(&req).as_deref(),
        get_user_agent(&req).as_deref(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Template deactivated"})))
}

// ═══════════════════════════════════════════════════════════
// SCHEDULES
// ═══════════════════════════════════════════════════════════

/// POST /api/inspection/schedules
pub async fn create_schedule(
    pool: web::Data<PgPool>,
    body: web::Json<CreateScheduleRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth_user, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    // Verify template
    let template = insp_db::get_template(pool.get_ref(), body.template_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Template not found".to_string()))?;

    let schedule = insp_db::create_schedule(
        pool.get_ref(),
        body.template_id,
        body.assigned_to,
        body.start_date,
        body.end_date,
        body.notes.as_deref(),
        Some(auth_user.user_id),
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Auto-generate instances for the next 30 days
    let instances = engine::generate_instances(pool.get_ref(), &schedule, &template, 30).await?;

    audit_action(
        pool.get_ref(),
        &auth_user,
        "schedule_created",
        Some("task_schedule"),
        Some(&schedule.id.to_string()),
        Some(serde_json::json!({
            "template_id": body.template_id,
            "assigned_to": body.assigned_to,
            "instances_generated": instances.len(),
        })),
        get_ip(&req).as_deref(),
        get_user_agent(&req).as_deref(),
    )
    .await?;

    Ok(HttpResponse::Created().json(serde_json::json!({
        "schedule": schedule,
        "instances_generated": instances.len(),
    })))
}

/// GET /api/inspection/schedules
pub async fn list_schedules(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    let schedules = insp_db::list_schedules_for_user(pool.get_ref(), auth_user.user_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(schedules))
}

// ═══════════════════════════════════════════════════════════
// TASK INSTANCES
// ═══════════════════════════════════════════════════════════

/// GET /api/inspection/tasks
pub async fn list_tasks(
    pool: web::Data<PgPool>,
    query: web::Query<TaskListQuery>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).min(100);
    let offset = (page - 1) * page_size;

    let instances = insp_db::list_instances_for_user(
        pool.get_ref(),
        auth_user.user_id,
        query.status.as_ref(),
        query.from_date,
        query.to_date,
        page_size,
        offset,
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let total = insp_db::count_instances_for_user(
        pool.get_ref(),
        auth_user.user_id,
        query.status.as_ref(),
        query.from_date,
        query.to_date,
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Enrich with template details
    let mut details = Vec::new();
    for inst in instances {
        let template = insp_db::get_template(pool.get_ref(), inst.template_id)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let subtasks = insp_db::get_subtasks(pool.get_ref(), inst.template_id)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let submission = insp_db::get_submission_for_instance(pool.get_ref(), inst.id)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        details.push(TaskInstanceDetail {
            template_name: template.as_ref().map(|t| t.name.clone()).unwrap_or_default(),
            template_group: template.as_ref().and_then(|t| t.group_name.clone()),
            instance: inst,
            subtasks,
            submission,
        });
    }

    Ok(HttpResponse::Ok().json(TaskListResponse {
        tasks: details,
        total,
        page,
        page_size,
    }))
}

/// GET /api/inspection/tasks/{id}
pub async fn get_task(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    let id = path.into_inner();
    let instance = insp_db::get_instance(pool.get_ref(), id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Task instance not found".to_string()))?;

    // Verify ownership or admin
    if instance.assigned_to != auth_user.user_id {
        require_role(&auth_user, &[UserRole::OperationsAdmin, UserRole::DepartmentManager, UserRole::Reviewer])?;
    }

    let template = insp_db::get_template(pool.get_ref(), instance.template_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let subtasks = insp_db::get_subtasks(pool.get_ref(), instance.template_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let submission = insp_db::get_submission_for_instance(pool.get_ref(), id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(TaskInstanceDetail {
        template_name: template.as_ref().map(|t| t.name.clone()).unwrap_or_default(),
        template_group: template.as_ref().and_then(|t| t.group_name.clone()),
        instance,
        subtasks,
        submission,
    }))
}

/// POST /api/inspection/tasks/{id}/start
pub async fn start_task(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;

    let id = path.into_inner();
    let instance = insp_db::get_instance(pool.get_ref(), id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Task instance not found".to_string()))?;

    if instance.assigned_to != auth_user.user_id {
        return Err(AppError::Forbidden);
    }

    if instance.status != TaskInstanceStatus::Scheduled && instance.status != TaskInstanceStatus::Makeup {
        return Err(AppError::BadRequest(format!(
            "Cannot start task in '{}' status",
            serde_json::to_string(&instance.status).unwrap_or_default()
        )));
    }

    let updated = insp_db::update_instance_status(pool.get_ref(), id, &TaskInstanceStatus::InProgress)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    audit_action(
        pool.get_ref(), &auth_user, "task_started", Some("task_instance"),
        Some(&id.to_string()), None,
        get_ip(&req).as_deref(), get_user_agent(&req).as_deref(),
    ).await?;

    Ok(HttpResponse::Ok().json(updated))
}

// ═══════════════════════════════════════════════════════════
// SUBMISSIONS
// ═══════════════════════════════════════════════════════════

/// POST /api/inspection/submissions
pub async fn submit_task(
    pool: web::Data<PgPool>,
    body: web::Json<CreateSubmissionRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    let instance = insp_db::get_instance(pool.get_ref(), body.instance_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Task instance not found".to_string()))?;

    if instance.assigned_to != auth_user.user_id {
        return Err(AppError::Forbidden);
    }

    if instance.status != TaskInstanceStatus::InProgress
        && instance.status != TaskInstanceStatus::Scheduled
        && instance.status != TaskInstanceStatus::Makeup
    {
        return Err(AppError::BadRequest(
            "Task is not in a submittable state".to_string(),
        ));
    }

    // Get subtasks for validation
    let subtasks = insp_db::get_subtasks(pool.get_ref(), instance.template_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Validate immediately
    let validation_result = validation::validate_submission(&subtasks, &body.responses);

    if !validation_result.is_valid {
        // Return validation errors immediately without creating submission
        return Ok(HttpResponse::UnprocessableEntity().json(serde_json::json!({
            "valid": false,
            "validation": validation_result,
        })));
    }

    // Encrypt sensitive notes at rest — fail explicitly if encryption unavailable
    let stored_notes = if let Some(ref notes) = body.notes {
        if notes.is_empty() {
            Some(String::new())
        } else {
            let encrypted = crate::encryption::field_encryption::encrypt_field(pool.get_ref(), notes)
                .await
                .map_err(|e| {
                    log::error!("Encryption failed for submission notes: {}", e);
                    AppError::InternalError("Cannot store sensitive data: encryption unavailable".to_string())
                })?;
            Some(encrypted)
        }
    } else {
        None
    };

    // Create submission
    let submission = insp_db::create_submission(
        pool.get_ref(),
        body.instance_id,
        auth_user.user_id,
        stored_notes.as_deref(),
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Save individual subtask responses
    for resp in &body.responses {
        let subtask = subtasks.iter().find(|s| s.id == resp.subtask_id);
        insp_db::create_subtask_response(
            pool.get_ref(),
            submission.id,
            resp.subtask_id,
            &resp.response_value,
            true,
            None,
        )
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    // Persist validation results
    validation::persist_validation(pool.get_ref(), submission.id, &validation_result).await?;

    // Update instance status
    insp_db::update_instance_status(pool.get_ref(), instance.id, &TaskInstanceStatus::Submitted)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    audit_action(
        pool.get_ref(),
        &auth_user,
        "task_submitted",
        Some("task_instance"),
        Some(&instance.id.to_string()),
        Some(serde_json::json!({
            "submission_id": submission.id,
            "response_count": body.responses.len(),
        })),
        get_ip(&req).as_deref(),
        get_user_agent(&req).as_deref(),
    )
    .await?;

    // Get full submission detail
    let responses = insp_db::get_responses_for_submission(pool.get_ref(), submission.id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    let validations = insp_db::get_validations_for_submission(pool.get_ref(), submission.id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Created().json(serde_json::json!({
        "valid": true,
        "validation": validation_result,
        "submission": SubmissionDetail {
            submission,
            responses,
            validations,
        }
    })))
}

/// GET /api/inspection/submissions/{id}
pub async fn get_submission(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;

    let id = path.into_inner();
    let submission = insp_db::get_submission(pool.get_ref(), id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Submission not found".to_string()))?;

    if submission.submitted_by != auth_user.user_id {
        require_role(&auth_user, &[UserRole::Reviewer, UserRole::OperationsAdmin, UserRole::DepartmentManager])?;
    }

    // Decrypt notes for authorized read: prefer encrypted_notes, fall back to legacy plaintext
    let mut response_submission = submission.clone();
    if let Some(ref enc) = submission.encrypted_notes {
        if !enc.is_empty() {
            match crate::encryption::field_encryption::decrypt_field(pool.get_ref(), enc).await {
                Ok(decrypted) => { response_submission.notes = Some(decrypted); }
                Err(_) => { response_submission.notes = Some("[encrypted]".to_string()); }
            }
        }
    }
    // notes field from legacy rows passes through as-is

    let responses = insp_db::get_responses_for_submission(pool.get_ref(), id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    let validations = insp_db::get_validations_for_submission(pool.get_ref(), id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(SubmissionDetail {
        submission: response_submission,
        responses,
        validations,
    }))
}

/// PUT /api/inspection/submissions/{id}/review
pub async fn review_submission(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    body: web::Json<ReviewSubmissionRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth_user, &[UserRole::Reviewer, UserRole::OperationsAdmin])?;

    let id = path.into_inner();
    let submission = insp_db::review_submission(
        pool.get_ref(),
        id,
        &body.status,
        auth_user.user_id,
        body.review_notes.as_deref(),
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // If approved, mark instance as completed
    if body.status == SubmissionStatus::Approved {
        insp_db::update_instance_status(
            pool.get_ref(),
            submission.instance_id,
            &TaskInstanceStatus::Completed,
        )
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    audit_action(
        pool.get_ref(),
        &auth_user,
        "submission_reviewed",
        Some("task_submission"),
        Some(&id.to_string()),
        Some(serde_json::json!({"status": body.status, "review_notes": body.review_notes})),
        get_ip(&req).as_deref(),
        get_user_agent(&req).as_deref(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(submission))
}

// ═══════════════════════════════════════════════════════════
// REMINDERS
// ═══════════════════════════════════════════════════════════

/// GET /api/inspection/reminders
pub async fn get_reminders(
    pool: web::Data<PgPool>,
    query: web::Query<ReminderQuery>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    apply_rate_limit(pool.get_ref(), auth_user.user_id).await?;

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).min(100);
    let offset = (page - 1) * page_size;

    let reminders = insp_db::get_reminders(
        pool.get_ref(),
        auth_user.user_id,
        query.status.as_ref(),
        query.reminder_type.as_ref(),
        page_size,
        offset,
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let unread_count = insp_db::count_unread_reminders(pool.get_ref(), auth_user.user_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // rough total count using unread + others
    let total = reminders.len() as i64 + offset;

    Ok(HttpResponse::Ok().json(ReminderInbox {
        unread_count,
        reminders,
        total,
        page,
        page_size,
    }))
}

/// POST /api/inspection/reminders/{id}/read
pub async fn mark_read(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    let id = path.into_inner();

    insp_db::mark_reminder_read(pool.get_ref(), id, auth_user.user_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Marked as read"})))
}

/// POST /api/inspection/reminders/{id}/dismiss
pub async fn dismiss_reminder(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    let id = path.into_inner();

    insp_db::dismiss_reminder(pool.get_ref(), id, auth_user.user_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Dismissed"})))
}

/// POST /api/inspection/reminders/read-all
pub async fn mark_all_read(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;

    let count = insp_db::mark_all_read(pool.get_ref(), auth_user.user_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"marked_read": count})))
}

// ═══════════════════════════════════════════════════════════
// SCHEDULING ENGINE
// ═══════════════════════════════════════════════════════════

/// POST /api/inspection/generate-instances  (admin: trigger instance generation)
pub async fn generate_instances(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth_user, &[UserRole::OperationsAdmin])?;

    // Generate for all active schedules
    let schedules: Vec<TaskSchedule> = sqlx::query_as(
        "SELECT * FROM task_schedules WHERE is_active = TRUE",
    )
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let mut total_generated = 0usize;
    for schedule in &schedules {
        let template = insp_db::get_template(pool.get_ref(), schedule.template_id)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if let Some(ref t) = template {
            let instances = engine::generate_instances(pool.get_ref(), schedule, t, 30).await?;
            total_generated += instances.len();
        }
    }

    // Process overdue
    let overdue_report = engine::process_overdue(pool.get_ref()).await?;

    // Generate upcoming reminders
    let reminder_count = engine::generate_upcoming_reminders(pool.get_ref()).await?;

    audit_action(
        pool.get_ref(),
        &auth_user,
        "instances_generated",
        Some("scheduling"),
        None,
        Some(serde_json::json!({
            "schedules_processed": schedules.len(),
            "instances_generated": total_generated,
            "overdue_report": overdue_report,
            "reminders_generated": reminder_count,
        })),
        get_ip(&req).as_deref(),
        get_user_agent(&req).as_deref(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "schedules_processed": schedules.len(),
        "instances_generated": total_generated,
        "overdue_report": overdue_report,
        "reminders_generated": reminder_count,
    })))
}

/// POST /api/inspection/process-overdue  (admin: trigger overdue processing)
pub async fn process_overdue(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth_user = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth_user, &[UserRole::OperationsAdmin])?;

    let report = engine::process_overdue(pool.get_ref()).await?;

    Ok(HttpResponse::Ok().json(report))
}

// ═══════════════════════════════════════════════════════════
// ROUTE CONFIG
// ═══════════════════════════════════════════════════════════

pub fn inspection_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/inspection")
            // Templates
            .route("/templates", web::post().to(create_template))
            .route("/templates", web::get().to(list_templates))
            .route("/templates/{id}", web::get().to(get_template))
            .route("/templates/{id}", web::put().to(update_template))
            .route("/templates/{id}", web::delete().to(deactivate_template))
            .route("/templates/{id}/subtasks", web::put().to(set_subtasks))
            // Schedules
            .route("/schedules", web::post().to(create_schedule))
            .route("/schedules", web::get().to(list_schedules))
            // Task instances
            .route("/tasks", web::get().to(list_tasks))
            .route("/tasks/{id}", web::get().to(get_task))
            .route("/tasks/{id}/start", web::post().to(start_task))
            // Submissions
            .route("/submissions", web::post().to(submit_task))
            .route("/submissions/{id}", web::get().to(get_submission))
            .route("/submissions/{id}/review", web::put().to(review_submission))
            // Reminders
            .route("/reminders", web::get().to(get_reminders))
            .route("/reminders/read-all", web::post().to(mark_all_read))
            .route("/reminders/{id}/read", web::post().to(mark_read))
            .route("/reminders/{id}/dismiss", web::post().to(dismiss_reminder))
            // Scheduling engine
            .route("/generate-instances", web::post().to(generate_instances))
            .route("/process-overdue", web::post().to(process_overdue)),
    );
}
