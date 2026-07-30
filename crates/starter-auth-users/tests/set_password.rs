//! `admin::set_password` / `admin::change_password` against the real
//! sqlite store.
//!
//! Covers the outcomes HTTP callers branch on: the operator reset, the
//! self-serve rotation, the wrong-current-password rejection, the
//! OAuth-only `PasswordNotSet` case, weak-password validation, and the
//! "user not found" store wording that `PUT
//! /admin/users/{id}/password` matches on to return 404.

#![cfg(feature = "sqlite")]

use starter_auth_users::{
    admin::{self, AdminError, ChangePasswordError},
    password,
    role::Role,
    store::{SqliteUserStore, UserStore},
};
use starter_store_sqlite::{migrate, migrate::MigrationSource, testing::ephemeral};

static AUTH_USERS_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/starter_auth_users");
// The `password_hash` NOT NULL relaxation ships in the OAuth crate's
// migration set, not this one — see the note in `tests/flow.rs`. The
// OAuth-only-user cases below need it, so apply it inline the same way.
static AUTH_OAUTH_SQLITE_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("../starter-auth-oauth/migrations/starter_auth_oauth_sqlite");

/// Fresh in-memory store with the auth-users schema applied.
async fn store() -> SqliteUserStore {
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
        .unwrap();
    SqliteUserStore::new(pool)
}

/// Insert a user directly, bypassing `create_admin` so the test can
/// control whether a local password hash exists at all.
async fn seed(store: &SqliteUserStore, id: &str, email: &str, plaintext: Option<&str>) {
    let hash = plaintext.map(|p| password::hash(p).unwrap());
    store
        .create(id, email, hash.as_deref(), Role::Admin)
        .await
        .unwrap();
}

/// The stored hash verifies against `plaintext`.
async fn stored_password_is(store: &SqliteUserStore, id: &str, plaintext: &str) -> bool {
    let user = store.find_by_id(id).await.unwrap().unwrap();
    let hash = user.password_hash.expect("password hash should be set");
    password::verify(plaintext, &hash).unwrap()
}

#[tokio::test]
async fn set_password_replaces_the_hash() {
    let store = store().await;
    seed(&store, "u1", "a@example.com", Some("original-password-1")).await;

    admin::set_password(&store, "u1", "replacement-password-1")
        .await
        .unwrap();

    assert!(stored_password_is(&store, "u1", "replacement-password-1").await);
    assert!(!stored_password_is(&store, "u1", "original-password-1").await);
}

#[tokio::test]
async fn set_password_gives_an_oauth_only_user_a_local_password() {
    let store = store().await;
    // No hash: the GitHub-sign-in-only user. This is the case the
    // operator reset lever exists for.
    seed(&store, "u1", "a@example.com", None).await;

    admin::set_password(&store, "u1", "first-local-password-1")
        .await
        .unwrap();

    assert!(stored_password_is(&store, "u1", "first-local-password-1").await);
}

#[tokio::test]
async fn set_password_on_missing_user_says_user_not_found() {
    let store = store().await;

    let err = admin::set_password(&store, "nope", "some-password-12")
        .await
        .unwrap_err();

    // dev-pulse's `set_user_password` matches on this exact substring
    // to return 404 instead of 500. Changing the wording breaks that.
    match err {
        AdminError::Store(msg) => assert!(
            msg.contains("user not found"),
            "expected \"user not found\" in {msg:?}"
        ),
        other => panic!("expected Store, got {other:?}"),
    }
}

#[tokio::test]
async fn set_password_rejects_a_weak_password() {
    let store = store().await;
    seed(&store, "u1", "a@example.com", Some("original-password-1")).await;

    let err = admin::set_password(&store, "u1", "short")
        .await
        .unwrap_err();

    assert!(matches!(err, AdminError::Validation(_)), "got {err:?}");
    // The rejection must not have clobbered the existing credential.
    assert!(stored_password_is(&store, "u1", "original-password-1").await);
}

#[tokio::test]
async fn change_password_rotates_when_the_current_password_matches() {
    let store = store().await;
    seed(&store, "u1", "a@example.com", Some("original-password-1")).await;

    admin::change_password(&store, "u1", "original-password-1", "rotated-password-1")
        .await
        .unwrap();

    assert!(stored_password_is(&store, "u1", "rotated-password-1").await);
}

#[tokio::test]
async fn change_password_rejects_a_wrong_current_password() {
    let store = store().await;
    seed(&store, "u1", "a@example.com", Some("original-password-1")).await;

    let err = admin::change_password(&store, "u1", "wrong-password-99", "rotated-password-1")
        .await
        .unwrap_err();

    assert!(
        matches!(err, ChangePasswordError::WrongPassword),
        "got {err:?}"
    );
    assert!(stored_password_is(&store, "u1", "original-password-1").await);
}

#[tokio::test]
async fn change_password_reports_password_not_set_for_an_oauth_only_user() {
    let store = store().await;
    seed(&store, "u1", "a@example.com", None).await;

    let err = admin::change_password(&store, "u1", "anything-at-all-1", "rotated-password-1")
        .await
        .unwrap_err();

    // Must not read as WrongPassword: there is nothing to verify
    // against, and the remedy is an operator set_password, not a retry.
    assert!(
        matches!(err, ChangePasswordError::PasswordNotSet),
        "got {err:?}"
    );
}

#[tokio::test]
async fn change_password_reports_not_found_for_a_missing_user() {
    let store = store().await;

    let err = admin::change_password(&store, "nope", "whatever-pass-1", "rotated-password-1")
        .await
        .unwrap_err();

    assert!(matches!(err, ChangePasswordError::NotFound), "got {err:?}");
}

#[tokio::test]
async fn change_password_rejects_a_weak_new_password() {
    let store = store().await;
    seed(&store, "u1", "a@example.com", Some("original-password-1")).await;

    let err = admin::change_password(&store, "u1", "original-password-1", "short")
        .await
        .unwrap_err();

    assert!(
        matches!(err, ChangePasswordError::Validation(_)),
        "got {err:?}"
    );
    assert!(stored_password_is(&store, "u1", "original-password-1").await);
}
