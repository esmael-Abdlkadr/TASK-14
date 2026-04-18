use actix_web::{HttpResponse, ResponseError};
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    // Auth errors
    InvalidCredentials,
    AccountLocked { minutes_remaining: i64 },
    PasswordTooShort,
    PasswordRequirementsNotMet(String),
    SessionExpired,
    SessionNotFound,
    Unauthorized,
    Forbidden,

    // Rate limiting
    RateLimitExceeded { retry_after_secs: u64 },
    BotDetected,

    // Step-up
    StepUpRequired(String),
    StepUpFailed,

    // Device binding
    UnknownDevice,
    DeviceBindingFailed,

    // Encryption
    EncryptionError(String),
    DecryptionError(String),
    KeyNotFound,

    // General
    DatabaseError(String),
    InternalError(String),
    BadRequest(String),
    NotFound(String),
    Conflict(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::InvalidCredentials => write!(f, "Invalid username or password"),
            AppError::AccountLocked { minutes_remaining } => {
                write!(f, "Account locked. Try again in {} minutes", minutes_remaining)
            }
            AppError::PasswordTooShort => write!(f, "Password must be at least 12 characters"),
            AppError::PasswordRequirementsNotMet(msg) => write!(f, "Password requirements not met: {}", msg),
            AppError::SessionExpired => write!(f, "Session has expired due to inactivity"),
            AppError::SessionNotFound => write!(f, "Session not found"),
            AppError::Unauthorized => write!(f, "Authentication required"),
            AppError::Forbidden => write!(f, "Insufficient permissions"),
            AppError::RateLimitExceeded { retry_after_secs } => {
                write!(f, "Rate limit exceeded. Retry after {} seconds", retry_after_secs)
            }
            AppError::BotDetected => write!(f, "Request blocked: suspicious activity detected"),
            AppError::StepUpRequired(action) => {
                write!(f, "Step-up verification required for: {}", action)
            }
            AppError::StepUpFailed => write!(f, "Step-up verification failed"),
            AppError::UnknownDevice => write!(f, "Unknown device. Please verify your identity"),
            AppError::DeviceBindingFailed => write!(f, "Failed to bind device"),
            AppError::EncryptionError(msg) => write!(f, "Encryption error: {}", msg),
            AppError::DecryptionError(msg) => write!(f, "Decryption error: {}", msg),
            AppError::KeyNotFound => write!(f, "Encryption key not found"),
            AppError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            AppError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            AppError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::Conflict(msg) => write!(f, "Conflict: {}", msg),
        }
    }
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::InvalidCredentials => {
                HttpResponse::Unauthorized().json(serde_json::json!({
                    "error": "invalid_credentials",
                    "message": self.to_string()
                }))
            }
            AppError::AccountLocked { .. } => {
                HttpResponse::Forbidden().json(serde_json::json!({
                    "error": "account_locked",
                    "message": self.to_string()
                }))
            }
            AppError::PasswordTooShort | AppError::PasswordRequirementsNotMet(_) => {
                HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "password_policy",
                    "message": self.to_string()
                }))
            }
            AppError::SessionExpired | AppError::SessionNotFound => {
                HttpResponse::Unauthorized().json(serde_json::json!({
                    "error": "session_invalid",
                    "message": self.to_string()
                }))
            }
            AppError::Unauthorized => {
                HttpResponse::Unauthorized().json(serde_json::json!({
                    "error": "unauthorized",
                    "message": self.to_string()
                }))
            }
            AppError::Forbidden => {
                HttpResponse::Forbidden().json(serde_json::json!({
                    "error": "forbidden",
                    "message": self.to_string()
                }))
            }
            AppError::RateLimitExceeded { retry_after_secs } => {
                HttpResponse::TooManyRequests()
                    .insert_header(("Retry-After", retry_after_secs.to_string()))
                    .json(serde_json::json!({
                        "error": "rate_limit_exceeded",
                        "message": self.to_string()
                    }))
            }
            AppError::BotDetected => {
                HttpResponse::TooManyRequests().json(serde_json::json!({
                    "error": "bot_detected",
                    "message": self.to_string()
                }))
            }
            AppError::StepUpRequired(action) => {
                HttpResponse::Forbidden().json(serde_json::json!({
                    "error": "stepup_required",
                    "action": action,
                    "message": self.to_string()
                }))
            }
            AppError::StepUpFailed => {
                HttpResponse::Unauthorized().json(serde_json::json!({
                    "error": "stepup_failed",
                    "message": self.to_string()
                }))
            }
            AppError::UnknownDevice => {
                HttpResponse::Forbidden().json(serde_json::json!({
                    "error": "unknown_device",
                    "message": self.to_string()
                }))
            }
            AppError::BadRequest(msg) => {
                HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "bad_request",
                    "message": msg
                }))
            }
            AppError::NotFound(msg) => {
                HttpResponse::NotFound().json(serde_json::json!({
                    "error": "not_found",
                    "message": msg
                }))
            }
            AppError::Conflict(msg) => {
                HttpResponse::Conflict().json(serde_json::json!({
                    "error": "conflict",
                    "message": msg
                }))
            }
            _ => {
                log::error!("Internal error: {}", self);
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "internal_error",
                    "message": "An internal error occurred"
                }))
            }
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        log::error!("Database error: {:?}", err);
        AppError::DatabaseError(err.to_string())
    }
}

/// Postgres `unique_violation` (23505) → HTTP 409; otherwise a database error for logging.
pub fn map_sqlx_unique_violation(err: sqlx::Error, message: &'static str) -> AppError {
    if let Some(db) = err.as_database_error() {
        if db.code().is_some_and(|c| c == "23505") {
            return AppError::Conflict(message.into());
        }
    }
    AppError::DatabaseError(err.to_string())
}

#[cfg(test)]
mod error_tests {
    use super::*;
    use actix_web::{http::StatusCode, ResponseError};

    #[test]
    fn display_covers_all_variants() {
        let cases: Vec<(AppError, &'static str)> = vec![
            (AppError::InvalidCredentials, "Invalid username"),
            (
                AppError::AccountLocked {
                    minutes_remaining: 5,
                },
                "Try again in 5 minutes",
            ),
            (AppError::PasswordTooShort, "12 characters"),
            (
                AppError::PasswordRequirementsNotMet("up".into()),
                "Password requirements",
            ),
            (AppError::SessionExpired, "expired"),
            (AppError::SessionNotFound, "Session not found"),
            (AppError::Unauthorized, "Authentication required"),
            (AppError::Forbidden, "Insufficient permissions"),
            (
                AppError::RateLimitExceeded {
                    retry_after_secs: 30,
                },
                "Retry after 30 seconds",
            ),
            (AppError::BotDetected, "suspicious activity"),
            (
                AppError::StepUpRequired("export_csv".into()),
                "Step-up verification required",
            ),
            (AppError::StepUpFailed, "Step-up verification failed"),
            (AppError::UnknownDevice, "Unknown device"),
            (AppError::DeviceBindingFailed, "Failed to bind device"),
            (AppError::EncryptionError("e".into()), "Encryption error"),
            (AppError::DecryptionError("d".into()), "Decryption error"),
            (AppError::KeyNotFound, "Encryption key not found"),
            (AppError::DatabaseError("db".into()), "Database error"),
            (AppError::InternalError("i".into()), "Internal error"),
            (AppError::BadRequest("b".into()), "Bad request"),
            (AppError::NotFound("n".into()), "Not found"),
            (AppError::Conflict("c".into()), "Conflict"),
        ];
        for (err, needle) in cases {
            let s = err.to_string();
            assert!(
                s.contains(needle),
                "display {:?} → {:?}, expected needle {:?}",
                err,
                s,
                needle
            );
        }
    }

    #[test]
    fn error_response_status_codes() {
        assert_eq!(
            AppError::InvalidCredentials.error_response().status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::AccountLocked {
                minutes_remaining: 1
            }
            .error_response()
            .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            AppError::PasswordTooShort.error_response().status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AppError::SessionExpired.error_response().status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::Unauthorized.error_response().status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::Forbidden.error_response().status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            AppError::RateLimitExceeded {
                retry_after_secs: 9
            }
            .error_response()
            .status(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            AppError::BotDetected.error_response().status(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            AppError::StepUpRequired("x".into())
                .error_response()
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            AppError::StepUpFailed.error_response().status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::UnknownDevice.error_response().status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            AppError::BadRequest("bad".into()).error_response().status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AppError::NotFound("nf".into()).error_response().status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            AppError::Conflict("x".into()).error_response().status(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            AppError::DatabaseError("db".into())
                .error_response()
                .status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn from_sqlx_error_maps_to_database_error() {
        let e: AppError = sqlx::Error::RowNotFound.into();
        match e {
            AppError::DatabaseError(msg) => {
                assert!(!msg.is_empty());
            }
            _ => panic!("expected DatabaseError"),
        }
    }
}
