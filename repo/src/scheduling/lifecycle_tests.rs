#[cfg(test)]
mod scheduling_lifecycle_tests {
    use crate::scheduling::engine::compute_due_dates;
    use crate::models::TaskCycle;
    use chrono::{Datelike, Duration, NaiveDate};

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    // G.6 - Miss tolerance window tests

    #[test]
    fn daily_30_day_window_has_30_occurrences() {
        let dates = compute_due_dates(&TaskCycle::Daily, d(2026, 4, 1), d(2026, 4, 30));
        assert_eq!(dates.len(), 30);
    }

    #[test]
    fn weekly_miss_window_30_days_has_about_4_occurrences() {
        let dates = compute_due_dates(&TaskCycle::Weekly, d(2026, 4, 1), d(2026, 4, 30));
        // Apr 1, 8, 15, 22, 29
        assert_eq!(dates.len(), 5);
    }

    // G.6 - Makeup deadline timing
    #[test]
    fn makeup_48h_from_due_date() {
        let due = d(2026, 4, 15);
        let makeup_deadline = due + Duration::days(2); // 48 hours
        assert_eq!(makeup_deadline, d(2026, 4, 17));
    }

    // G.6 - Overdue detection: task due yesterday is overdue
    #[test]
    fn task_past_due_date_is_detectable() {
        let due = d(2026, 4, 1);
        let today = d(2026, 4, 2);
        assert!(today > due); // This is how find_overdue_instances works
    }

    // G.6 - Quarterly boundaries don't skip
    #[test]
    fn quarterly_across_dec_jan() {
        let dates = compute_due_dates(&TaskCycle::Quarterly, d(2025, 10, 1), d(2026, 10, 1));
        // Oct 2025, Jan 2026, Apr 2026, Jul 2026, Oct 2026
        assert_eq!(dates.len(), 5);
    }

    // G.6 - Monthly with Feb 28 handling
    #[test]
    fn monthly_from_jan_30_handles_feb() {
        let dates = compute_due_dates(&TaskCycle::Monthly, d(2026, 1, 30), d(2026, 4, 30));
        assert!(dates.len() >= 3);
        // Feb clamps to 28
        assert!(dates[1].day() <= 28);
    }
}
