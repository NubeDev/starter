//! Integration coverage for `PgTeamAdminStore` -- the
//! Postgres-backed [`TeamAdminStore`] over the `rubix_teams`
//! table.
//!
//! Spins an ephemeral Postgres, applies the rubix-teams
//! migration, then exercises:
//!
//! 1. Empty list on a fresh table.
//! 2. `create` round-trips a row (including an empty members
//!    map) and `get` echoes byte-exact.
//! 3. `create` rejects duplicate `name` with `Conflict` and the
//!    message names the offending name (matches the in-memory
//!    fake byte-exact).
//! 4. `assign` then a second `assign` keeps the original
//!    `assigned_at_ms` -- idempotency, the (prior, new) tuple
//!    matches byte-exact so the verb skips the audit row.
//! 5. `unassign` removes the member; a second `unassign` of
//!    the same user is a no-op.
//! 6. `unassign` / `assign` against a missing team return
//!    `NotFound` (the verb relies on this signal).
//! 7. `put` bypasses uniqueness and restores a snapshot
//!    verbatim including the members map (undo path).
//! 8. `delete` removes the row; a subsequent `delete` returns
//!    `NotFound` (matches the in-memory fake -- the verb
//!    distinguishes missing-target from a successful no-op).

use std::collections::BTreeMap;

use rubix_spi::starter::error::Error;
use rubix_spi::team::{TeamAdminStore, TeamRow};
use rubix_store_postgres::{PgTeamAdminStore, RUBIX_TEAMS_MIGRATION_SOURCE};
use starter_store_postgres::{migrate, testing::with_database};

fn row(id: &str, name: &str) -> TeamRow {
    TeamRow {
        team_id: id.into(),
        name: name.into(),
        description: None,
        members: BTreeMap::new(),
    }
}

#[tokio::test]
#[ignore = "requires Docker (testcontainers Postgres); run via the integration job"]
async fn team_store_round_trip_against_postgres() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(RUBIX_TEAMS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("apply rubix_teams migration");

    let store = PgTeamAdminStore::new(pool.clone());

    // 1) Empty on fresh boot.
    let initial = store.list().await.expect("list empty");
    assert!(initial.is_empty());

    // 2) create round-trip with empty members + description.
    let inserted = store.create(row("t-1", "Ops")).await.expect("create");
    assert_eq!(inserted, row("t-1", "Ops"));
    let got = store.get("t-1").await.expect("get").expect("present");
    assert_eq!(got, row("t-1", "Ops"));

    // 3) duplicate name -> Conflict.
    let err = store.create(row("t-2", "Ops")).await.expect_err("dup name");
    match err {
        Error::Conflict { message } => assert!(
            message.contains("Ops"),
            "conflict names the team: {message}"
        ),
        other => panic!("expected Conflict, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "requires Docker (testcontainers Postgres); run via the integration job"]
async fn assign_unassign_idempotency_preserves_assigned_at() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(RUBIX_TEAMS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("apply migration");

    let store = PgTeamAdminStore::new(pool.clone());
    store.create(row("t-1", "Ops")).await.expect("create");

    // First assign lands a real change.
    let (prior, new) = store.assign("t-1", "u-1", 100).await.expect("assign");
    assert!(prior.members.is_empty());
    assert_eq!(new.members.get("u-1"), Some(&100));

    // Second assign with a different now_ms KEEPS the original
    // 100 -- the (prior, new) tuple matches byte-exact so no
    // audit row is recorded.
    let (prior2, new2) = store.assign("t-1", "u-1", 999).await.expect("assign noop");
    assert_eq!(prior2.members.get("u-1"), Some(&100));
    assert_eq!(new2.members.get("u-1"), Some(&100));
    assert_eq!(prior2, new2, "no-op assign returns matching halves");

    // Unassign removes.
    let (prior3, new3) = store.unassign("t-1", "u-1").await.expect("unassign");
    assert_eq!(prior3.members.get("u-1"), Some(&100));
    assert!(new3.members.is_empty());

    // Second unassign is a no-op.
    let (prior4, new4) = store.unassign("t-1", "u-1").await.expect("unassign noop");
    assert!(prior4.members.is_empty());
    assert!(new4.members.is_empty());
    assert_eq!(prior4, new4);
}

#[tokio::test]
#[ignore = "requires Docker (testcontainers Postgres); run via the integration job"]
async fn missing_team_returns_not_found() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(RUBIX_TEAMS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("apply migration");

    let store = PgTeamAdminStore::new(pool.clone());

    let err = store
        .assign("missing", "u-1", 100)
        .await
        .expect_err("assign on missing team");
    assert!(matches!(err, Error::NotFound { .. }), "got {err:?}");

    let err = store
        .unassign("missing", "u-1")
        .await
        .expect_err("unassign on missing team");
    assert!(matches!(err, Error::NotFound { .. }), "got {err:?}");

    let err = store.delete("missing").await.expect_err("delete missing");
    assert!(matches!(err, Error::NotFound { .. }), "got {err:?}");
}

#[tokio::test]
#[ignore = "requires Docker (testcontainers Postgres); run via the integration job"]
async fn put_restores_snapshot_verbatim_with_members_map() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(RUBIX_TEAMS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("apply migration");

    let store = PgTeamAdminStore::new(pool.clone());
    store.create(row("t-1", "Ops")).await.expect("create");

    // Snapshot restore with description + populated members map.
    let mut members = BTreeMap::new();
    members.insert("u-1".to_owned(), 100);
    members.insert("u-2".to_owned(), 200);
    let snapshot = TeamRow {
        team_id: "t-1".into(),
        name: "Ops Renamed".into(),
        description: Some("ops team description".into()),
        members,
    };
    store.put(snapshot.clone()).await.expect("put snapshot");
    let after = store.get("t-1").await.expect("get").expect("present");
    assert_eq!(after, snapshot);

    // Delete then a redundant delete (undo of create followed by
    // a peer or replay) -- second one returns NotFound per the
    // trait contract.
    store.delete("t-1").await.expect("delete");
    let err = store.delete("t-1").await.expect_err("redundant delete");
    assert!(matches!(err, Error::NotFound { .. }), "got {err:?}");
}
