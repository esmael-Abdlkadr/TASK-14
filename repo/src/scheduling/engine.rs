use chrono::{Datelike, Duration, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::inspection as insp_db;
use crate::errors::AppError;
use crate::models::{
    ReminderType, TaskCycle, TaskInstance, TaskInstanceStatus, TaskSchedule, TaskTemplate,
};

/// Generate upcoming task instances for a schedule based on its template's cycle.
/// Generates instances up to `horizon_days` into the future.
pub async fn generate_instances(
    pool: &PgPool,
    schedule: &TaskSchedule,
    template: &TaskTemplate,
    horizon_days: i64,
) -> Result<Vec<TaskInstance>, AppError> {
    let today = Utc::now().date_naive();
    let horizon = today + Duration::days(horizon_days);

    let end_bound = match schedule.end_date {
        Some(end) if end < horizon => end,
        _ => horizon,
    };

    let due_dates = compute_due_dates(&template.cycle, schedule.start_date, end_bound);
    let mut created = Vec::new();

    for due_date in due_dates {
        if due_date < today {
            continue; // Don't generate past instances during forward generation
        }

        let exists = insp_db::instance_exists(pool, schedule.id, due_date)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if !exists {
            let instance = insp_db::create_instance(
                pool,
                schedule.id,
                template.id,
                schedule.assigned_to,
                due_date,
                template.time_window_start,
                template.time_window_end,
                false,
                None,
                None,
            )
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

            created.push(instance);
        }
    }

    Ok(created)
}

/// Compute all due dates for a given cycle between start and end.
pub fn compute_due_dates(cycle: &TaskCycle, start: NaiveDate, end: NaiveDate) -> Vec<NaiveDate> {
    let mut dates = Vec::new();
    let mut current = start;

    while current <= end {
        dates.push(current);
        current = next_occurrence(cycle, current);
        if current <= *dates.last().unwrap() {
            break; // Safety: prevent infinite loop for one_time
        }
    }

    dates
}

/// Calculate the next due date from a given date based on cycle.
fn next_occurrence(cycle: &TaskCycle, from: NaiveDate) -> NaiveDate {
    match cycle {
        TaskCycle::Daily => from + Duration::days(1),
        TaskCycle::Weekly => from + Duration::weeks(1),
        TaskCycle::Biweekly => from + Duration::weeks(2),
        TaskCycle::Monthly => {
            let month = from.month();
            let year = from.year();
            if month == 12 {
                NaiveDate::from_ymd_opt(year + 1, 1, from.day().min(28))
                    .unwrap_or(from + Duration::days(30))
            } else {
                NaiveDate::from_ymd_opt(year, month + 1, from.day().min(28))
                    .unwrap_or(from + Duration::days(30))
            }
        }
        TaskCycle::Quarterly => {
            let month = from.month();
            let year = from.year();
            let new_month = month + 3;
            if new_month > 12 {
                NaiveDate::from_ymd_opt(year + 1, new_month - 12, from.day().min(28))
                    .unwrap_or(from + Duration::days(90))
            } else {
                NaiveDate::from_ymd_opt(year, new_month, from.day().min(28))
                    .unwrap_or(from + Duration::days(90))
            }
        }
        TaskCycle::OneTime => from + Duration::days(999999), // effectively no repeat
    }
}

/// Process overdue instances: mark them overdue, check fault tolerance,
/// create makeup instances if allowed.
pub async fn process_overdue(pool: &PgPool) -> Result<OverdueReport, AppError> {
    let overdue_instances = insp_db::find_overdue_instances(pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let mut marked_overdue = 0u32;
    let mut marked_missed = 0u32;
    let mut makeups_created = 0u32;
    let mut reminders_sent = 0u32;

    for instance in overdue_instances {
        let template = insp_db::get_template(pool, instance.template_id)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let template = match template {
            Some(t) => t,
            None => continue,
        };

        // Mark as overdue
        insp_db::update_instance_status(pool, instance.id, &TaskInstanceStatus::Overdue)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        marked_overdue += 1;

        // Check if makeup is possible
        if template.makeup_allowed {
            let already_has_makeup = insp_db::has_makeup_instance(pool, instance.id)
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;

            if !already_has_makeup {
                let makeup_deadline =
                    Utc::now() + Duration::hours(template.makeup_deadline_hours as i64);

                let today = Utc::now().date_naive();
                insp_db::create_instance(
                    pool,
                    instance.schedule_id,
                    instance.template_id,
                    instance.assigned_to,
                    today,
                    template.time_window_start,
                    template.time_window_end,
                    true,
                    Some(instance.id),
                    Some(makeup_deadline),
                )
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;
                makeups_created += 1;

                // Create makeup deadline reminder
                let _ = insp_db::create_reminder(
                    pool,
                    instance.assigned_to,
                    Some(instance.id),
                    &ReminderType::MakeupDeadline,
                    &format!("Makeup available: {}", template.name),
                    &format!(
                        "You missed '{}' on {}. Complete the makeup task within {} hours to avoid it counting as missed.",
                        template.name,
                        instance.due_date,
                        template.makeup_deadline_hours
                    ),
                    Some(today),
                )
                .await;
                reminders_sent += 1;
            }
        }

        // Check makeup deadline expiration → mark as missed
        if instance.is_makeup {
            if let Some(deadline) = instance.makeup_deadline {
                if Utc::now() > deadline {
                    insp_db::update_instance_status(pool, instance.id, &TaskInstanceStatus::Missed)
                        .await
                        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
                    marked_missed += 1;

                    // Check fault tolerance
                    let miss_count = insp_db::count_misses_in_window(
                        pool,
                        instance.schedule_id,
                        template.miss_window_days,
                    )
                    .await
                    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

                    if miss_count > template.allowed_misses as i64 {
                        let _ = insp_db::create_reminder(
                            pool,
                            instance.assigned_to,
                            Some(instance.id),
                            &ReminderType::MissedWarning,
                            &format!("Fault tolerance exceeded: {}", template.name),
                            &format!(
                                "You have missed {} occurrences of '{}' in the last {} days (allowed: {}). Please address immediately.",
                                miss_count, template.name, template.miss_window_days, template.allowed_misses
                            ),
                            None,
                        )
                        .await;
                        reminders_sent += 1;
                    }
                }
            }
        }

        // Send overdue reminder
        let _ = insp_db::create_reminder(
            pool,
            instance.assigned_to,
            Some(instance.id),
            &ReminderType::Overdue,
            &format!("Overdue: {}", template.name),
            &format!(
                "'{}' was due on {} and has not been completed.",
                template.name, instance.due_date
            ),
            Some(instance.due_date),
        )
        .await;
        reminders_sent += 1;
    }

    Ok(OverdueReport {
        marked_overdue,
        marked_missed,
        makeups_created,
        reminders_sent,
    })
}

/// Generate upcoming reminders for tasks due soon (within next 24 hours)
pub async fn generate_upcoming_reminders(pool: &PgPool) -> Result<u32, AppError> {
    let tomorrow = Utc::now().date_naive() + Duration::days(1);
    let today = Utc::now().date_naive();

    let upcoming = sqlx::query_as::<_, TaskInstance>(
        r#"
        SELECT * FROM task_instances
        WHERE status = 'scheduled'::task_instance_status
          AND due_date BETWEEN $1 AND $2
        "#,
    )
    .bind(today)
    .bind(tomorrow)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let mut count = 0u32;
    for instance in upcoming {
        let template = insp_db::get_template(pool, instance.template_id)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if let Some(t) = template {
            let reminder_type = if instance.due_date == today {
                ReminderType::DueSoon
            } else {
                ReminderType::Upcoming
            };

            let _ = insp_db::create_reminder(
                pool,
                instance.assigned_to,
                Some(instance.id),
                &reminder_type,
                &format!("Upcoming: {}", t.name),
                &format!(
                    "'{}' is due on {} between {} and {}.",
                    t.name, instance.due_date, instance.window_start, instance.window_end
                ),
                Some(instance.due_date),
            )
            .await;
            count += 1;
        }
    }

    Ok(count)
}

#[derive(Debug, serde::Serialize)]
pub struct OverdueReport {
    pub marked_overdue: u32,
    pub marked_missed: u32,
    pub makeups_created: u32,
    pub reminders_sent: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_daily_dates() {
        let start = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 4, 5).unwrap();
        let dates = compute_due_dates(&TaskCycle::Daily, start, end);
        assert_eq!(dates.len(), 5);
        assert_eq!(dates[0], start);
        assert_eq!(dates[4], end);
    }

    #[test]
    fn test_compute_weekly_dates() {
        let start = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 4, 30).unwrap();
        let dates = compute_due_dates(&TaskCycle::Weekly, start, end);
        assert_eq!(dates.len(), 5); // Apr 1, 8, 15, 22, 29
    }

    #[test]
    fn test_compute_monthly_dates() {
        let start = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 6, 30).unwrap();
        let dates = compute_due_dates(&TaskCycle::Monthly, start, end);
        assert_eq!(dates.len(), 6); // Jan-Jun
    }

    #[test]
    fn test_compute_one_time() {
        let start = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let dates = compute_due_dates(&TaskCycle::OneTime, start, end);
        assert_eq!(dates.len(), 1);
    }

    #[test]
    fn test_compute_biweekly_dates() {
        let start = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        let dates = compute_due_dates(&TaskCycle::Biweekly, start, end);
        assert_eq!(dates.len(), 3); // Apr 1, 15, 29
    }

    #[test]
    fn test_compute_quarterly_dates() {
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let dates = compute_due_dates(&TaskCycle::Quarterly, start, end);
        assert_eq!(dates.len(), 4); // Jan, Apr, Jul, Oct
    }
}
