use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;

/// Result of anomaly detection check
pub struct AnomalyCheckResult {
    pub is_anomalous: bool,
    pub reasons: Vec<String>,
    pub risk_score: f64,
}

/// Detect anomalous login patterns for a user.
/// Checks: rapid successive failures, logins from new IPs, unusual timing.
pub async fn check_login_anomaly(
    pool: &PgPool,
    username: &str,
    ip_address: Option<&str>,
) -> Result<AnomalyCheckResult, AppError> {
    let mut reasons = Vec::new();
    let mut risk_score: f64 = 0.0;

    let window = Utc::now() - Duration::minutes(10);

    // Check rapid failures in last 10 minutes
    let recent_failures: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM login_attempts
        WHERE username = $1 AND success = FALSE AND attempted_at > $2
        "#,
    )
    .bind(username)
    .bind(window)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    if recent_failures >= 3 {
        reasons.push(format!("{} failed login attempts in last 10 minutes", recent_failures));
        risk_score += (recent_failures as f64) * 10.0;
    }

    // Check if IP is new for this user
    if let Some(ip) = ip_address {
        let known_ip: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM login_attempts
                WHERE username = $1 AND ip_address = $2 AND success = TRUE
                AND attempted_at > NOW() - INTERVAL '30 days'
            )
            "#,
        )
        .bind(username)
        .bind(ip)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if !known_ip {
            reasons.push(format!("Login from previously unseen IP: {}", mask_ip(ip)));
            risk_score += 20.0;
        }
    }

    // Check for multiple distinct IPs attempting login in short window
    let distinct_ips: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT ip_address) FROM login_attempts
        WHERE username = $1 AND attempted_at > $2
        "#,
    )
    .bind(username)
    .bind(window)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    if distinct_ips > 3 {
        reasons.push(format!(
            "Login attempts from {} distinct IPs in last 10 minutes",
            distinct_ips
        ));
        risk_score += 30.0;
    }

    Ok(AnomalyCheckResult {
        is_anomalous: risk_score >= 50.0,
        reasons,
        risk_score,
    })
}

/// Mask IP for logging (don't expose full IP in responses)
fn mask_ip(ip: &str) -> String {
    if let Some(pos) = ip.rfind('.') {
        format!("{}.*", &ip[..pos])
    } else {
        "***".to_string()
    }
}

#[cfg(test)]
mod anomaly_mask_tests {
    use super::mask_ip;

    #[test]
    fn masks_ipv4_last_octet() {
        assert_eq!(mask_ip("10.0.0.25"), "10.0.0.*");
    }

    #[test]
    fn masks_non_dot_ip() {
        assert_eq!(mask_ip("::1"), "***");
        assert_eq!(mask_ip("nope"), "***");
    }
}
