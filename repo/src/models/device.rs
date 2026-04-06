use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct DeviceBinding {
    pub id: Uuid,
    pub user_id: Uuid,
    #[serde(skip_serializing)]
    pub device_fingerprint: String,
    pub device_name: Option<String>,
    pub bound_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub is_trusted: bool,
    #[serde(skip_serializing)]
    pub encrypted_fingerprint: Option<String>,
    #[serde(skip_serializing)]
    pub fingerprint_hash: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BindDeviceRequest {
    pub device_fingerprint: String,
    pub device_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TrustDeviceRequest {
    pub device_id: Uuid,
    pub password: String,
}
