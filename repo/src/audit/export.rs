use std::io::Write;

use printpdf::*;
use sqlx::PgPool;

use crate::db::audit::query_audit_log;
use crate::errors::AppError;
use crate::models::{AuditExportQuery, AuditLogEntry, AuditLogQuery, ExportFormat};

/// Export audit log entries to CSV format (bytes)
pub async fn export_csv(pool: &PgPool, query: &AuditExportQuery) -> Result<Vec<u8>, AppError> {
    let audit_query = AuditLogQuery {
        user_id: None,
        action: query.action.clone(),
        resource_type: None,
        from_date: query.from_date,
        to_date: query.to_date,
        page: Some(1),
        page_size: Some(10000), // Export up to 10k records
    };

    let (entries, _total) = query_audit_log(pool, &audit_query)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let mut writer = csv::Writer::from_writer(Vec::new());

    // Header
    writer
        .write_record(&[
            "Event ID",
            "Timestamp",
            "Username",
            "Role",
            "Action",
            "Resource Type",
            "Resource ID",
            "Details",
            "IP Address",
            "Entry Hash",
        ])
        .map_err(|e| AppError::InternalError(format!("CSV write error: {}", e)))?;

    for entry in &entries {
        writer
            .write_record(&[
                entry.event_id.to_string(),
                entry.created_at.to_rfc3339(),
                entry.username.clone(),
                format!("{:?}", entry.role),
                entry.action.clone(),
                entry.resource_type.clone().unwrap_or_default(),
                entry.resource_id.clone().unwrap_or_default(),
                entry
                    .details
                    .as_ref()
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
                entry.ip_address.clone().unwrap_or_default(),
                entry.entry_hash.clone(),
            ])
            .map_err(|e| AppError::InternalError(format!("CSV write error: {}", e)))?;
    }

    writer
        .into_inner()
        .map_err(|e| AppError::InternalError(format!("CSV flush error: {}", e)))
}

/// Export audit log entries to PDF format (bytes)
pub async fn export_pdf(pool: &PgPool, query: &AuditExportQuery) -> Result<Vec<u8>, AppError> {
    let audit_query = AuditLogQuery {
        user_id: None,
        action: query.action.clone(),
        resource_type: None,
        from_date: query.from_date,
        to_date: query.to_date,
        page: Some(1),
        page_size: Some(10000),
    };

    let (entries, _total) = query_audit_log(pool, &audit_query)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let (doc, page1, layer1) =
        PdfDocument::new("CivicSort Audit Log", Mm(297.0), Mm(210.0), "Layer 1");

    let font = doc
        .add_builtin_font(BuiltinFont::Courier)
        .map_err(|e| AppError::InternalError(format!("PDF font error: {}", e)))?;

    let current_layer = doc.get_page(page1).get_layer(layer1);

    // Title
    current_layer.use_text(
        "CivicSort - Audit Log Report",
        14.0,
        Mm(10.0),
        Mm(200.0),
        &font,
    );

    current_layer.use_text(
        &format!("Generated: {}", chrono::Utc::now().to_rfc3339()),
        9.0,
        Mm(10.0),
        Mm(193.0),
        &font,
    );

    current_layer.use_text(
        &format!("Total entries: {}", entries.len()),
        9.0,
        Mm(10.0),
        Mm(187.0),
        &font,
    );

    // Table header
    let mut y_pos = 178.0;
    let header = "Timestamp                    | User          | Action               | Resource";
    current_layer.use_text(header, 7.0, Mm(10.0), Mm(y_pos), &font);
    y_pos -= 4.0;

    current_layer.use_text(
        &"-".repeat(90),
        7.0,
        Mm(10.0),
        Mm(y_pos),
        &font,
    );
    y_pos -= 4.0;

    for entry in entries.iter().take(40) {
        // Fit ~40 entries per page
        if y_pos < 15.0 {
            break;
        }
        let line = format!(
            "{} | {:13} | {:20} | {}:{}",
            entry.created_at.format("%Y-%m-%d %H:%M:%S"),
            truncate_str(&entry.username, 13),
            truncate_str(&entry.action, 20),
            entry.resource_type.as_deref().unwrap_or("-"),
            entry.resource_id.as_deref().unwrap_or("-"),
        );
        current_layer.use_text(&line, 7.0, Mm(10.0), Mm(y_pos), &font);
        y_pos -= 4.0;
    }

    let pdf_bytes = doc
        .save_to_bytes()
        .map_err(|e| AppError::InternalError(format!("PDF save error: {}", e)))?;

    Ok(pdf_bytes)
}

/// Export audit log in the requested format
pub async fn export_audit_log(
    pool: &PgPool,
    query: &AuditExportQuery,
) -> Result<(Vec<u8>, &'static str, &'static str), AppError> {
    match query.format {
        ExportFormat::Csv => {
            let data = export_csv(pool, query).await?;
            Ok((data, "text/csv", "audit_log.csv"))
        }
        ExportFormat::Pdf => {
            let data = export_pdf(pool, query).await?;
            Ok((data, "application/pdf", "audit_log.pdf"))
        }
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}
