//! Cron expression parser for the TaskScheduler
//!
//! Supports: *, */N, exact numbers (0-59), ranges (N-M)

use chrono::{Datelike, Timelike};

/// Parsed cron expression (5 fields: min hour day-month month day-week)
#[derive(Debug, Clone, PartialEq)]
pub struct CronSchedule {
    pub minutes: CronField,
    pub hours: CronField,
    pub day_of_month: CronField,
    pub month: CronField,
    pub day_of_week: CronField,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CronField {
    Any,
    Exact(Vec<u32>),
    Step(u32),
    Range(u32, u32),
}

impl CronField {
    fn matches(&self, value: u32) -> bool {
        match self {
            CronField::Any => true,
            CronField::Exact(vals) => vals.contains(&value),
            CronField::Step(step) => value % step == 0,
            CronField::Range(start, end) => value >= *start && value <= *end,
        }
    }
}

/// Parse a cron expression like "0 3 * * *" or "*/15 * * * *"
pub fn parse_cron(expr: &str) -> Option<CronSchedule> {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() != 5 {
        return None;
    }

    Some(CronSchedule {
        minutes: parse_field(parts[0], 0, 59)?,
        hours: parse_field(parts[1], 0, 23)?,
        day_of_month: parse_field(parts[2], 1, 31)?,
        month: parse_field(parts[3], 1, 12)?,
        day_of_week: parse_field(parts[4], 0, 6)?,
    })
}

fn parse_field(s: &str, min: u32, max: u32) -> Option<CronField> {
    if s == "*" {
        return Some(CronField::Any);
    }
    if let Some(step_str) = s.strip_prefix("*/") {
        let step: u32 = step_str.parse().ok()?;
        return Some(CronField::Step(step));
    }
    if s.contains('-') {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() == 2 {
            let start: u32 = parts[0].parse().ok()?;
            let end: u32 = parts[1].parse().ok()?;
            if start >= min && end <= max && start <= end {
                return Some(CronField::Range(start, end));
            }
        }
        return None;
    }
    if s.contains(',') {
        let vals: Vec<u32> = s.split(',').filter_map(|v| v.parse().ok()).collect();
        if vals.iter().all(|v| *v >= min && *v <= max) {
            return Some(CronField::Exact(vals));
        }
        return None;
    }
    let val: u32 = s.parse().ok()?;
    if val >= min && val <= max {
        Some(CronField::Exact(vec![val]))
    } else {
        None
    }
}

/// Compute the next execution time from a cron schedule and a reference time.
pub fn next_run(schedule: &CronSchedule, from: chrono::DateTime<chrono::Utc>) -> Option<chrono::DateTime<chrono::Utc>> {
    let mut candidate = from + chrono::Duration::minutes(1);
    // Try up to 2 years ahead
    let limit = from + chrono::Duration::days(730);
    while candidate <= limit {
        let minute = candidate.time().minute();
        let hour = candidate.time().hour();
        let dom = candidate.date_naive().day();
        let month = candidate.date_naive().month();
        let dow = candidate.date_naive().weekday().num_days_from_sunday();

        if schedule.minutes.matches(minute)
            && schedule.hours.matches(hour)
            && schedule.day_of_month.matches(dom)
            && schedule.month.matches(month)
            && schedule.day_of_week.matches(dow)
        {
            return Some(candidate);
        }
        candidate = candidate + chrono::Duration::minutes(1);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_any() {
        let s = parse_cron("* * * * *").unwrap();
        assert_eq!(s.minutes, CronField::Any);
        assert_eq!(s.hours, CronField::Any);
    }

    #[test]
    fn test_parse_exact() {
        let s = parse_cron("0 3 * * *").unwrap();
        assert_eq!(s.minutes, CronField::Exact(vec![0]));
        assert_eq!(s.hours, CronField::Exact(vec![3]));
    }

    #[test]
    fn test_parse_step() {
        let s = parse_cron("*/15 * * * *").unwrap();
        assert_eq!(s.minutes, CronField::Step(15));
    }

    #[test]
    fn test_next_run_daily() {
        let s = parse_cron("0 3 * * *").unwrap();
        let from = chrono::Utc::now();
        let next = next_run(&s, from).unwrap();
        assert_eq!(next.time().minute(), 0);
        assert_eq!(next.time().hour(), 3);
    }
}
