use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "user_role", rename_all = "snake_case")]
pub enum UserRole {
    FieldInspector,
    Reviewer,
    OperationsAdmin,
    DepartmentManager,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "account_status", rename_all = "snake_case")]
pub enum AccountStatus {
    Active,
    Locked,
    Disabled,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: UserRole,
    pub status: AccountStatus,
    pub locked_until: Option<DateTime<Utc>>,
    pub failed_attempts: i32,
    pub last_failed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User response with sensitive fields masked
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub role: UserRole,
    pub status: AccountStatus,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        UserResponse {
            id: user.id,
            username: user.username,
            role: user.role,
            status: user.status,
            created_at: user.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: UserRole,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub device_fingerprint: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub session_token: String,
    pub user: UserResponse,
    pub device_trusted: bool,
    pub requires_device_binding: bool,
}

#[derive(Debug, Deserialize)]
pub struct StepUpRequest {
    pub password: String,
    pub action_type: String,
}
