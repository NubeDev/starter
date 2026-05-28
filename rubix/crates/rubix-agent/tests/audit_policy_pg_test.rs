//! Integration coverage for `PgAuditPolicyStore` — the
//! Postgres-backed [`AuditPolicyStore`] over the
//! `changelog_kind_policy` table.
//!
//! Spins an ephemeral Postgres, applies the upstream `changelog`
//! migration source (which provisions the table) and the rubix
//! `changelog_policy` seed source (which pins the audit-floor
//! rows for `user`, `team`, `tenant`), and exercises:
//!
//! 1. `list` returns the seeded floor rows in stable order, with
//!    `max_age_days = None` (the audit-floor invariant).
//! 2. `upsert` of a fresh kind round-trips and `get` echoes the
//!    row byte-exact.
//! 3. `upsert` with the same `(kind, max_age_days)` is a no-op:
//!    `updated_at_ms` does not advance even after a real clock
//!    tick (the trait contract verb bodies rely on to detect
//!    idempotency under \u{00A7}3.4 of the undo invariants).
//! 4. `upsert` with a changed `max_age_days` returns the prior
//!    row and a new row with a strictly later `updated_at_ms`.
//! 5. `put` bypasses idempotency and restores a snapshot
//!    verbatim, including its epoch-millisecond `updated_at_ms`
//!    (the undo path — \u{00A7}3.1 echo rule).
//! 6. `delete` is idempotent on missing rows; a delete-then-list
//!    round-trip removes only the targeted row.

use rubix_spi::audit::{AuditPolicyRow, AuditPolicyStore};
use rubix_store_postgres::{PgAuditPolicyStore, CHANGELOG_POLICY_MIGRATION_SOURCE};
use starter_changelog_postgres::migration_source as changelog_migration_source;
use starter_store_postgres::{migrate, testing::with_database};

#[tokio::test]
#[ignore = "requires Docker (testcontainers Postgres); run via the integration job"]
async fn audit_policy_round_trip_against_postgres() {
    let (pool, _guard) = with_database().await;
    // Upstream source provisions `changelog_kind_policy`; the
    // rubix-side source seeds the audit-floor rows on top.
    migrate(&pool)
        .with_source(changelog_migration_source())
        .with_source(CHANGELOG_POLICY_MIGRATION_SOURCE)
        .run()
        .await
        .expect("apply changelog + changelog_policy migrations");

    let store = PgAuditPolicyStore::new(pool.clone());

    // 1) Seeded floor rows: stable order, NULL curve.
    let seeded = store.list().await.expect("list seeded");
    let kinds: Vec<_> = seeded.iter().map(|r| r.resource_kind.as_str()).collect();
    assert_eq!(
        kinds,
        ["team", "tenant", "user"],
        "audit-floor seed rows in stable order"
    );
    for row in &seeded {
        assert!(
            row.max_age_days.is_none(),
            "audit-floor row {} must be pinned to forever",
            row.resource_kind
        );
    }

    // 2) Fresh upsert + get round-trip.
    let (prior, new) = store
        .upsert("flow_def", Some(30))
        .await
        .expect("upsert flow_def");
    assert!(prior.is_none(), "no prior row for fresh kind");
    assert_eq!(new.resource_kind, "flow_def");
    assert_eq!(new.max_age_days, Some(30));
    assert!(new.updated_at_ms > 0);

    let echoed = store
        .get("flow_def")
        .await
        .expect("get flow_def")
        .expect("flow_def present");
    assert_eq!(echoed, new, "get echoes upsert byte-exact");

    // 3) No-op upsert: same kind + same curve must not touch
    //    `updated_at_ms` even after a clock tick.
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    let (noop_prior, noop_new) = store
        .upsert("flow_def", Some(30))
        .await
        .expect("noop upsert");
    assert_eq!(noop_prior.as_ref(), Some(&new), "no-op echoes prior");
    assert_eq!(
        noop_new.updated_at_ms, new.updated_at_ms,
        "no-op preserves updated_at_ms (contract for verb idempotency)"
    );

    // 4) Changing the curve advances `updated_at_ms`.
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    let (changed_prior, changed_new) = store
        .upsert("flow_def", Some(60))
        .await
        .expect("change curve");
    assert_eq!(changed_prior.as_ref(), Some(&new));
    assert_eq!(changed_new.max_age_days, Some(60));
    assert!(
        changed_new.updated_at_ms > new.updated_at_ms,
        "real write advances updated_at_ms"
    );

    // 5) `put` restores a snapshot verbatim (undo path).
    let snapshot = AuditPolicyRow {
        resource_kind: "flow_def".to_string(),
        max_age_days: Some(30),
        updated_at_ms: new.updated_at_ms,
    };
    store.put(snapshot.clone()).await.expect("put snapshot");
    let restored = store
        .get("flow_def")
        .await
        .expect("get restored")
        .expect("flow_def present");
    assert_eq!(
        restored, snapshot,
        "put restores AuditPolicyRow byte-exact incl. updated_at_ms"
    );

    // 6) Delete is idempotent on missing rows; only the targeted
    //    row is removed from the surface.
    store
        .delete("not_a_real_kind")
        .await
        .expect("delete missing is idempotent");
    store.delete("flow_def").await.expect("delete flow_def");
    assert!(
        store
            .get("flow_def")
            .await
            .expect("get after delete")
            .is_none(),
        "flow_def gone after delete"
    );
    let final_kinds: Vec<_> = store
        .list()
        .await
        .expect("list after delete")
        .into_iter()
        .map(|r| r.resource_kind)
        .collect();
    assert_eq!(
        final_kinds,
        vec!["team", "tenant", "user"],
        "delete only removed flow_def; seeded floor rows intact"
    );
}
