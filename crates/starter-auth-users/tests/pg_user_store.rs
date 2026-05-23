//! Integration test for `PgUserStore` — spins up a real Postgres
//! container via `starter-store-postgres::testing::with_database`,
//! applies the `auth_users` migration source, and exercises every
//! method on the `UserStore` trait against the live DB.
//!
//! Marked `#[ignore]` by default: the testcontainers run requires
//! Docker on the host and is meant for the integration CI job.
//!
//! Run with:
//!
//! ```text
//! cargo test -p starter-auth-users --features postgres \
//!     --test pg_user_store -- --ignored
//! ```

#![cfg(feature = "postgres")]

use starter_auth_users::migration::postgres_migration_source;
use starter_auth_users::role::Role;
use starter_auth_users::store::{PgUserStore, UserStore, UserStoreError};
use starter_store_postgres::{migrate, testing::with_database};

const ARGON2_FIXTURE: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$Y2FmZWJhYmU$ZmFrZWZha2VmYWtlZmFrZWZha2VmYWtl";

#[tokio::test]
#[ignore = "requires Docker / testcontainers; run via the integration CI job"]
async fn pg_user_store_round_trips_every_method_against_live_postgres() {
    let (pool, _guard) = with_database().await;

    migrate(&pool)
        .with_source(postgres_migration_source())
        .run()
        .await
        .expect("apply auth_users postgres migrations");

    let store = PgUserStore::new(pool);

    // create() — happy path, both with and without a password hash.
    store
        .create("user-1", "op@example.com", Some(ARGON2_FIXTURE), Role::Admin)
        .await
        .expect("create local-password admin");
    store
        .create("user-2", "oauth@example.com", None, Role::Reader)
        .await
        .expect("create third-party-only user");

    // create() — Conflict on duplicate email.
    let dup = store
        .create("user-3", "op@example.com", Some(ARGON2_FIXTURE), Role::Reader)
        .await;
    assert!(matches!(dup, Err(UserStoreError::Conflict)));

    // find_by_email — present + missing.
    let by_email = store
        .find_by_email("op@example.com")
        .await
        .expect("find_by_email succeeds")
        .expect("op exists");
    assert_eq!(by_email.id, "user-1");
    assert_eq!(by_email.role, Role::Admin);
    assert_eq!(by_email.password_hash.as_deref(), Some(ARGON2_FIXTURE));

    let miss = store
        .find_by_email("nobody@example.com")
        .await
        .expect("find_by_email succeeds on miss");
    assert!(miss.is_none());

    // find_by_id — present + missing.
    let by_id = store
        .find_by_id("user-2")
        .await
        .expect("find_by_id succeeds")
        .expect("oauth user exists");
    assert_eq!(by_id.email, "oauth@example.com");
    assert!(by_id.password_hash.is_none(), "oauth user has no local password");

    let miss = store
        .find_by_id("ghost")
        .await
        .expect("find_by_id succeeds on miss");
    assert!(miss.is_none());

    // set_email_verified — flips the bit; second flip is a no-op
    // (the column already defaults TRUE post-0004).
    store
        .set_email_verified("user-2", false)
        .await
        .expect("set_email_verified false");
    store
        .set_email_verified("user-2", true)
        .await
        .expect("set_email_verified true");
}
