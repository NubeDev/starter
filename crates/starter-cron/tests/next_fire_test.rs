//! Unit tests for [`starter_cron::next_fire`].
//!
//! Four cases, one per behaviour we promise downstream:
//!
//! 1. **Weekly** — Monday-09:00 schedule rolls forward to the right
//!    Monday from a Wednesday `now`.
//! 2. **Every 15 minutes** — `*/15` in the minute field lands on the
//!    next quarter-hour boundary.
//! 3. **Business hours** — 09:00 every weekday skips Saturday and
//!    Sunday from a Friday-evening `now`.
//! 4. **Malformed** — gibberish surfaces [`CronError::Parse`].

use chrono::{Datelike, TimeZone, Timelike, Utc, Weekday};
use starter_cron::{next_fire, CronError};

#[test]
fn weekly_monday_morning_rolls_to_next_monday() {
    // Wednesday 2026-05-13 12:00:00 UTC.
    let now = Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap();
    // "At 09:00 every Monday."
    let fire = next_fire(now, "0 0 9 * * MON").expect("valid weekly cron");

    assert_eq!(fire.weekday(), Weekday::Mon);
    assert_eq!(fire.hour(), 9);
    assert_eq!(fire.minute(), 0);
    assert!(fire > now);
    // Must land on the *next* Monday — within a week.
    assert!((fire - now).num_days() < 7);
}

#[test]
fn every_fifteen_minutes_lands_on_next_quarter_hour() {
    // 2026-05-13 12:07:30 UTC — next slot is 12:15:00.
    let now = Utc.with_ymd_and_hms(2026, 5, 13, 12, 7, 30).unwrap();
    let fire = next_fire(now, "0 */15 * * * *").expect("valid 15-minute cron");

    let expected = Utc.with_ymd_and_hms(2026, 5, 13, 12, 15, 0).unwrap();
    assert_eq!(fire, expected);
}

#[test]
fn business_hours_skips_the_weekend() {
    // Friday 2026-05-15 18:00:00 UTC — past today's 09:00 slot,
    // weekend in the way, so next fire is Monday 09:00.
    let now = Utc.with_ymd_and_hms(2026, 5, 15, 18, 0, 0).unwrap();
    let fire = next_fire(now, "0 0 9 * * MON-FRI").expect("valid business-hours cron");

    assert_eq!(fire.weekday(), Weekday::Mon);
    assert_eq!(fire.hour(), 9);
    // Monday after that Friday is 2026-05-18.
    assert_eq!(fire.day(), 18);
    assert_eq!(fire.month(), 5);
    assert_eq!(fire.year(), 2026);
}

#[test]
fn malformed_expression_returns_parse_error() {
    let now = Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap();
    let err = next_fire(now, "not a cron expression").expect_err("must reject gibberish");

    match err {
        CronError::Parse { expr, .. } => assert_eq!(expr, "not a cron expression"),
        other => panic!("expected CronError::Parse, got {other:?}"),
    }
}
