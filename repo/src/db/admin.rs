use chrono::{Duration, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    Campaign, CampaignStatus, DashboardKpis, InstanceStatusCount, ItemOverview, KpiMetric,
    KpiSnapshot, RegionCount, ReportConfig, ReportFormat, RoleCount, StatusCount, Tag,
    UserOverview, WorkOrderOverview,
};

// ── Campaigns ───────────────────────────────────────────────

pub async fn create_campaign(
    pool: &PgPool, name: &str, description: Option<&str>, start_date: NaiveDate,
    end_date: NaiveDate, target_region: Option<&str>, target_audience: Option<&str>,
    goals: Option<&serde_json::Value>, created_by: Option<Uuid>,
) -> Result<Campaign, sqlx::Error> {
    sqlx::query_as::<_, Campaign>(
        r#"INSERT INTO campaigns (name, description, start_date, end_date, target_region, target_audience, goals, created_by)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING *"#,
    ).bind(name).bind(description).bind(start_date).bind(end_date)
    .bind(target_region).bind(target_audience).bind(goals).bind(created_by)
    .fetch_one(pool).await
}

pub async fn get_campaign(pool: &PgPool, id: Uuid) -> Result<Option<Campaign>, sqlx::Error> {
    sqlx::query_as::<_, Campaign>("SELECT * FROM campaigns WHERE id=$1")
        .bind(id).fetch_optional(pool).await
}

pub async fn list_campaigns(
    pool: &PgPool, status: Option<&CampaignStatus>, limit: i64, offset: i64,
) -> Result<Vec<Campaign>, sqlx::Error> {
    match status {
        Some(s) => sqlx::query_as::<_, Campaign>(
            "SELECT * FROM campaigns WHERE status=$1 ORDER BY start_date DESC LIMIT $2 OFFSET $3",
        ).bind(s).bind(limit).bind(offset).fetch_all(pool).await,
        None => sqlx::query_as::<_, Campaign>(
            "SELECT * FROM campaigns ORDER BY start_date DESC LIMIT $1 OFFSET $2",
        ).bind(limit).bind(offset).fetch_all(pool).await,
    }
}

pub async fn update_campaign(
    pool: &PgPool, id: Uuid, name: &str, description: Option<&str>,
    status: &CampaignStatus, start_date: NaiveDate, end_date: NaiveDate,
    target_region: Option<&str>, target_audience: Option<&str>,
    goals: Option<&serde_json::Value>,
) -> Result<Campaign, sqlx::Error> {
    sqlx::query_as::<_, Campaign>(
        r#"UPDATE campaigns SET name=$2, description=$3, status=$4, start_date=$5, end_date=$6,
        target_region=$7, target_audience=$8, goals=$9, updated_at=NOW()
        WHERE id=$1 RETURNING *"#,
    ).bind(id).bind(name).bind(description).bind(status).bind(start_date)
    .bind(end_date).bind(target_region).bind(target_audience).bind(goals)
    .fetch_one(pool).await
}

pub async fn set_campaign_tags(pool: &PgPool, campaign_id: Uuid, tag_ids: &[Uuid]) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM campaign_tags WHERE campaign_id=$1")
        .bind(campaign_id).execute(pool).await?;
    for tid in tag_ids {
        sqlx::query("INSERT INTO campaign_tags (campaign_id, tag_id) VALUES ($1,$2) ON CONFLICT DO NOTHING")
            .bind(campaign_id).bind(tid).execute(pool).await?;
    }
    Ok(())
}

pub async fn get_campaign_tags(pool: &PgPool, campaign_id: Uuid) -> Result<Vec<Tag>, sqlx::Error> {
    sqlx::query_as::<_, Tag>(
        "SELECT t.* FROM tags t JOIN campaign_tags ct ON ct.tag_id=t.id WHERE ct.campaign_id=$1 ORDER BY t.name",
    ).bind(campaign_id).fetch_all(pool).await
}

// ── Tags ────────────────────────────────────────────────────

pub async fn create_tag(pool: &PgPool, name: &str, color: Option<&str>) -> Result<Tag, sqlx::Error> {
    sqlx::query_as::<_, Tag>(
        "INSERT INTO tags (name, color) VALUES ($1,$2) RETURNING *",
    ).bind(name).bind(color).fetch_one(pool).await
}

pub async fn list_tags(pool: &PgPool) -> Result<Vec<Tag>, sqlx::Error> {
    sqlx::query_as::<_, Tag>("SELECT * FROM tags ORDER BY name").fetch_all(pool).await
}

pub async fn delete_tag(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM tags WHERE id=$1").bind(id).execute(pool).await?;
    Ok(r.rows_affected() > 0)
}

pub async fn set_category_tags(pool: &PgPool, category_id: Uuid, tag_ids: &[Uuid]) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM category_tags WHERE category_id=$1")
        .bind(category_id).execute(pool).await?;
    for tid in tag_ids {
        sqlx::query("INSERT INTO category_tags (category_id, tag_id) VALUES ($1,$2) ON CONFLICT DO NOTHING")
            .bind(category_id).bind(tid).execute(pool).await?;
    }
    Ok(())
}

// ── KPI Queries ─────────────────────────────────────────────

/// Correct sorting guidance conversion rate:
/// ratio of approved reviews to total submitted reviews
pub async fn kpi_sorting_conversion_rate(
    pool: &PgPool, from: NaiveDate, to: NaiveDate,
) -> Result<f64, sqlx::Error> {
    let row: (i64, i64) = sqlx::query_as(
        r#"SELECT
            COALESCE(SUM(CASE WHEN r.recommendation='approve' THEN 1 ELSE 0 END), 0),
            COALESCE(COUNT(*), 0)
        FROM reviews r
        WHERE r.status IN ('submitted','finalized')
          AND r.submitted_at IS NOT NULL
          AND r.submitted_at::date BETWEEN $1 AND $2"#,
    ).bind(from).bind(to).fetch_one(pool).await?;
    Ok(if row.1 > 0 { row.0 as f64 / row.1 as f64 * 100.0 } else { 0.0 })
}

/// Template reuse rate: average number of schedules per template
pub async fn kpi_template_reuse_rate(pool: &PgPool) -> Result<f64, sqlx::Error> {
    let row: (i64, i64) = sqlx::query_as(
        r#"SELECT
            (SELECT COUNT(*) FROM task_schedules WHERE is_active=TRUE),
            (SELECT COUNT(*) FROM task_templates WHERE is_active=TRUE)"#,
    ).fetch_one(pool).await?;
    Ok(if row.1 > 0 { row.0 as f64 / row.1 as f64 } else { 0.0 })
}

/// Retention rate: users who logged in during the window / total active users
pub async fn kpi_retention(pool: &PgPool, days: i64) -> Result<f64, sqlx::Error> {
    let cutoff = Utc::now() - Duration::days(days);
    let row: (i64, i64) = sqlx::query_as(
        r#"SELECT
            (SELECT COUNT(DISTINCT user_id) FROM sessions WHERE created_at >= $1 AND is_valid=TRUE),
            (SELECT COUNT(*) FROM users WHERE status='active'::account_status)"#,
    ).bind(cutoff).fetch_one(pool).await?;
    Ok(if row.1 > 0 { row.0 as f64 / row.1 as f64 * 100.0 } else { 0.0 })
}

/// Build the full dashboard KPIs
pub async fn get_dashboard_kpis(pool: &PgPool, from: NaiveDate, to: NaiveDate) -> Result<DashboardKpis, sqlx::Error> {
    let prev_duration = to - from;
    let prev_from = from - prev_duration;
    let prev_to = from;

    let current_conversion = kpi_sorting_conversion_rate(pool, from, to).await?;
    let prev_conversion = kpi_sorting_conversion_rate(pool, prev_from, prev_to).await?;

    let current_reuse = kpi_template_reuse_rate(pool).await?;
    let ret_30 = kpi_retention(pool, 30).await?;
    let ret_60 = kpi_retention(pool, 60).await?;
    let ret_90 = kpi_retention(pool, 90).await?;

    let active_users: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE status='active'::account_status",
    ).fetch_one(pool).await?;

    let total_completed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM task_instances WHERE status='completed'::task_instance_status",
    ).fetch_one(pool).await?;

    let total_reviews: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM reviews WHERE status IN ('submitted','finalized')",
    ).fetch_one(pool).await?;

    let total_kb: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kb_entries WHERE is_active=TRUE",
    ).fetch_one(pool).await?;

    let active_campaigns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM campaigns WHERE status='active'::campaign_status",
    ).fetch_one(pool).await?;

    let overdue: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM task_instances WHERE status='overdue'::task_instance_status",
    ).fetch_one(pool).await?;

    let pending_reviews: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM review_assignments WHERE status IN ('pending'::review_assignment_status,'in_progress'::review_assignment_status)",
    ).fetch_one(pool).await?;

    fn trend(current: f64, previous: f64) -> f64 {
        if previous == 0.0 { 0.0 } else { ((current - previous) / previous) * 100.0 }
    }

    Ok(DashboardKpis {
        sorting_conversion_rate: KpiMetric {
            current: current_conversion, previous: prev_conversion,
            trend: trend(current_conversion, prev_conversion),
            label: "Correct Sorting Guidance Rate (%)".into(),
        },
        template_reuse_rate: KpiMetric {
            current: current_reuse, previous: 0.0, trend: 0.0,
            label: "Template Reuse Rate".into(),
        },
        retention_30d: KpiMetric {
            current: ret_30, previous: 0.0, trend: 0.0, label: "30-Day Retention (%)".into(),
        },
        retention_60d: KpiMetric {
            current: ret_60, previous: 0.0, trend: 0.0, label: "60-Day Retention (%)".into(),
        },
        retention_90d: KpiMetric {
            current: ret_90, previous: 0.0, trend: 0.0, label: "90-Day Retention (%)".into(),
        },
        active_users,
        total_tasks_completed: total_completed,
        total_reviews_completed: total_reviews,
        total_kb_entries: total_kb,
        active_campaigns,
        overdue_tasks: overdue,
        pending_reviews,
    })
}

// ── Overviews ───────────────────────────────────────────────

pub async fn get_user_overview(pool: &PgPool) -> Result<UserOverview, sqlx::Error> {
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(pool).await?;
    let by_role = sqlx::query_as::<_, RoleCount>(
        "SELECT role::text AS role, COUNT(*) AS count FROM users GROUP BY role ORDER BY count DESC",
    ).fetch_all(pool).await?;
    let by_status = sqlx::query_as::<_, StatusCount>(
        "SELECT status::text AS status, COUNT(*) AS count FROM users GROUP BY status ORDER BY count DESC",
    ).fetch_all(pool).await?;
    let recent: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT user_id) FROM sessions WHERE created_at >= NOW() - INTERVAL '7 days'",
    ).fetch_one(pool).await?;
    Ok(UserOverview { total_users: total, by_role, by_status, recent_logins: recent })
}

pub async fn get_item_overview(pool: &PgPool) -> Result<ItemOverview, sqlx::Error> {
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM kb_entries").fetch_one(pool).await?;
    let active: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kb_entries WHERE is_active=TRUE",
    ).fetch_one(pool).await?;
    let cats: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM kb_categories").fetch_one(pool).await?;
    let by_region = sqlx::query_as::<_, RegionCount>(
        "SELECT region, COUNT(*) AS count FROM kb_entries WHERE is_active=TRUE GROUP BY region ORDER BY count DESC",
    ).fetch_all(pool).await?;
    let recent: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kb_entries WHERE updated_at >= NOW() - INTERVAL '7 days'",
    ).fetch_one(pool).await?;
    Ok(ItemOverview { total_kb_entries: total, active_entries: active, total_categories: cats, entries_by_region: by_region, recent_updates: recent })
}

pub async fn get_work_order_overview(pool: &PgPool) -> Result<WorkOrderOverview, sqlx::Error> {
    let templates: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM task_templates WHERE is_active=TRUE",
    ).fetch_one(pool).await?;
    let schedules: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM task_schedules WHERE is_active=TRUE",
    ).fetch_one(pool).await?;
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM task_instances").fetch_one(pool).await?;
    let completed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM task_instances WHERE status='completed'::task_instance_status",
    ).fetch_one(pool).await?;
    let by_status = sqlx::query_as::<_, InstanceStatusCount>(
        "SELECT status::text AS status, COUNT(*) AS count FROM task_instances GROUP BY status ORDER BY count DESC",
    ).fetch_all(pool).await?;

    let avg_hours: f64 = sqlx::query_scalar::<_, Option<f64>>(
        r#"SELECT AVG(EXTRACT(EPOCH FROM (completed_at - started_at)) / 3600.0)
        FROM task_instances WHERE completed_at IS NOT NULL AND started_at IS NOT NULL"#,
    ).fetch_one(pool).await?.unwrap_or(0.0);

    let rate = if total > 0 { completed as f64 / total as f64 * 100.0 } else { 0.0 };

    Ok(WorkOrderOverview {
        total_templates: templates, active_schedules: schedules, total_instances: total,
        by_status, completion_rate: rate, avg_completion_time_hours: avg_hours,
    })
}

// ── KPI Snapshots ───────────────────────────────────────────

pub async fn save_kpi_snapshot(
    pool: &PgPool, metric_name: &str, value: f32, dimensions: Option<&serde_json::Value>,
    period_start: NaiveDate, period_end: NaiveDate,
) -> Result<KpiSnapshot, sqlx::Error> {
    sqlx::query_as::<_, KpiSnapshot>(
        "INSERT INTO kpi_snapshots (metric_name, metric_value, dimensions, period_start, period_end) VALUES ($1,$2,$3,$4,$5) RETURNING *",
    ).bind(metric_name).bind(value).bind(dimensions).bind(period_start).bind(period_end)
    .fetch_one(pool).await
}

pub async fn get_kpi_trend(
    pool: &PgPool, metric_name: &str, limit: i64,
) -> Result<Vec<KpiSnapshot>, sqlx::Error> {
    sqlx::query_as::<_, KpiSnapshot>(
        "SELECT * FROM kpi_snapshots WHERE metric_name=$1 ORDER BY period_end DESC LIMIT $2",
    ).bind(metric_name).bind(limit).fetch_all(pool).await
}

// ── Report Configs ──────────────────────────────────────────

pub async fn create_report_config(
    pool: &PgPool, name: &str, report_type: &str, parameters: &serde_json::Value,
    format: &ReportFormat, created_by: Option<Uuid>,
) -> Result<ReportConfig, sqlx::Error> {
    sqlx::query_as::<_, ReportConfig>(
        "INSERT INTO report_configs (name, report_type, parameters, format, created_by) VALUES ($1,$2,$3,$4,$5) RETURNING *",
    ).bind(name).bind(report_type).bind(parameters).bind(format).bind(created_by)
    .fetch_one(pool).await
}

pub async fn list_report_configs(pool: &PgPool) -> Result<Vec<ReportConfig>, sqlx::Error> {
    sqlx::query_as::<_, ReportConfig>("SELECT * FROM report_configs ORDER BY name")
        .fetch_all(pool).await
}
