use sqlx::PgPool;
use uuid::Uuid;

use crate::db::inspection as insp_db;
use crate::errors::AppError;
use crate::models::{
    SubtaskResponseInput, TemplateSubtask, ValidationItem, ValidationResult,
};

/// Validate a submission's subtask responses against the template's requirements.
/// Returns immediate validation feedback.
pub fn validate_submission(
    subtasks: &[TemplateSubtask],
    responses: &[SubtaskResponseInput],
) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Check all required subtasks have responses
    for subtask in subtasks {
        let response = responses.iter().find(|r| r.subtask_id == subtask.id);

        match response {
            None if subtask.is_required => {
                errors.push(ValidationItem {
                    field: subtask.id.to_string(),
                    message: format!("Required field '{}' is missing", subtask.title),
                });
            }
            None => {
                // Optional, skip
            }
            Some(resp) => {
                // Type-specific validation
                validate_response_type(subtask, &resp.response_value, &mut errors, &mut warnings);
            }
        }
    }

    // Check for responses to unknown subtasks
    for resp in responses {
        if !subtasks.iter().any(|s| s.id == resp.subtask_id) {
            warnings.push(ValidationItem {
                field: resp.subtask_id.to_string(),
                message: "Response for unknown subtask (will be ignored)".to_string(),
            });
        }
    }

    ValidationResult {
        is_valid: errors.is_empty(),
        errors,
        warnings,
    }
}

/// Validate a single response value against the expected type.
fn validate_response_type(
    subtask: &TemplateSubtask,
    value: &serde_json::Value,
    errors: &mut Vec<ValidationItem>,
    warnings: &mut Vec<ValidationItem>,
) {
    match subtask.expected_type.as_str() {
        "checkbox" => {
            if !value.get("checked").and_then(|v| v.as_bool()).is_some() {
                errors.push(ValidationItem {
                    field: subtask.id.to_string(),
                    message: format!("'{}': Expected checkbox response with 'checked' boolean", subtask.title),
                });
            }
        }
        "text" => {
            match value.get("text").and_then(|v| v.as_str()) {
                Some(text) if text.trim().is_empty() && subtask.is_required => {
                    errors.push(ValidationItem {
                        field: subtask.id.to_string(),
                        message: format!("'{}': Text response cannot be empty", subtask.title),
                    });
                }
                None if subtask.is_required => {
                    errors.push(ValidationItem {
                        field: subtask.id.to_string(),
                        message: format!("'{}': Expected text response with 'text' field", subtask.title),
                    });
                }
                _ => {}
            }
        }
        "number" => {
            let num = value.get("number").and_then(|v| v.as_f64());
            if num.is_none() && subtask.is_required {
                errors.push(ValidationItem {
                    field: subtask.id.to_string(),
                    message: format!("'{}': Expected numeric response with 'number' field", subtask.title),
                });
            }

            // Check min/max from options if present
            if let (Some(num), Some(opts)) = (num, &subtask.options) {
                if let Some(min) = opts.get("min").and_then(|v| v.as_f64()) {
                    if num < min {
                        errors.push(ValidationItem {
                            field: subtask.id.to_string(),
                            message: format!("'{}': Value {} is below minimum {}", subtask.title, num, min),
                        });
                    }
                }
                if let Some(max) = opts.get("max").and_then(|v| v.as_f64()) {
                    if num > max {
                        errors.push(ValidationItem {
                            field: subtask.id.to_string(),
                            message: format!("'{}': Value {} exceeds maximum {}", subtask.title, num, max),
                        });
                    }
                }
            }
        }
        "photo" => {
            if !value.get("photo_id").and_then(|v| v.as_str()).is_some() {
                if subtask.is_required {
                    errors.push(ValidationItem {
                        field: subtask.id.to_string(),
                        message: format!("'{}': Photo is required", subtask.title),
                    });
                }
            }
        }
        "select" => {
            let selected = value.get("selected").and_then(|v| v.as_str());
            if selected.is_none() && subtask.is_required {
                errors.push(ValidationItem {
                    field: subtask.id.to_string(),
                    message: format!("'{}': Selection is required", subtask.title),
                });
            }

            // Validate against allowed options
            if let (Some(selected), Some(opts)) = (selected, &subtask.options) {
                if let Some(allowed) = opts.get("choices").and_then(|v| v.as_array()) {
                    let valid_choices: Vec<&str> = allowed
                        .iter()
                        .filter_map(|v| v.as_str())
                        .collect();
                    if !valid_choices.contains(&selected) {
                        errors.push(ValidationItem {
                            field: subtask.id.to_string(),
                            message: format!(
                                "'{}': '{}' is not a valid option. Choose from: {}",
                                subtask.title,
                                selected,
                                valid_choices.join(", ")
                            ),
                        });
                    }
                }
            }
        }
        _ => {
            warnings.push(ValidationItem {
                field: subtask.id.to_string(),
                message: format!("'{}': Unknown expected type '{}'", subtask.title, subtask.expected_type),
            });
        }
    }
}

/// Persist validation results to the database for the submission
pub async fn persist_validation(
    pool: &PgPool,
    submission_id: Uuid,
    result: &ValidationResult,
) -> Result<(), AppError> {
    for err in &result.errors {
        insp_db::create_validation(
            pool,
            submission_id,
            &err.field,
            false,
            Some(&err.message),
            "error",
        )
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    for warn in &result.warnings {
        insp_db::create_validation(
            pool,
            submission_id,
            &warn.field,
            true,
            Some(&warn.message),
            "warning",
        )
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_subtask(title: &str, expected_type: &str, required: bool, options: Option<serde_json::Value>) -> TemplateSubtask {
        TemplateSubtask {
            id: Uuid::new_v4(),
            template_id: Uuid::new_v4(),
            title: title.to_string(),
            description: None,
            sort_order: 0,
            is_required: required,
            expected_type: expected_type.to_string(),
            options,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_valid_checkbox_submission() {
        let sub = make_subtask("Check signage", "checkbox", true, None);
        let resp = vec![SubtaskResponseInput {
            subtask_id: sub.id,
            response_value: serde_json::json!({"checked": true}),
        }];
        let result = validate_submission(&[sub], &resp);
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_missing_required() {
        let sub = make_subtask("Check signage", "checkbox", true, None);
        let result = validate_submission(&[sub], &[]);
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_invalid_number_range() {
        let sub = make_subtask("Temperature", "number", true, Some(serde_json::json!({"min": 0, "max": 100})));
        let resp = vec![SubtaskResponseInput {
            subtask_id: sub.id,
            response_value: serde_json::json!({"number": 150}),
        }];
        let result = validate_submission(&[sub], &resp);
        assert!(!result.is_valid);
        assert!(result.errors[0].message.contains("exceeds maximum"));
    }

    #[test]
    fn test_invalid_select_option() {
        let sub = make_subtask("Condition", "select", true,
            Some(serde_json::json!({"choices": ["good", "fair", "poor"]})));
        let resp = vec![SubtaskResponseInput {
            subtask_id: sub.id,
            response_value: serde_json::json!({"selected": "excellent"}),
        }];
        let result = validate_submission(&[sub], &resp);
        assert!(!result.is_valid);
        assert!(result.errors[0].message.contains("not a valid option"));
    }

    #[test]
    fn test_optional_skip() {
        let sub = make_subtask("Notes", "text", false, None);
        let result = validate_submission(&[sub], &[]);
        assert!(result.is_valid);
    }

    #[test]
    fn test_empty_text_required() {
        let sub = make_subtask("Notes", "text", true, None);
        let resp = vec![SubtaskResponseInput {
            subtask_id: sub.id,
            response_value: serde_json::json!({"text": "  "}),
        }];
        let result = validate_submission(&[sub], &resp);
        assert!(!result.is_valid);
    }
}
