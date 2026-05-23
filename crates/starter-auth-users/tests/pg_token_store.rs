//! Integration test for `PgTokenStore` — spins up a real Postgres
//! container via `starter-store-postgres::testing::with_database`,
//! applies the `auth_users` migration source, and exercises every
//! method on the `TokenStore` trait against the live DB.
//!
//! Also asserts the `scopes` column type is `jsonb` — locking the
//! migration's intent (see docs/design/auth/README.md). The Rust
//! seam keeps `scopes` as a JSON-encoded `String`; the SQL boundary
//! casts to/from `jsonb`.
//!
//! Marked `#[ignore]` by default: the testcontainers run requires
//! Docker on the host and is meant for the integration CI job.
//!
//! Run with:
//!
//! ```text
//! cargo test -p starter-auth-users --features postgres \
//!     --test pg_token_store -- --ignored
//! ```

#![cfg(feature = "postgres")]

use chrono::{Duration, Utc};
use sqlx::Row;
use starter_auth_users::migration::postgres_migration_source;
use starter_auth_users::role::Role;
use starter_auth_users::scope::Scope;
use starter_auth_users::store::{PgTokenStore, PgUserStore, TokenStore, UserStore};
use starter_store_postgres::{migrate, testing::with_database};

const ARGON2_FIXTURE: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$Y2FmZWJhYmU$ZmFrZWZha2VmYWtlZmFrZWZha2VmYWtl";

#[tokio::test]
#[ignore = "requires Docker / testcontainers; run via the integration CI job"]
async fn pg_token_store_round_trips_every_method_against_live_postgres() {
    let (pool, _guard) = with_database().await;

    migrate(&pool)
        .with_source(postgres_migration_source())
        .run()
        .await
        .expect("apply auth_users postgres migrations");

    // Lock the migration's intent: `scopes` is JSONB, not TEXT.
    let scopes_type: String = sqlx::query(
        "SELECT data_type FROM information_schema.columns \
         WHERE table_name = 'starter_auth_users_tokens' AND column_name = 'scopes'",
    )
    .fetch_one(pool.sqlx())
    .await
    .expect("read scopes column type")
    .get(0);
    assert_eq!(
        scopes_type, "jsonb",
        "scopes column must be jsonb (per migration 0003)"
    );

    // Tokens FK user_id → users(id); seed a user first.
    let users = PgUserStore::new(pool.clone());
    users
        .create("user-1", "op@example.com", Some(ARGON2_FIXTURE), Role::Admin)
        .await
        .expect("seed user");

    let store = PgTokenStore::new(pool);

    let future = Utc::now() + Duration::hours(1);
    let past = Utc::now() - Duration::seconds(5);
    let scopes_a = vec![Scope::new("system.read"), Scope::new("system.alert")];
    let scopes_empty: Vec<Scope> = vec![];

    // create() — with scopes + expiry + tenant binding.
    store
        .create(
            "tok-1",
            "user-1",
            ARGON2_FIXTURE,
            &scopes_a,
            "tenant-a",
            Some(future),
        )
        .await
        .expect("create tok-1");

    // create() — no expiry, empty scopes, super-admin sentinel tenant.
    store
        .create("tok-2", "user-1", ARGON2_FIXTURE, &scopes_empty, "*", None)
        .await
        .expect("create tok-2");

    // create() — already expired; find_active must skip it.
    store
        .create(
            "tok-expired",
            "user-1",
            ARGON2_FIXTURE,
            &scopes_a,
            "tenant-a",
            Some(past),
        )
        .await
        .expect("create expired tok");

    // find_active — present, scopes round-trip, tenant carried through.
    let got = store
        .find_active("tok-1")
        .await
        .expect("find_active succeeds")
        .expect("tok-1 active");
    assert_eq!(got.user_id, "user-1");
    assert_eq!(got.tenant_id, "tenant-a");
    let scope_strs: Vec<&str> = got.scopes.iter().map(Scope::as_str).collect();
    assert_eq!(scope_strs, vec!["system.read", "system.alert"]);
    assert!(got.revoked_at.is_none());
    assert!(got.expires_at.is_some());

    // find_active — no-expiry token is active forever.
    let t2 = store
        .find_active("tok-2")
        .await
        .expect("find_active succeeds")
        .expect("tok-2 active");
    assert_eq!(t2.tenant_id, "*");
    assert!(t2.scopes.is_empty());
    assert!(t2.expires_at.is_none());

    // find_active — missing id.
    assert!(store
        .find_active("nope")
        .await
        .expect("find_active succeeds on miss")
        .is_none());

    // find_active — expired row is invisible.
    assert!(
        store
            .find_active("tok-expired")
            .await
            .expect("find_active succeeds on expired")
            .is_none(),
        "expired token must not be returned"
    );

    // touch_last_used — best-effort; must succeed even when the
    // row is missing (the auth path only logs on failure).
    store
        .touch_last_used("tok-1")
        .await
        .expect("touch_last_used existing");
    store
        .touch_last_used("ghost")
        .await
        .expect("touch_last_used missing must not error");

    // revoke — first call clears it.
    store.revoke("tok-1").await.expect("revoke tok-1");
    assert!(
        store
            .find_active("tok-1")
            .await
            .expect("find_active post-revoke")
            .is_none(),
        "revoked token must not be active"
    );

    // revoke — idempotent on already-revoked and missing.
    store
        .revoke("tok-1")
        .await
        .expect("revoke idempotent on already-revoked");
    store
        .revoke("ghost")
        .await
        .expect("revoke idempotent on missing id");

    // revoke_for_membership — seed a few more rows to exercise the
    // (user_id, tenant_id) filter.
    store
        .create(
            "tok-3",
            "user-1",
            ARGON2_FIXTURE,
            &scopes_a,
            "tenant-b",
            Some(future),
        )
        .await
        .expect("create tok-3");
    store
        .create(
            "tok-4",
            "user-1",
            ARGON2_FIXTURE,
            &scopes_a,
            "tenant-b",
            Some(future),
        )
        .await
        .expect("create tok-4");

    let revoked = store
        .revoke_for_membership("user-1", "tenant-b")
        .await
        .expect("revoke_for_membership tenant-b");
    assert_eq!(revoked, 2, "both tenant-b tokens revoked");

    // Idempotent: second call affects zero rows.
    let revoked_again = store
        .revoke_for_membership("user-1", "tenant-b")
        .await
        .expect("revoke_for_membership idempotent");
    assert_eq!(revoked_again, 0);

    // tok-2 (tenant `*`) is untouched.
    assert!(
        store
            .find_active("tok-2")
            .await
            .expect("find_active tok-2")
            .is_some(),
        "tenant `*` token survives tenant-b membership revoke"
    );
}
