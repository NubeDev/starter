//! Integration coverage for `PgUserAdminStore` \u{2014} the
//! Postgres-backed [`UserAdminStore`] over the `rubix_users`
//! table.
//!
//! Spins an ephemeral Postgres, applies the rubix-tenants
//! migration (needed for the FK on `rubix_users.tenant_id`) and
//! the rubix-users migration, then exercises:
//!
//! 1. Empty list on a fresh table.
//! 2. `create` round-trips a row and `get` / `find_by_email`
//!    return it byte-exact.
//! 3. `create` rejects a duplicate email with `Conflict`.
//! 4. `disable` then a second `disable` keeps the original
//!    `disabled_at_ms` (idempotency \u{2014} \u{00A7}3.1 echo
//!    rule: the `(prior, new)` tuple matches on no-op).
//! 5. `enable` clears the marker and is idempotent on already-enabled.
//! 6. `set_role` / `set_prefs` / `set_tenant` no-op when the
//!    requested value already matches.
//! 7. `set_tenant` with a `tenant_id` that does not resolve in
//!    `rubix_tenants` returns `Conflict` via the FK.
//! 8. `put` bypasses email-uniqueness for the same `user_id`
//!    (undo restore path).
//! 9. `delete` is idempotent on missing rows.

use rubix_spi::starter::error::Error;
use rubix_spi::user::{UserAdminStore, UserRow};
use rubix_store_postgres::{
    PgUserAdminStore, RUBIX_TENANTS_MIGRATION_SOURCE, RUBIX_USERS_MIGRATION_SOURCE,
};
use serde_json::json;
use starter_store_postgres::{migrate, testing::with_database};

fn row(id: &str, email: &str) -> UserRow {
    UserRow {
        user_id: id.into(),
        email: email.into(),
        role: "reader".into(),
        disabled_at_ms: None,
        prefs_json: None,
        tenant_id: None,
    }
}

#[tokio::test]
#[ignore = "requires Docker (testcontainers Postgres); run via the integration job"]
async fn user_store_round_trip_against_postgres() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(RUBIX_TENANTS_MIGRATION_SOURCE)
        .with_source(RUBIX_USERS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("apply rubix_users migration");

    let store = PgUserAdminStore::new(pool.clone());

    // 1) Empty on fresh boot.
    let initial = store.list().await.expect("list empty");
    assert!(initial.is_empty(), "no users on a fresh table");

    // 2) create round-trip.
    let inserted = store.create(row("u-1", "a@x")).await.expect("create u-1");
    assert_eq!(inserted, row("u-1", "a@x"));
    let got = store.get("u-1").await.expect("get u-1").expect("present");
    assert_eq!(got, row("u-1", "a@x"));
    let by_email = store
        .find_by_email("a@x")
        .await
        .expect("find a@x")
        .expect("present");
    assert_eq!(by_email, row("u-1", "a@x"));

    // 3) duplicate email \u{2192} Conflict.
    let err = store
        .create(row("u-2", "a@x"))
        .await
        .expect_err("duplicate email rejected");
    match err {
        Error::Conflict { message } => {
            assert!(
                message.contains("a@x"),
                "conflict message names the email: {message}"
            );
        }
        other => panic!("expected Conflict, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "requires Docker (testcontainers Postgres); run via the integration job"]
async fn disable_enable_idempotency_preserves_prior_timestamp() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(RUBIX_TENANTS_MIGRATION_SOURCE)
        .with_source(RUBIX_USERS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("apply rubix_users migration");

    let store = PgUserAdminStore::new(pool.clone());
    store.create(row("u-1", "a@x")).await.expect("create");

    // First disable lands a real change.
    let (prior, new) = store.disable("u-1", 100).await.expect("disable @100");
    assert!(prior.disabled_at_ms.is_none());
    assert_eq!(new.disabled_at_ms, Some(100));

    // Second disable is a no-op and KEEPS the original timestamp
    // \u{2014} the (prior, new) tuple matches and no audit row is
    // recorded.
    let (prior2, new2) = store.disable("u-1", 999).await.expect("disable @999");
    assert_eq!(prior2.disabled_at_ms, Some(100));
    assert_eq!(new2.disabled_at_ms, Some(100));

    // Enable clears it.
    let (prior3, new3) = store.enable("u-1").await.expect("enable");
    assert_eq!(prior3.disabled_at_ms, Some(100));
    assert!(new3.disabled_at_ms.is_none());

    // Second enable is a no-op.
    let (prior4, new4) = store.enable("u-1").await.expect("enable idempotent");
    assert!(prior4.disabled_at_ms.is_none());
    assert!(new4.disabled_at_ms.is_none());
}

#[tokio::test]
#[ignore = "requires Docker (testcontainers Postgres); run via the integration job"]
async fn set_role_prefs_tenant_idempotency() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(RUBIX_TENANTS_MIGRATION_SOURCE)
        .with_source(RUBIX_USERS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("apply rubix_users migration");

    let store = PgUserAdminStore::new(pool.clone());
    store.create(row("u-1", "a@x")).await.expect("create");

    // set_role \u{2014} real change then no-op.
    let (prior, new) = store
        .set_role("u-1", "writer")
        .await
        .expect("set_role writer");
    assert_eq!(prior.role, "reader");
    assert_eq!(new.role, "writer");
    let (prior2, new2) = store
        .set_role("u-1", "writer")
        .await
        .expect("set_role no-op");
    assert_eq!(prior2.role, "writer");
    assert_eq!(new2.role, "writer");

    // set_prefs \u{2014} real change then no-op.
    let prefs = json!({ "locale": "en-US" });
    let (prior, new) = store
        .set_prefs("u-1", prefs.clone())
        .await
        .expect("set_prefs");
    assert!(prior.prefs_json.is_none());
    assert_eq!(new.prefs_json, Some(prefs.clone()));
    let (prior2, new2) = store
        .set_prefs("u-1", prefs.clone())
        .await
        .expect("set_prefs no-op");
    assert_eq!(prior2.prefs_json, Some(prefs.clone()));
    assert_eq!(new2.prefs_json, Some(prefs));

    // set_tenant \u{2014} valid id assigns, then no-op.
    // Use the bundled `"system"` seed (present courtesy of the
    // rubix_tenants migration that includes the seed).
    let (prior, new) = store
        .set_tenant("u-1", Some("system".into()))
        .await
        .expect("assign tenant");
    assert!(prior.tenant_id.is_none());
    assert_eq!(new.tenant_id, Some("system".into()));
    let (prior2, new2) = store
        .set_tenant("u-1", Some("system".into()))
        .await
        .expect("assign no-op");
    assert_eq!(prior2.tenant_id, Some("system".into()));
    assert_eq!(new2.tenant_id, Some("system".into()));
}

#[tokio::test]
#[ignore = "requires Docker (testcontainers Postgres); run via the integration job"]
async fn set_tenant_with_missing_tenant_is_conflict() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(RUBIX_TENANTS_MIGRATION_SOURCE)
        .with_source(RUBIX_USERS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("apply migrations");

    let store = PgUserAdminStore::new(pool.clone());
    store.create(row("u-1", "a@x")).await.expect("create");

    // The verb pre-check normally catches this; the store stays
    // defensive so a direct-call regression or undo replay
    // against a stale snapshot surfaces a clean Conflict.
    let err = store
        .set_tenant("u-1", Some("does-not-exist".into()))
        .await
        .expect_err("FK violation");
    assert!(matches!(err, Error::Conflict { .. }), "got {err:?}");
}

#[tokio::test]
#[ignore = "requires Docker (testcontainers Postgres); run via the integration job"]
async fn put_bypasses_idempotency_and_delete_is_idempotent() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(RUBIX_TENANTS_MIGRATION_SOURCE)
        .with_source(RUBIX_USERS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("apply migrations");

    let store = PgUserAdminStore::new(pool.clone());
    store.create(row("u-1", "a@x")).await.expect("create");

    // put overwrites verbatim \u{2014} undo snapshot restore.
    let snapshot = UserRow {
        user_id: "u-1".into(),
        email: "a@x".into(),
        role: "admin".into(),
        disabled_at_ms: Some(42),
        prefs_json: Some(json!({"theme":"dark"})),
        tenant_id: Some("system".into()),
    };
    store.put(snapshot.clone()).await.expect("put snapshot");
    let after = store.get("u-1").await.expect("get").expect("present");
    assert_eq!(after, snapshot);

    // delete is idempotent on missing rows (undo of create then
    // a redundant undo).
    store.delete("u-1").await.expect("delete");
    store
        .delete("u-1")
        .await
        .expect("delete on missing is a no-op");
}
