//! Integration tests for [`starter_prefs::store::PgPrefsStore`].
//!
//! Port of `tests/sqlite_store.rs` for the Postgres backend. The test
//! contracts are identical — the three Phase 1 SCOPE.md smoke-test
//! properties apply equally to both backends:
//!
//! 1. **Upsert-then-get round-trip preserves every field.**
//! 2. **NULL-valued columns come back NULL.**
//! 3. **The multi-org pattern works for any N.**
//!
//! Tests spin up an ephemeral Postgres container via the
//! `starter-store-postgres` testcontainers helper. Each test is
//! marked `#[ignore = "requires docker"]` so CI can run them
//! explicitly:
//!
//! ```text
//! cargo test -p starter-prefs --features postgres \
//!     -- --ignored
//! ```

#![cfg(feature = "postgres")]

use starter_prefs::resolver::{OrgPrefsRow, StringPref, UnitPref, UserPrefsRow};
use starter_prefs::store::{PrefsStore, PgPrefsStore};
use starter_spi::preferences::{
    DateFormat, NumberFormat, Theme, TimeFormat, UnitSystem, WeekStart,
};
use starter_spi::units::Unit;
use starter_store_postgres::testing::{with_database, ContainerGuard};

/// Construct a fresh Postgres store with migrations applied.
///
/// Returns the store together with the `ContainerGuard` — the
/// caller must keep the guard alive for the entire test or the
/// container will be torn down mid-test.
async fn fresh_store() -> (PgPrefsStore, ContainerGuard) {
    let (pool, guard) = with_database().await;
    let store = PgPrefsStore::new(pool.sqlx().clone());
    store.migrate().await.expect("apply postgres prefs migrations");
    (store, guard)
}

/// A fully-populated user row — every column carries an explicit
/// non-null value. The roundtrip test reads it back and asserts
/// byte-equal.
fn full_user_row() -> UserPrefsRow {
    UserPrefsRow {
        timezone: Some(StringPref::Explicit("Australia/Brisbane".to_owned())),
        locale: Some("en-AU".to_owned()),
        language: Some("en".to_owned()),
        unit_system: Some(UnitSystem::Metric),
        // BBQ case: explicit °F override on an otherwise-metric row.
        temperature_unit: Some(UnitPref::Explicit(Unit::Fahrenheit)),
        pressure_unit: Some(UnitPref::Explicit(Unit::Kilopascal)),
        speed_unit: Some(UnitPref::Explicit(Unit::KilometerPerHour)),
        length_unit: Some(UnitPref::Explicit(Unit::Meter)),
        // Also exercise the `Auto` sentinel — it must round-trip.
        mass_unit: Some(UnitPref::Auto),
        date_format: Some(DateFormat::IsoYMD),
        time_format: Some(TimeFormat::H24),
        week_start: Some(WeekStart::Monday),
        number_format: Some(NumberFormat::CommaDot),
        currency: Some(StringPref::Auto),
        theme: Some(Theme::Dark),
    }
}

/// A fully-populated org row, mirroring [`full_user_row`] minus
/// `theme` (org rows have no theme column per the SCOPE Decisions
/// block).
fn full_org_row() -> OrgPrefsRow {
    OrgPrefsRow {
        timezone: Some(StringPref::Explicit("Australia/Brisbane".to_owned())),
        locale: Some("en-AU".to_owned()),
        language: Some("en".to_owned()),
        unit_system: Some(UnitSystem::Metric),
        temperature_unit: Some(UnitPref::Explicit(Unit::Celsius)),
        pressure_unit: Some(UnitPref::Auto),
        speed_unit: Some(UnitPref::Explicit(Unit::KilometerPerHour)),
        length_unit: Some(UnitPref::Explicit(Unit::Meter)),
        mass_unit: Some(UnitPref::Explicit(Unit::Kilogram)),
        date_format: Some(DateFormat::DmySlash),
        time_format: Some(TimeFormat::H12),
        week_start: Some(WeekStart::Sunday),
        number_format: Some(NumberFormat::DotComma),
        currency: Some(StringPref::Explicit("AUD".to_owned())),
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn user_row_round_trip_preserves_every_field() {
    let (store, _guard) = fresh_store().await;
    let written = full_user_row();
    store
        .upsert_user_prefs("alice", "ws1", written.clone())
        .await
        .expect("upsert user prefs");

    let read = store
        .get_user_prefs("alice", "ws1")
        .await
        .expect("get user prefs")
        .expect("row exists");

    assert_eq!(read, written, "user row must round-trip verbatim");
}

#[tokio::test]
#[ignore = "requires docker"]
async fn org_row_round_trip_preserves_every_field() {
    let (store, _guard) = fresh_store().await;
    let written = full_org_row();
    store
        .upsert_org_prefs("ws1", written.clone())
        .await
        .expect("upsert org prefs");

    let read = store
        .get_org_prefs("ws1")
        .await
        .expect("get org prefs")
        .expect("row exists");

    assert_eq!(read, written, "org row must round-trip verbatim");
}

#[tokio::test]
#[ignore = "requires docker"]
async fn null_columns_round_trip_as_none() {
    let (store, _guard) = fresh_store().await;
    // A sparse user row: only locale set, everything else NULL.
    // No implicit defaulting at the storage layer — the resolver
    // does R3, not the store.
    let sparse = UserPrefsRow {
        locale: Some("fr-FR".to_owned()),
        ..UserPrefsRow::default()
    };
    store
        .upsert_user_prefs("bob", "ws1", sparse.clone())
        .await
        .expect("upsert sparse user row");

    let read = store
        .get_user_prefs("bob", "ws1")
        .await
        .expect("get user prefs")
        .expect("row exists");

    assert_eq!(read.locale.as_deref(), Some("fr-FR"));
    // Every other field must still be None — the storage layer
    // never coerces NULL into a default.
    assert!(read.timezone.is_none());
    assert!(read.language.is_none());
    assert!(read.unit_system.is_none());
    assert!(read.temperature_unit.is_none());
    assert!(read.pressure_unit.is_none());
    assert!(read.speed_unit.is_none());
    assert!(read.length_unit.is_none());
    assert!(read.mass_unit.is_none());
    assert!(read.date_format.is_none());
    assert!(read.time_format.is_none());
    assert!(read.week_start.is_none());
    assert!(read.number_format.is_none());
    assert!(read.currency.is_none());
    assert!(read.theme.is_none());
}

#[tokio::test]
#[ignore = "requires docker"]
async fn missing_row_returns_none() {
    let (store, _guard) = fresh_store().await;
    assert!(store
        .get_user_prefs("nobody", "ws1")
        .await
        .expect("query missing user")
        .is_none());
    assert!(store
        .get_org_prefs("nowhere")
        .await
        .expect("query missing org")
        .is_none());
}

#[tokio::test]
#[ignore = "requires docker"]
async fn upsert_overwrites_existing_row() {
    let (store, _guard) = fresh_store().await;
    store
        .upsert_user_prefs("alice", "ws1", full_user_row())
        .await
        .unwrap();

    // Second write swaps the entire row.
    let replacement = UserPrefsRow {
        locale: Some("de-DE".to_owned()),
        theme: Some(Theme::Light),
        ..UserPrefsRow::default()
    };
    store
        .upsert_user_prefs("alice", "ws1", replacement.clone())
        .await
        .unwrap();

    let read = store.get_user_prefs("alice", "ws1").await.unwrap().unwrap();
    assert_eq!(read, replacement);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn multi_org_rows_per_user_are_independent_for_any_n() {
    let (store, _guard) = fresh_store().await;

    // SCOPE: "a user belongs to N orgs and has one row per
    // (user_id, workspace_id)". Drive N up to a meaningful spread so
    // we exercise more than the boundary cases.
    const N: usize = 7;
    let workspaces: Vec<String> = (0..N).map(|i| format!("ws-{i:02}")).collect();

    for (i, ws) in workspaces.iter().enumerate() {
        let row = UserPrefsRow {
            // Encode the index into a column so each row is uniquely
            // identifiable on read-back.
            locale: Some(format!("loc-{i}")),
            // Alternate unit_system across rows so we know we're
            // pulling the right one (not just the first / last).
            unit_system: Some(if i % 2 == 0 {
                UnitSystem::Metric
            } else {
                UnitSystem::Imperial
            }),
            theme: Some(if i % 3 == 0 {
                Theme::Dark
            } else {
                Theme::Light
            }),
            ..UserPrefsRow::default()
        };
        store
            .upsert_user_prefs("carol", ws, row)
            .await
            .expect("upsert per-workspace row");
    }

    for (i, ws) in workspaces.iter().enumerate() {
        let read = store
            .get_user_prefs("carol", ws)
            .await
            .expect("get per-workspace row")
            .expect("row exists");
        assert_eq!(read.locale.as_deref(), Some(format!("loc-{i}").as_str()));
        assert_eq!(
            read.unit_system,
            Some(if i % 2 == 0 {
                UnitSystem::Metric
            } else {
                UnitSystem::Imperial
            })
        );
        assert_eq!(
            read.theme,
            Some(if i % 3 == 0 {
                Theme::Dark
            } else {
                Theme::Light
            })
        );
    }

    // And a different user in the same workspace stays disjoint.
    assert!(store
        .get_user_prefs("dave", &workspaces[0])
        .await
        .unwrap()
        .is_none());
}
