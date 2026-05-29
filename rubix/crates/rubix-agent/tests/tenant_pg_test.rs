//! Integration coverage for `PgRubixTenantStore` \u{2014} the
//! Postgres-backed [`TenantStore`] over the `rubix_tenants`
//! table.
//!
//! Spins an ephemeral Postgres, applies
//! [`RUBIX_TENANTS_MIGRATION_SOURCE`] (which provisions the
//! table AND seeds the bundled `"system"` tenant), and
//! exercises:
//!
//! 1. The seed row is visible: `list` returns `[system]` after
//!    migration only.
//! 2. `create` round-trips a fresh row and `get` echoes byte-exact.
//! 3. `create` rejects duplicate `tenant_id` with `Conflict`
//!    (PRIMARY KEY) and the message names the offending id.
//! 4. `create` rejects duplicate `name` with `Conflict`
//!    (UNIQUE constraint) and the message names the offending
//!    name.
//! 5. `put` bypasses uniqueness and restores a snapshot
//!    verbatim (undo path \u{2014} \u{00A7}3.1 echo rule).
//! 6. `delete` removes the row; a subsequent `delete` on the
//!    same id returns `NotFound`.

use rubix_spi::starter::error::Error;
use rubix_spi::tenant::{TenantRow, TenantStore};
use rubix_store_postgres::{PgRubixTenantStore, RUBIX_TENANTS_MIGRATION_SOURCE};
use starter_store_postgres::{migrate, testing::with_database};

fn row(id: &str, name: &str, locale: &str) -> TenantRow {
    TenantRow {
        tenant_id: id.into(),
        name: name.into(),
        locale: locale.into(),
    }
}

#[tokio::test]
#[ignore = "requires Docker (testcontainers Postgres); run via the integration job"]
async fn tenant_store_round_trip_against_postgres() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(RUBIX_TENANTS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("apply rubix_tenants migration");

    let store = PgRubixTenantStore::new(pool.clone());

    // 1) Seed row visible.
    let initial = store.list().await.expect("list initial");
    assert_eq!(initial.len(), 1, "only the seeded system tenant present");
    assert_eq!(initial[0].tenant_id, "system");
    assert_eq!(initial[0].name, "System");
    assert_eq!(initial[0].locale, "en");

    // 2) create round-trip.
    let inserted = store
        .create(row("acme", "Acme", "en"))
        .await
        .expect("create acme");
    assert_eq!(inserted, row("acme", "Acme", "en"));
    let echoed = store.get("acme").await.expect("get acme").expect("present");
    assert_eq!(echoed, inserted);

    // 3) Duplicate id rejected with id-bearing message.
    let dup_id = store
        .create(row("acme", "Different Name", "en"))
        .await
        .unwrap_err();
    match dup_id {
        Error::Conflict { message } => {
            assert!(
                message.contains("acme"),
                "duplicate-id message names the id; got: {message}"
            );
        }
        other => panic!("expected Conflict, got {other:?}"),
    }

    // 4) Duplicate name rejected with name-bearing message.
    let dup_name = store.create(row("other", "Acme", "en")).await.unwrap_err();
    match dup_name {
        Error::Conflict { message } => {
            assert!(
                message.contains("Acme"),
                "duplicate-name message names the name; got: {message}"
            );
        }
        other => panic!("expected Conflict, got {other:?}"),
    }

    // 5) `put` bypasses uniqueness and restores the snapshot.
    let snapshot = row("acme", "Acme Inc", "es");
    store.put(snapshot.clone()).await.expect("put snapshot");
    let restored = store
        .get("acme")
        .await
        .expect("get restored")
        .expect("present");
    assert_eq!(
        restored, snapshot,
        "put restores TenantRow byte-exact (locale + name)"
    );

    // 6) delete removes; second delete is NotFound.
    store.delete("acme").await.expect("delete acme");
    assert!(
        store.get("acme").await.expect("get after delete").is_none(),
        "acme gone after delete"
    );
    let missing = store.delete("acme").await.unwrap_err();
    assert!(
        matches!(missing, Error::NotFound { .. }),
        "second delete returns NotFound; got {missing:?}"
    );

    // Final state: only the seed survives.
    let final_rows = store.list().await.expect("list final");
    let ids: Vec<_> = final_rows.iter().map(|r| r.tenant_id.as_str()).collect();
    assert_eq!(ids, ["system"], "delete cleaned up only acme");
}
