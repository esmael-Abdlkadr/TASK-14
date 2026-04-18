use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::review as review_db;
use crate::errors::{map_sqlx_unique_violation, AppError};
use crate::middleware::auth_middleware::{authenticate_request, require_role};
use crate::middleware::audit_middleware::audit_action;
use crate::middleware::rate_limit_middleware::apply_rate_limit;
use crate::models::*;
use crate::review::{assignment, consistency};

fn get_ip(req: &HttpRequest) -> Option<String> {
    req.peer_addr().map(|a| a.ip().to_string())
}
fn get_user_agent(req: &HttpRequest) -> Option<String> {
    req.headers().get("User-Agent").and_then(|v| v.to_str().ok()).map(String::from)
}

// ═══════════════════════════════════════════════════════════
// SCORECARDS
// ═══════════════════════════════════════════════════════════

/// POST /api/reviews/scorecards
pub async fn create_scorecard(
    pool: web::Data<PgPool>,
    body: web::Json<CreateScorecardRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;
    apply_rate_limit(pool.get_ref(), auth.user_id).await?;

    let sc = review_db::create_scorecard(
        pool.get_ref(), &body.name, body.description.as_deref(),
        &body.target_type, body.passing_score, Some(auth.user_id),
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let dims = if let Some(ref d) = body.dimensions {
        review_db::set_dimensions(pool.get_ref(), sc.id, d)
            .await.map_err(|e| AppError::DatabaseError(e.to_string()))?
    } else { Vec::new() };

    let mut rules = Vec::new();
    if let Some(ref cr) = body.consistency_rules {
        for r in cr {
            let rule = review_db::create_consistency_rule(pool.get_ref(), sc.id, r)
                .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
            rules.push(rule);
        }
    }

    audit_action(pool.get_ref(), &auth, "scorecard_created", Some("scorecard"),
        Some(&sc.id.to_string()), Some(serde_json::json!({"name": &body.name})),
        get_ip(&req).as_deref(), get_user_agent(&req).as_deref()).await?;

    Ok(HttpResponse::Created().json(ScorecardWithDimensions {
        scorecard: sc, dimensions: dims, consistency_rules: rules,
    }))
}

/// GET /api/reviews/scorecards
pub async fn list_scorecards(
    pool: web::Data<PgPool>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    apply_rate_limit(pool.get_ref(), auth.user_id).await?;
    let cards = review_db::list_scorecards(pool.get_ref(), None)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(cards))
}

/// GET /api/reviews/scorecards/{id}
pub async fn get_scorecard(
    pool: web::Data<PgPool>, path: web::Path<Uuid>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    let id = path.into_inner();
    let sc = review_db::get_scorecard(pool.get_ref(), id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Scorecard not found".into()))?;
    let dims = review_db::get_dimensions(pool.get_ref(), id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    let rules = review_db::get_consistency_rules(pool.get_ref(), id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(ScorecardWithDimensions {
        scorecard: sc, dimensions: dims, consistency_rules: rules,
    }))
}

/// PUT /api/reviews/scorecards/{id}/dimensions
pub async fn set_dimensions(
    pool: web::Data<PgPool>, path: web::Path<Uuid>,
    body: web::Json<Vec<CreateDimensionInput>>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;
    let id = path.into_inner();
    let dims = review_db::set_dimensions(pool.get_ref(), id, &body)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(dims))
}

// ═══════════════════════════════════════════════════════════
// ASSIGNMENTS
// ═══════════════════════════════════════════════════════════

/// POST /api/reviews/assignments
pub async fn create_assignment(
    pool: web::Data<PgPool>,
    body: web::Json<CreateAssignmentRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;
    apply_rate_limit(pool.get_ref(), auth.user_id).await?;

    // Resolve submitter for COI checks
    let submitter_id = resolve_submitter(pool.get_ref(), &body.target_type, body.target_id).await?;

    let is_blind = body.is_blind.unwrap_or(false);

    let assign = match body.reviewer_id {
        Some(reviewer_id) => {
            assignment::manual_assign(
                pool.get_ref(), reviewer_id, &body.target_type, body.target_id,
                body.scorecard_id, submitter_id, is_blind, Some(auth.user_id), body.due_date,
            ).await?
        }
        None => {
            assignment::auto_assign(
                pool.get_ref(), &body.target_type, body.target_id,
                body.scorecard_id, submitter_id, is_blind, Some(auth.user_id), body.due_date,
            ).await?
        }
    };

    audit_action(pool.get_ref(), &auth, "review_assigned", Some("review_assignment"),
        Some(&assign.id.to_string()),
        Some(serde_json::json!({
            "reviewer_id": assign.reviewer_id, "method": assign.method,
            "is_blind": assign.is_blind,
        })),
        get_ip(&req).as_deref(), get_user_agent(&req).as_deref()).await?;

    Ok(HttpResponse::Created().json(assign))
}

/// GET /api/reviews/queue  (reviewer's assignment queue)
pub async fn review_queue(
    pool: web::Data<PgPool>,
    query: web::Query<ReviewQueueQuery>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::Reviewer, UserRole::OperationsAdmin])?;
    apply_rate_limit(pool.get_ref(), auth.user_id).await?;

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).min(100);
    let offset = (page - 1) * page_size;

    let assignments = review_db::list_assignments_for_reviewer(
        pool.get_ref(), auth.user_id, query.status.as_ref(), page_size, offset,
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let total = review_db::count_assignments_for_reviewer(
        pool.get_ref(), auth.user_id, query.status.as_ref(),
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let mut details = Vec::new();
    for a in assignments {
        let detail = build_assignment_detail(pool.get_ref(), &a).await?;
        details.push(detail);
    }

    Ok(HttpResponse::Ok().json(ReviewQueueResponse {
        assignments: details, total, page, page_size,
    }))
}

/// GET /api/reviews/assignments/{id}
pub async fn get_assignment(
    pool: web::Data<PgPool>, path: web::Path<Uuid>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    let id = path.into_inner();
    let a = review_db::get_assignment(pool.get_ref(), id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Assignment not found".into()))?;

    if a.reviewer_id != auth.user_id {
        require_role(&auth, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;
    }

    let detail = build_assignment_detail(pool.get_ref(), &a).await?;
    Ok(HttpResponse::Ok().json(detail))
}

/// POST /api/reviews/assignments/{id}/recuse
pub async fn recuse(
    pool: web::Data<PgPool>, path: web::Path<Uuid>,
    body: web::Json<RecusalRequest>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    let id = path.into_inner();
    let a = review_db::get_assignment(pool.get_ref(), id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Assignment not found".into()))?;

    if a.reviewer_id != auth.user_id {
        return Err(AppError::Forbidden);
    }

    let recused = review_db::recuse_assignment(pool.get_ref(), id, &body.reason)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Auto-reassign
    let submitter_id = resolve_submitter(pool.get_ref(), &a.target_type, a.target_id).await?;
    let new_assignment = assignment::reassign_after_recusal(pool.get_ref(), &recused, submitter_id).await;

    audit_action(pool.get_ref(), &auth, "review_recused", Some("review_assignment"),
        Some(&id.to_string()),
        Some(serde_json::json!({"reason": &body.reason, "reassigned": new_assignment.is_ok()})),
        get_ip(&req).as_deref(), get_user_agent(&req).as_deref()).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "recused": recused,
        "reassigned_to": new_assignment.ok().map(|a| a.reviewer_id),
    })))
}

// ═══════════════════════════════════════════════════════════
// REVIEWS
// ═══════════════════════════════════════════════════════════

/// POST /api/reviews/assignments/{id}/submit
pub async fn submit_review(
    pool: web::Data<PgPool>, path: web::Path<Uuid>,
    body: web::Json<SubmitReviewRequest>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::Reviewer, UserRole::OperationsAdmin])?;
    apply_rate_limit(pool.get_ref(), auth.user_id).await?;

    let assignment_id = path.into_inner();
    let a = review_db::get_assignment(pool.get_ref(), assignment_id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Assignment not found".into()))?;

    if a.reviewer_id != auth.user_id {
        return Err(AppError::Forbidden);
    }

    if a.status != ReviewAssignmentStatus::Pending && a.status != ReviewAssignmentStatus::InProgress {
        return Err(AppError::BadRequest("Assignment is not in a reviewable state".into()));
    }

    // Get scorecard dimensions and rules
    let dims = review_db::get_dimensions(pool.get_ref(), a.scorecard_id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    let rules = review_db::get_consistency_rules(pool.get_ref(), a.scorecard_id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Validate scores
    consistency::validate_review_scores(&dims, &body.scores)?;

    // Run consistency checks
    let cc_output = consistency::check_consistency(&dims, &rules, &body.scores);

    if cc_output.has_errors {
        return Ok(HttpResponse::UnprocessableEntity().json(serde_json::json!({
            "valid": false,
            "consistency": cc_output,
            "message": "Review has consistency errors that must be resolved",
        })));
    }

    if cc_output.has_warnings && !body.acknowledge_warnings.unwrap_or(false) {
        return Ok(HttpResponse::Conflict().json(serde_json::json!({
            "valid": false,
            "consistency": cc_output,
            "message": "Review has consistency warnings. Set acknowledge_warnings=true to proceed.",
        })));
    }

    // Compute weighted score
    let overall_score = consistency::compute_weighted_score(&dims, &body.scores);

    // Create or update review
    let review = match review_db::get_review_by_assignment(pool.get_ref(), assignment_id).await
        .map_err(|e| AppError::DatabaseError(e.to_string()))? {
        Some(existing) => existing,
        None => review_db::create_review(
            pool.get_ref(), assignment_id, auth.user_id,
            a.scorecard_id, &a.target_type, a.target_id,
        ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?,
    };

    // Save scores
    for s in &body.scores {
        review_db::upsert_score(pool.get_ref(), review.id, s.dimension_id, s.rating, s.comment.as_deref())
            .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    // Persist consistency results
    consistency::persist_consistency_results(pool.get_ref(), review.id, &rules, &cc_output).await?;

    // Submit review
    let submitted = review_db::submit_review(
        pool.get_ref(), review.id, overall_score,
        body.overall_comment.as_deref(), &body.recommendation,
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Update assignment status
    review_db::update_assignment_status(pool.get_ref(), assignment_id, &ReviewAssignmentStatus::Completed)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let scores = review_db::get_scores(pool.get_ref(), review.id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    let cc_results = review_db::get_consistency_results(pool.get_ref(), review.id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    audit_action(pool.get_ref(), &auth, "review_submitted", Some("review"),
        Some(&review.id.to_string()),
        Some(serde_json::json!({
            "overall_score": overall_score, "recommendation": &body.recommendation,
            "consistency_warnings": cc_output.results.len(),
        })),
        get_ip(&req).as_deref(), get_user_agent(&req).as_deref()).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "valid": true,
        "consistency": cc_output,
        "review": ReviewDetail { review: submitted, scores, consistency_results: cc_results },
    })))
}

/// GET /api/reviews/{id}
pub async fn get_review(
    pool: web::Data<PgPool>, path: web::Path<Uuid>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    let id = path.into_inner();
    let review = review_db::get_review(pool.get_ref(), id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Review not found".into()))?;
    if review.reviewer_id != auth.user_id {
        require_role(&auth, &[UserRole::OperationsAdmin, UserRole::DepartmentManager])?;
    }
    let scores = review_db::get_scores(pool.get_ref(), id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    let cc = review_db::get_consistency_results(pool.get_ref(), id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(ReviewDetail { review, scores, consistency_results: cc }))
}

// ═══════════════════════════════════════════════════════════
// CONFLICT OF INTEREST
// ═══════════════════════════════════════════════════════════

/// POST /api/reviews/coi
pub async fn declare_coi(
    pool: web::Data<PgPool>, body: web::Json<DeclareCoiRequest>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    let coi = review_db::declare_coi(
        pool.get_ref(), auth.user_id, &body.conflict_type,
        body.target_user_id, body.department.as_deref(),
        body.description.as_deref(), Some(auth.user_id),
    ).await.map_err(|e| map_sqlx_unique_violation(e, "Conflict of interest already declared"))?;

    audit_action(pool.get_ref(), &auth, "coi_declared", Some("conflict_of_interest"),
        Some(&coi.id.to_string()), Some(serde_json::json!({"type": &body.conflict_type})),
        get_ip(&req).as_deref(), get_user_agent(&req).as_deref()).await?;

    Ok(HttpResponse::Created().json(coi))
}

/// GET /api/reviews/coi
pub async fn list_coi(
    pool: web::Data<PgPool>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    let conflicts = review_db::get_coi_for_reviewer(pool.get_ref(), auth.user_id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(conflicts))
}

/// DELETE /api/reviews/coi/{id}
pub async fn revoke_coi(
    pool: web::Data<PgPool>, path: web::Path<Uuid>, req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let auth = authenticate_request(pool.get_ref(), &req).await?;
    require_role(&auth, &[UserRole::OperationsAdmin])?;
    let coi_id = path.into_inner();
    review_db::revoke_coi(pool.get_ref(), coi_id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    audit_action(pool.get_ref(), &auth, "coi_revoked", Some("conflict_of_interest"),
        Some(&coi_id.to_string()), None,
        get_ip(&req).as_deref(), get_user_agent(&req).as_deref()).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "COI revoked"})))
}

// ═══════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════

/// Resolve the submitter user ID from a review target
async fn resolve_submitter(
    pool: &PgPool, target_type: &ReviewTargetType, target_id: Uuid,
) -> Result<Uuid, AppError> {
    match target_type {
        ReviewTargetType::InspectionSubmission => {
            let sub = crate::db::inspection::get_submission(pool, target_id)
                .await.map_err(|e| AppError::DatabaseError(e.to_string()))?
                .ok_or(AppError::NotFound("Submission not found".into()))?;
            Ok(sub.submitted_by)
        }
        ReviewTargetType::DisputedClassification => {
            let dispute = crate::db::dispute::get_dispute(pool, target_id)
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?
                .ok_or(AppError::NotFound("Classification dispute not found".into()))?;
            Ok(dispute.disputed_by)
        }
    }
}

/// Build enriched assignment detail with scorecard, dimensions, target summary
async fn build_assignment_detail(
    pool: &PgPool, a: &ReviewAssignment,
) -> Result<AssignmentDetail, AppError> {
    let sc = review_db::get_scorecard(pool, a.scorecard_id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or(AppError::NotFound("Scorecard not found".into()))?;
    let dims = review_db::get_dimensions(pool, a.scorecard_id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    let rules = review_db::get_consistency_rules(pool, a.scorecard_id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
    let existing_review = review_db::get_review_by_assignment(pool, a.id)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let target_summary = build_target_summary(pool, &a.target_type, a.target_id, a.is_blind).await?;

    Ok(AssignmentDetail {
        assignment: a.clone(), scorecard: sc, dimensions: dims,
        consistency_rules: rules, target_summary, existing_review,
    })
}

/// Build target summary, anonymizing submitter info for blind reviews
async fn build_target_summary(
    pool: &PgPool, target_type: &ReviewTargetType, target_id: Uuid, is_blind: bool,
) -> Result<TargetSummary, AppError> {
    match target_type {
        ReviewTargetType::InspectionSubmission => {
            let sub = crate::db::inspection::get_submission(pool, target_id)
                .await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

            let (title, submitted_at, submitter_name, details) = match sub {
                Some(ref s) => {
                    let submitter = if is_blind {
                        None
                    } else {
                        let u = crate::db::users::find_by_id(pool, s.submitted_by).await?;
                        u.map(|u| u.username)
                    };
                    (
                        format!("Inspection Submission"),
                        Some(s.submitted_at.to_rfc3339()),
                        submitter,
                        serde_json::json!({
                            "status": s.status,
                            "notes": s.notes,
                        }),
                    )
                }
                None => ("Unknown".into(), None, None, serde_json::json!({})),
            };

            Ok(TargetSummary {
                target_type: target_type.clone(), target_id, title,
                submitted_at, submitter_name, details,
            })
        }
        ReviewTargetType::DisputedClassification => {
            let dispute = crate::db::dispute::get_dispute(pool, target_id)
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;
            let (title, submitted_at, submitter_name, details) = match dispute {
                Some(ref d) => {
                    let submitter = if is_blind {
                        None
                    } else {
                        let u = crate::db::users::find_by_id(pool, d.disputed_by).await?;
                        u.map(|u| u.username)
                    };
                    (
                        "Disputed Classification".to_string(),
                        Some(d.created_at.to_rfc3339()),
                        submitter,
                        serde_json::json!({
                            "status": d.status,
                            "reason": d.reason,
                            "proposed_category": d.proposed_category,
                        }),
                    )
                }
                None => ("Unknown Dispute".into(), None, None, serde_json::json!({})),
            };
            Ok(TargetSummary {
                target_type: target_type.clone(), target_id, title,
                submitted_at, submitter_name, details,
            })
        }
    }
}

// ═══════════════════════════════════════════════════════════
// ROUTE CONFIG
// ═══════════════════════════════════════════════════════════

pub fn review_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/reviews")
            // Scorecards
            .route("/scorecards", web::post().to(create_scorecard))
            .route("/scorecards", web::get().to(list_scorecards))
            .route("/scorecards/{id}", web::get().to(get_scorecard))
            .route("/scorecards/{id}/dimensions", web::put().to(set_dimensions))
            // Assignments
            .route("/assignments", web::post().to(create_assignment))
            .route("/assignments/{id}", web::get().to(get_assignment))
            .route("/assignments/{id}/recuse", web::post().to(recuse))
            .route("/assignments/{id}/submit", web::post().to(submit_review))
            // Queue
            .route("/queue", web::get().to(review_queue))
            // COI — must come before /{id} to prevent "coi" being parsed as UUID
            .route("/coi", web::post().to(declare_coi))
            .route("/coi", web::get().to(list_coi))
            .route("/coi/{id}", web::delete().to(revoke_coi))
            // Reviews (catch-all /{id} last)
            .route("/{id}", web::get().to(get_review)),
    );
}
