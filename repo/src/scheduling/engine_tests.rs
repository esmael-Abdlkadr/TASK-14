#[cfg(test)]
mod extended_engine_tests {
    use crate::scheduling::engine::compute_due_dates;
    use crate::models::TaskCycle;
    use chrono::{Datelike, NaiveDate};

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    // ── Daily cycle ─────────────────────────────────────────

    #[test]
    fn daily_single_day() {
        let dates = compute_due_dates(&TaskCycle::Daily, d(2026, 4, 1), d(2026, 4, 1));
        assert_eq!(dates.len(), 1);
        assert_eq!(dates[0], d(2026, 4, 1));
    }

    #[test]
    fn daily_7_days() {
        let dates = compute_due_dates(&TaskCycle::Daily, d(2026, 4, 1), d(2026, 4, 7));
        assert_eq!(dates.len(), 7);
    }

    #[test]
    fn daily_across_month_boundary() {
        let dates = compute_due_dates(&TaskCycle::Daily, d(2026, 3, 30), d(2026, 4, 2));
        assert_eq!(dates.len(), 4);
        assert_eq!(dates[1], d(2026, 3, 31));
        assert_eq!(dates[2], d(2026, 4, 1));
    }

    // ── Weekly cycle ────────────────────────────────────────

    #[test]
    fn weekly_exact_4_weeks() {
        let dates = compute_due_dates(&TaskCycle::Weekly, d(2026, 4, 1), d(2026, 4, 22));
        assert_eq!(dates.len(), 4);
        assert_eq!(dates[3], d(2026, 4, 22));
    }

    #[test]
    fn weekly_less_than_one_week() {
        let dates = compute_due_dates(&TaskCycle::Weekly, d(2026, 4, 1), d(2026, 4, 5));
        assert_eq!(dates.len(), 1);
    }

    // ── Biweekly cycle ──────────────────────────────────────

    #[test]
    fn biweekly_two_months() {
        let dates = compute_due_dates(&TaskCycle::Biweekly, d(2026, 1, 1), d(2026, 2, 28));
        // Jan 1, 15, 29, Feb 12, 26
        assert_eq!(dates.len(), 5);
    }

    // ── Monthly cycle ───────────────────────────────────────

    #[test]
    fn monthly_jan_to_dec() {
        let dates = compute_due_dates(&TaskCycle::Monthly, d(2026, 1, 15), d(2026, 12, 31));
        assert_eq!(dates.len(), 12);
    }

    #[test]
    fn monthly_31st_clamps_to_28() {
        // Starting on Jan 31, months without 31 days should clamp
        let dates = compute_due_dates(&TaskCycle::Monthly, d(2026, 1, 31), d(2026, 4, 30));
        assert!(dates.len() >= 3);
        // Feb should use 28
        assert_eq!(dates[1].day(), 28);
    }

    #[test]
    fn monthly_across_year() {
        let dates = compute_due_dates(&TaskCycle::Monthly, d(2025, 11, 15), d(2026, 3, 15));
        assert_eq!(dates.len(), 5); // Nov, Dec, Jan, Feb, Mar
    }

    // ── Quarterly cycle ─────────────────────────────────────

    #[test]
    fn quarterly_full_year() {
        let dates = compute_due_dates(&TaskCycle::Quarterly, d(2026, 1, 1), d(2026, 12, 31));
        assert_eq!(dates.len(), 4);
    }

    #[test]
    fn quarterly_across_year_boundary() {
        let dates = compute_due_dates(&TaskCycle::Quarterly, d(2025, 10, 1), d(2026, 7, 1));
        assert_eq!(dates.len(), 4); // Oct 2025, Jan, Apr, Jul 2026
    }

    // ── OneTime cycle ───────────────────────────────────────

    #[test]
    fn one_time_always_single() {
        let dates = compute_due_dates(&TaskCycle::OneTime, d(2026, 4, 1), d(2030, 12, 31));
        assert_eq!(dates.len(), 1);
    }

    // ── Edge cases ──────────────────────────────────────────

    #[test]
    fn end_before_start_empty() {
        let dates = compute_due_dates(&TaskCycle::Daily, d(2026, 4, 10), d(2026, 4, 1));
        assert!(dates.is_empty());
    }

    #[test]
    fn same_start_and_end() {
        let dates = compute_due_dates(&TaskCycle::Weekly, d(2026, 4, 1), d(2026, 4, 1));
        assert_eq!(dates.len(), 1);
    }
}
