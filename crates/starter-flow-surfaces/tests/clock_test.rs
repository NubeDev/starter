//! Unit tests for [`starter_flow_surfaces::clock`].
//!
//! These exercise the [`Clock`] seam without touching PG: that
//! makes them the fast feedback loop the Phase B.2 tick-loop
//! test will lean on once it lands. PG-touching tests live in
//! `tests/register_test.rs` behind the `--ignored` gate.

use std::sync::Arc;

use chrono::{Duration, TimeZone, Utc};

use starter_flow_surfaces::clock::{Clock, SystemClock, TestClock};

#[test]
fn system_clock_returns_recent_now() {
    let before = Utc::now();
    let observed = SystemClock::new().now();
    let after = Utc::now();
    assert!(before <= observed && observed <= after, "now() out of bounds");
}

#[test]
fn test_clock_holds_seed_until_mutated() {
    let seed = Utc.with_ymd_and_hms(2026, 5, 24, 8, 0, 0).unwrap();
    let clock = TestClock::new(seed);
    assert_eq!(clock.now(), seed);
    assert_eq!(clock.now(), seed, "two reads return the same instant");
}

#[test]
fn test_clock_advance_moves_now_forward() {
    let seed = Utc.with_ymd_and_hms(2026, 5, 24, 8, 0, 0).unwrap();
    let clock = TestClock::new(seed);
    clock.advance(Duration::seconds(60));
    assert_eq!(clock.now(), seed + Duration::seconds(60));
    clock.advance(Duration::hours(1));
    assert_eq!(clock.now(), seed + Duration::seconds(60) + Duration::hours(1));
}

#[test]
fn test_clock_set_replaces_now() {
    let clock = TestClock::epoch();
    let when = Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap();
    clock.set(when);
    assert_eq!(clock.now(), when);
}

#[test]
fn test_clock_is_shared_across_clones() {
    // Cloning a `TestClock` shares the inner `Arc<Mutex<…>>`, so a
    // mutation through one handle is visible through the other —
    // matches the way the Phase B.2 tick loop will see time
    // advance from the test thread's vantage point.
    let clock = TestClock::epoch();
    let other = clock.clone();
    clock.advance(Duration::seconds(5));
    assert_eq!(other.now(), clock.now());
}

#[test]
fn test_clock_works_through_dyn_trait_object() {
    // `service::FlowAsService` holds an `Arc<dyn Clock>`; confirm
    // the dyn dispatch path matches the concrete one so the seam
    // stays honest.
    let clock = TestClock::new(Utc.with_ymd_and_hms(2026, 5, 24, 8, 0, 0).unwrap());
    let erased: Arc<dyn Clock> = Arc::new(clock.clone());
    assert_eq!(erased.now(), clock.now());
    clock.advance(Duration::minutes(10));
    assert_eq!(erased.now(), clock.now());
}
