//! Integration coverage for [`SqliteIdentityStore`] +
//! [`OAuthLinkedProviders`] against the real migration set. Runs
//! the users-crate migrations first, then this crate's
//! `0001_oauth_identities.sql` + `0002_users_password_optional.sql`
//! in order, and verifies the FK + ordering invariants the
//! `LinkedProvidersLookup` impl relies on.

#![cfg(feature = "sqlite")]

use std::sync::Arc;

use chrono::Utc;
use starter_auth_oauth::{IdentityStore, OAuthIdentity, OAuthLinkedProviders, SqliteIdentityStore};
use starter_auth_users::store::SqliteUserStore;
use starter_auth_users::{store::UserStore, LinkedProvidersLookup, Role};
use starter_store_sqlite::{migrate, migrate::MigrationSource, testing::ephemeral, Pool};

static AUTH_USERS_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("../starter-auth-users/migrations/starter_auth_users");
static AUTH_OAUTH_SQLITE_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/starter_auth_oauth_sqlite");

async fn fresh_pool() -> Pool {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(MigrationSource {
            name: "starter_auth_users",
            migrator: &AUTH_USERS_MIGRATOR,
        })
        .with_source(MigrationSource {
            name: "starter_auth_oauth",
            migrator: &AUTH_OAUTH_SQLITE_MIGRATOR,
        })
        .run()
        .await
        .expect("migrations apply");
    pool
}

#[tokio::test]
async fn linked_providers_lists_distinct_providers_in_linked_at_order() {
    let pool = fresh_pool().await;
    let users = SqliteUserStore::new(pool.clone());
    users
        .create("u1", "a@example.com", None, Role::Reader)
        .await
        .expect("create user");

    let store = Arc::new(SqliteIdentityStore::new(pool));
    let earlier = Utc::now() - chrono::Duration::minutes(5);
    let later = Utc::now();

    store
        .insert(&OAuthIdentity {
            provider: "github".to_string(),
            provider_sub: "gh-42".to_string(),
            user_id: "u1".to_string(),
            email: Some("a@example.com".to_string()),
            display_name: Some("Ada".to_string()),
            linked_at: earlier,
        })
        .await
        .expect("insert github identity");
    store
        .insert(&OAuthIdentity {
            provider: "google".to_string(),
            provider_sub: "g-99".to_string(),
            user_id: "u1".to_string(),
            email: Some("a@example.com".to_string()),
            display_name: None,
            linked_at: later,
        })
        .await
        .expect("insert google identity");

    // Direct list_for_user round-trip.
    let rows = store.list_for_user("u1").await.expect("list");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].provider, "github", "oldest first");
    assert_eq!(rows[1].provider, "google");

    // LinkedProvidersLookup impl: same order, de-duplicated to
    // *provider ids* (not identities) per its trait contract.
    let lookup = OAuthLinkedProviders::new(store.clone());
    let providers = lookup
        .linked_providers("u1")
        .await
        .expect("linked providers");
    assert_eq!(providers, vec!["github".to_string(), "google".to_string()]);

    // Empty for an unknown user.
    let none = lookup.linked_providers("u2").await.expect("unknown user");
    assert!(none.is_empty());
}

#[tokio::test]
async fn insert_then_find_then_delete_is_idempotent() {
    let pool = fresh_pool().await;
    let users = SqliteUserStore::new(pool.clone());
    users
        .create("u1", "b@example.com", None, Role::Reader)
        .await
        .expect("create user");
    let store = SqliteIdentityStore::new(pool);

    let id = OAuthIdentity {
        provider: "github".to_string(),
        provider_sub: "gh-7".to_string(),
        user_id: "u1".to_string(),
        email: None,
        display_name: None,
        linked_at: Utc::now(),
    };
    store.insert(&id).await.expect("insert");

    let got = store
        .find("github", "gh-7")
        .await
        .expect("find")
        .expect("present");
    assert_eq!(got.user_id, "u1");

    // Composite key collision = Conflict.
    let err = store.insert(&id).await.unwrap_err();
    assert!(matches!(
        err,
        starter_auth_oauth::IdentityStoreError::Conflict
    ));

    store.delete("github", "gh-7").await.expect("delete");
    // Deleting twice is fine.
    store.delete("github", "gh-7").await.expect("delete again");
    assert!(store.find("github", "gh-7").await.unwrap().is_none());
}
