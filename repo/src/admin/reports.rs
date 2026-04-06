use chrono::NaiveDate;
use printpdf::*;
use sqlx::PgPool;

use crate::db::admin as admin_db;
use crate::errors::AppError;
use crate::models::{DashboardKpis, ReportFormat};

/// Generate a KPI summary report in the requested format
pub async fn generate_kpi_report(
    pool: &PgPool,
    from: NaiveDate,
    to: NaiveDate,
    format: &ReportFormat,
) -> Result<(Vec<u8>, &'static str, String), AppError> {
    let kpis = admin_db::get_dashboard_kpis(pool, from, to)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    match format {
        ReportFormat::Csv => {
            let data = kpi_to_csv(&kpis, from, to)?;
            Ok((data, "text/csv", format!("kpi_report_{}_{}.csv", from, to)))
        }
        ReportFormat::Pdf => {
            let data = kpi_to_pdf(&kpis, from, to)?;
            Ok((data, "application/pdf", format!("kpi_report_{}_{}.pdf", from, to)))
        }
    }
}

/// Generate user overview report
pub async fn generate_user_report(
    pool: &PgPool,
    format: &ReportFormat,
) -> Result<(Vec<u8>, &'static str, String), AppError> {
    let overview = admin_db::get_user_overview(pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    match format {
        ReportFormat::Csv => {
            let mut w = csv::Writer::from_writer(Vec::new());
            w.write_record(&["Metric", "Value"])
                .map_err(|e| AppError::InternalError(e.to_string()))?;
            w.write_record(&["Total Users", &overview.total_users.to_string()])
                .map_err(|e| AppError::InternalError(e.to_string()))?;
            w.write_record(&["Recent Logins (7d)", &overview.recent_logins.to_string()])
                .map_err(|e| AppError::InternalError(e.to_string()))?;

            w.write_record(&["", ""]).map_err(|e| AppError::InternalError(e.to_string()))?;
            w.write_record(&["Role", "Count"]).map_err(|e| AppError::InternalError(e.to_string()))?;
            for r in &overview.by_role {
                w.write_record(&[&r.role, &r.count.to_string()])
                    .map_err(|e| AppError::InternalError(e.to_string()))?;
            }

            w.write_record(&["", ""]).map_err(|e| AppError::InternalError(e.to_string()))?;
            w.write_record(&["Status", "Count"]).map_err(|e| AppError::InternalError(e.to_string()))?;
            for s in &overview.by_status {
                w.write_record(&[&s.status, &s.count.to_string()])
                    .map_err(|e| AppError::InternalError(e.to_string()))?;
            }

            let data = w.into_inner().map_err(|e| AppError::InternalError(e.to_string()))?;
            Ok((data, "text/csv", "user_overview.csv".into()))
        }
        ReportFormat::Pdf => {
            let (doc, p, l) = PdfDocument::new("User Overview", Mm(210.0), Mm(297.0), "L1");
            let font = doc.add_builtin_font(BuiltinFont::Courier)
                .map_err(|e| AppError::InternalError(e.to_string()))?;
            let layer = doc.get_page(p).get_layer(l);

            let mut y = 280.0;
            layer.use_text("CivicSort - User Overview Report", 14.0, Mm(10.0), Mm(y), &font);
            y -= 10.0;
            layer.use_text(&format!("Generated: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M")), 9.0, Mm(10.0), Mm(y), &font);
            y -= 10.0;
            layer.use_text(&format!("Total Users: {}", overview.total_users), 10.0, Mm(10.0), Mm(y), &font);
            y -= 6.0;
            layer.use_text(&format!("Recent Logins (7d): {}", overview.recent_logins), 10.0, Mm(10.0), Mm(y), &font);
            y -= 10.0;

            layer.use_text("Users by Role:", 10.0, Mm(10.0), Mm(y), &font); y -= 6.0;
            for r in &overview.by_role {
                layer.use_text(&format!("  {} : {}", r.role, r.count), 9.0, Mm(10.0), Mm(y), &font);
                y -= 5.0;
            }
            y -= 5.0;
            layer.use_text("Users by Status:", 10.0, Mm(10.0), Mm(y), &font); y -= 6.0;
            for s in &overview.by_status {
                layer.use_text(&format!("  {} : {}", s.status, s.count), 9.0, Mm(10.0), Mm(y), &font);
                y -= 5.0;
            }

            let bytes = doc.save_to_bytes().map_err(|e| AppError::InternalError(e.to_string()))?;
            Ok((bytes, "application/pdf", "user_overview.pdf".into()))
        }
    }
}

/// Generate work order overview report
pub async fn generate_workorder_report(
    pool: &PgPool,
    format: &ReportFormat,
) -> Result<(Vec<u8>, &'static str, String), AppError> {
    let overview = admin_db::get_work_order_overview(pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    match format {
        ReportFormat::Csv => {
            let mut w = csv::Writer::from_writer(Vec::new());
            w.write_record(&["Metric", "Value"]).map_err(|e| AppError::InternalError(e.to_string()))?;
            w.write_record(&["Active Templates", &overview.total_templates.to_string()]).map_err(|e| AppError::InternalError(e.to_string()))?;
            w.write_record(&["Active Schedules", &overview.active_schedules.to_string()]).map_err(|e| AppError::InternalError(e.to_string()))?;
            w.write_record(&["Total Instances", &overview.total_instances.to_string()]).map_err(|e| AppError::InternalError(e.to_string()))?;
            w.write_record(&["Completion Rate (%)", &format!("{:.1}", overview.completion_rate)]).map_err(|e| AppError::InternalError(e.to_string()))?;
            w.write_record(&["Avg Completion (hrs)", &format!("{:.1}", overview.avg_completion_time_hours)]).map_err(|e| AppError::InternalError(e.to_string()))?;

            w.write_record(&["", ""]).map_err(|e| AppError::InternalError(e.to_string()))?;
            w.write_record(&["Status", "Count"]).map_err(|e| AppError::InternalError(e.to_string()))?;
            for s in &overview.by_status {
                w.write_record(&[&s.status, &s.count.to_string()]).map_err(|e| AppError::InternalError(e.to_string()))?;
            }

            let data = w.into_inner().map_err(|e| AppError::InternalError(e.to_string()))?;
            Ok((data, "text/csv", "workorder_overview.csv".into()))
        }
        ReportFormat::Pdf => {
            let (doc, p, l) = PdfDocument::new("Work Order Overview", Mm(210.0), Mm(297.0), "L1");
            let font = doc.add_builtin_font(BuiltinFont::Courier)
                .map_err(|e| AppError::InternalError(e.to_string()))?;
            let layer = doc.get_page(p).get_layer(l);

            let mut y = 280.0;
            layer.use_text("CivicSort - Work Order Overview", 14.0, Mm(10.0), Mm(y), &font); y -= 10.0;
            layer.use_text(&format!("Generated: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M")), 9.0, Mm(10.0), Mm(y), &font); y -= 10.0;
            layer.use_text(&format!("Active Templates: {}", overview.total_templates), 10.0, Mm(10.0), Mm(y), &font); y -= 6.0;
            layer.use_text(&format!("Active Schedules: {}", overview.active_schedules), 10.0, Mm(10.0), Mm(y), &font); y -= 6.0;
            layer.use_text(&format!("Completion Rate: {:.1}%", overview.completion_rate), 10.0, Mm(10.0), Mm(y), &font); y -= 6.0;
            layer.use_text(&format!("Avg Time: {:.1} hrs", overview.avg_completion_time_hours), 10.0, Mm(10.0), Mm(y), &font); y -= 10.0;

            layer.use_text("Instances by Status:", 10.0, Mm(10.0), Mm(y), &font); y -= 6.0;
            for s in &overview.by_status {
                layer.use_text(&format!("  {} : {}", s.status, s.count), 9.0, Mm(10.0), Mm(y), &font); y -= 5.0;
            }

            let bytes = doc.save_to_bytes().map_err(|e| AppError::InternalError(e.to_string()))?;
            Ok((bytes, "application/pdf", "workorder_overview.pdf".into()))
        }
    }
}

/// Generate campaign report
pub async fn generate_campaign_report(
    pool: &PgPool,
    format: &ReportFormat,
) -> Result<(Vec<u8>, &'static str, String), AppError> {
    let campaigns = admin_db::list_campaigns(pool, None, 1000, 0)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    match format {
        ReportFormat::Csv => {
            let mut w = csv::Writer::from_writer(Vec::new());
            w.write_record(&["Name", "Status", "Start", "End", "Region", "Audience"])
                .map_err(|e| AppError::InternalError(e.to_string()))?;
            for c in &campaigns {
                w.write_record(&[
                    &c.name,
                    &format!("{:?}", c.status),
                    &c.start_date.to_string(),
                    &c.end_date.to_string(),
                    c.target_region.as_deref().unwrap_or(""),
                    c.target_audience.as_deref().unwrap_or(""),
                ]).map_err(|e| AppError::InternalError(e.to_string()))?;
            }
            let data = w.into_inner().map_err(|e| AppError::InternalError(e.to_string()))?;
            Ok((data, "text/csv", "campaign_report.csv".into()))
        }
        ReportFormat::Pdf => {
            let (doc, p, l) = PdfDocument::new("Campaign Report", Mm(297.0), Mm(210.0), "L1");
            let font = doc.add_builtin_font(BuiltinFont::Courier)
                .map_err(|e| AppError::InternalError(e.to_string()))?;
            let layer = doc.get_page(p).get_layer(l);

            let mut y = 200.0;
            layer.use_text("CivicSort - Campaign Report", 14.0, Mm(10.0), Mm(y), &font); y -= 10.0;
            layer.use_text(&format!("Total Campaigns: {}", campaigns.len()), 9.0, Mm(10.0), Mm(y), &font); y -= 8.0;

            for c in campaigns.iter().take(35) {
                if y < 15.0 { break; }
                let line = format!("{:30} | {:10} | {} - {}",
                    truncate(&c.name, 30), format!("{:?}", c.status), c.start_date, c.end_date);
                layer.use_text(&line, 7.0, Mm(10.0), Mm(y), &font); y -= 4.0;
            }

            let bytes = doc.save_to_bytes().map_err(|e| AppError::InternalError(e.to_string()))?;
            Ok((bytes, "application/pdf", "campaign_report.pdf".into()))
        }
    }
}

fn kpi_to_csv(kpis: &DashboardKpis, from: NaiveDate, to: NaiveDate) -> Result<Vec<u8>, AppError> {
    let mut w = csv::Writer::from_writer(Vec::new());
    w.write_record(&["KPI", "Value", "Previous", "Trend (%)", "Period"])
        .map_err(|e| AppError::InternalError(e.to_string()))?;

    let period = format!("{} to {}", from, to);
    for (label, m) in [
        (&kpis.sorting_conversion_rate.label, &kpis.sorting_conversion_rate),
        (&kpis.template_reuse_rate.label, &kpis.template_reuse_rate),
        (&kpis.retention_30d.label, &kpis.retention_30d),
        (&kpis.retention_60d.label, &kpis.retention_60d),
        (&kpis.retention_90d.label, &kpis.retention_90d),
    ] {
        w.write_record(&[
            label, &format!("{:.2}", m.current), &format!("{:.2}", m.previous),
            &format!("{:.1}", m.trend), &period,
        ]).map_err(|e| AppError::InternalError(e.to_string()))?;
    }

    w.write_record(&["", "", "", "", ""]).map_err(|e| AppError::InternalError(e.to_string()))?;
    w.write_record(&["Counter", "Value", "", "", ""])
        .map_err(|e| AppError::InternalError(e.to_string()))?;
    for (k, v) in [
        ("Active Users", kpis.active_users),
        ("Tasks Completed", kpis.total_tasks_completed),
        ("Reviews Completed", kpis.total_reviews_completed),
        ("KB Entries", kpis.total_kb_entries),
        ("Active Campaigns", kpis.active_campaigns),
        ("Overdue Tasks", kpis.overdue_tasks),
        ("Pending Reviews", kpis.pending_reviews),
    ] {
        w.write_record(&[k, &v.to_string(), "", "", ""])
            .map_err(|e| AppError::InternalError(e.to_string()))?;
    }

    w.into_inner().map_err(|e| AppError::InternalError(e.to_string()))
}

fn kpi_to_pdf(kpis: &DashboardKpis, from: NaiveDate, to: NaiveDate) -> Result<Vec<u8>, AppError> {
    let (doc, p, l) = PdfDocument::new("KPI Report", Mm(210.0), Mm(297.0), "L1");
    let font = doc.add_builtin_font(BuiltinFont::Courier)
        .map_err(|e| AppError::InternalError(e.to_string()))?;
    let layer = doc.get_page(p).get_layer(l);

    let mut y = 280.0;
    layer.use_text("CivicSort - KPI Dashboard Report", 14.0, Mm(10.0), Mm(y), &font); y -= 8.0;
    layer.use_text(&format!("Period: {} to {}", from, to), 9.0, Mm(10.0), Mm(y), &font); y -= 6.0;
    layer.use_text(&format!("Generated: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M")), 9.0, Mm(10.0), Mm(y), &font); y -= 12.0;

    layer.use_text("KEY PERFORMANCE INDICATORS", 11.0, Mm(10.0), Mm(y), &font); y -= 8.0;
    layer.use_text(&"-".repeat(70), 7.0, Mm(10.0), Mm(y), &font); y -= 6.0;

    for m in [
        &kpis.sorting_conversion_rate, &kpis.template_reuse_rate,
        &kpis.retention_30d, &kpis.retention_60d, &kpis.retention_90d,
    ] {
        let trend_arrow = if m.trend > 0.0 { "+" } else { "" };
        let line = format!("{:40} {:>8.2}  ({}{}%)", m.label, m.current, trend_arrow, format!("{:.1}", m.trend));
        layer.use_text(&line, 9.0, Mm(10.0), Mm(y), &font); y -= 6.0;
    }

    y -= 8.0;
    layer.use_text("COUNTERS", 11.0, Mm(10.0), Mm(y), &font); y -= 8.0;
    layer.use_text(&"-".repeat(70), 7.0, Mm(10.0), Mm(y), &font); y -= 6.0;

    for (k, v) in [
        ("Active Users", kpis.active_users),
        ("Tasks Completed", kpis.total_tasks_completed),
        ("Reviews Completed", kpis.total_reviews_completed),
        ("KB Entries", kpis.total_kb_entries),
        ("Active Campaigns", kpis.active_campaigns),
        ("Overdue Tasks", kpis.overdue_tasks),
        ("Pending Reviews", kpis.pending_reviews),
    ] {
        layer.use_text(&format!("{:40} {:>8}", k, v), 9.0, Mm(10.0), Mm(y), &font); y -= 6.0;
    }

    doc.save_to_bytes().map_err(|e| AppError::InternalError(e.to_string()))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max - 3]) }
}
