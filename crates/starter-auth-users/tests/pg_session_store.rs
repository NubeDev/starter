//! Integration test for `PgSessionStore` — spins up a real Postgres
//! container via `starter-store-postgres::testing::with_database`,
//! applies the `auth_users` migration source, and exercises every
//! method on the `SessionStore` trait against the live DB.
//!
//! Marked `#[ignore]` by default: the testcontainers run requires
//! Docker on the host and is meant for the integration CI job.
//!
//! Run with:
//!
//! ```text
//! cargo test -p starter-auth-users --features postgres \
//!     --test pg_session_store -- --ignored
//! ```

#![cfg(feature = "postgres")]

use chrono::{Duration, Utc};
use starter_auth_users::migration::postgres_migration_source;
use starter_auth_users::role::Role;
use starter_auth_users::store::{PgSessionStore, PgUserStore, SessionStore, UserStore};
use starter_store_postgres::{migrate, testing::with_database};

const ARGON2_FIXTURE: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$Y2FmZWJhYmU$ZmFrZWZha2VmYWtlZmFrZWZha2VmYWtl";

#[tokio::test]
#[ignore = "requires Docker / testcontainers; run via the integration CI job"]
async fn pg_session_store_round_trips_every_method_against_live_postgres() {
    let (pool, _guard) = with_database().await;

    migrate(&pool)
        .with_source(postgres_migration_source())
        .run()
        .await
        .expect("apply auth_users postgres migrations");

    // Sessions FK user_id → users(id); seed a user first.
    let users = PgUserStore::new(pool.clone());
    users
        .create("user-1", "op@example.com", Some(ARGON2_FIXTURE), Role::Admin)
        .await
        .expect("seed user");

    let store = PgSessionStore::new(pool);

    let future = Utc::now() + Duration::hours(1);
    let past = Utc::now() - Duration::seconds(5);

    // create() — with a tenant binding.
    let s1 = store
        .create("sess-1", "user-1", "csrf-1", Some("tenant-a"), future)
        .await
        .expect("create session with tenant");
    assert_eq!(s1.id, "sess-1");
    assert_eq!(s1.tenant_id.as_deref(), Some("tenant-a"));
    assert!(s1.revoked_at.is_none());

    // create() — no tenant binding (pre-multi-tenant shape; allowed).
    store
        .create("sess-2", "user-1", "csrf-2", None, future)
        .await
        .expect("create session without tenant");

    // create() — already expired; find_active must skip it.
    store
        .create("sess-expired", "user-1", "csrf-x", None, past)
        .await
        .expect("create expired session");

    // find_active — present.
    let got = store
        .find_active("sess-1")
        .await
        .expect("find_active succeeds")
        .expect("sess-1 active");
    assert_eq!(got.user_id, "user-1");
    assert_eq!(got.csrf_token, "csrf-1");
    assert_eq!(got.tenant_id.as_deref(), Some("tenant-a"));

    // find_active — missing id.
    let miss = store
        .find_active("nope")
        .await
        .expect("find_active succeeds on miss");
    assert!(miss.is_none());

    // find_active — expired row is invisible.
    let exp = store
        .find_active("sess-expired")
        .await
        .expect("find_active succeeds on expired");
    assert!(exp.is_none(), "expired session must not be returned");

    // revoke — first call clears it.
    store.revoke("sess-1").await.expect("revoke sess-1");
    let after_revoke = store
        .find_active("sess-1")
        .await
        .expect("find_active succeeds post-revoke");
    assert!(after_revoke.is_none(), "revoked session must not be active");

    // revoke — idempotent on already-revoked.
    store
        .revoke("sess-1")
        .await
        .expect("revoke is idempotent on already-revoked");

    // revoke — idempotent on missing.
    store
        .revoke("ghost")
        .await
        .expect("revoke is idempotent on missing id");

    // sess-2 remains active and untouched.
    let s2 = store
        .find_active("sess-2")
        .await
        .expect("find_active succeeds for sess-2")
        .expect("sess-2 still active");
    assert!(s2.tenant_id.is_none());
}
