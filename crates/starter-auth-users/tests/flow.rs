//! End-to-end coverage of the cookie session + API token flow
//! against an in-memory sqlite database.

#![cfg(feature = "sqlite")]

use std::sync::Arc;

use starter_auth_users::{
    admin::{create_admin, AdminError},
    store::{SqliteSessionStore, SqliteTokenStore, SqliteUserStore, UserStore},
    token, AuthAuthenticator, Role,
};
use starter_spi::auth::Authenticator;
use starter_store_sqlite::{migrate, migrate::MigrationSource, testing::ephemeral, Pool};

static AUTH_USERS_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/starter_auth_users");
// The `password_hash` NOT NULL relaxation ships in the OAuth crate's
// migration set, not this one (Hard rule R8 + SCOPE Constraints). The
// tests in this file rely on it so they apply it inline; consumers
// who do not enable OAuth will never see it.
static AUTH_OAUTH_SQLITE_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("../starter-auth-oauth/migrations/starter_auth_oauth_sqlite");

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

fn stores(
    pool: Pool,
) -> (
    Arc<SqliteUserStore>,
    Arc<SqliteSessionStore>,
    Arc<SqliteTokenStore>,
) {
    (
        Arc::new(SqliteUserStore::new(pool.clone())),
        Arc::new(SqliteSessionStore::new(pool.clone())),
        Arc::new(SqliteTokenStore::new(pool)),
    )
}

#[tokio::test]
async fn create_admin_then_login_through_session() {
    let pool = fresh_pool().await;
    let (users, sessions, _tokens) = stores(pool);

    let id = create_admin(users.as_ref(), "a@example.com", "hunter22hunter", Role::Admin)
        .await
        .expect("create admin");
    assert!(!id.is_empty());

    // Look up the user record so we have something to issue a session for.
    let user = users.find_by_email("a@example.com").await.unwrap().unwrap();
    let issued = starter_auth_users::session::issue(sessions.as_ref(), &user.id)
        .await
        .expect("issue session");
    assert!(issued.cookie_value.starts_with("sas_"));

    let principal = starter_auth_users::session::verify_session(
        sessions.as_ref(),
        users.as_ref(),
        &issued.cookie_value,
    )
    .await
    .expect("verify session");
    assert_eq!(principal.role, Role::Admin);
    assert_eq!(principal.subject, user.id);
}

#[tokio::test]
async fn duplicate_admin_email_conflict() {
    let pool = fresh_pool().await;
    let (users, _sessions, _tokens) = stores(pool);

    create_admin(users.as_ref(), "dup@example.com", "password-one1", Role::Admin)
        .await
        .unwrap();
    let err = create_admin(users.as_ref(), "dup@example.com", "password-two2", Role::Admin)
        .await
        .unwrap_err();
    assert!(matches!(err, AdminError::Conflict));
}

#[tokio::test]
async fn revoked_session_no_longer_verifies() {
    let pool = fresh_pool().await;
    let (users, sessions, _tokens) = stores(pool);

    create_admin(users.as_ref(), "r@example.com", "long-enough-pw", Role::Writer)
        .await
        .unwrap();
    let user = users.find_by_email("r@example.com").await.unwrap().unwrap();
    let issued = starter_auth_users::session::issue(sessions.as_ref(), &user.id)
        .await
        .unwrap();

    starter_auth_users::session::revoke(sessions.as_ref(), &issued.cookie_value)
        .await
        .unwrap();

    let err = starter_auth_users::session::verify_session(
        sessions.as_ref(),
        users.as_ref(),
        &issued.cookie_value,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        err,
        starter_auth_users::session::SessionError::NotFound
    ));
}

#[tokio::test]
async fn api_token_round_trip() {
    let pool = fresh_pool().await;
    let (users, _sessions, tokens) = stores(pool);

    create_admin(users.as_ref(), "t@example.com", "long-enough-pw", Role::Reader)
        .await
        .unwrap();
    let user = users.find_by_email("t@example.com").await.unwrap().unwrap();

    let issued = token::issue(tokens.as_ref(), &user.id, &[], None)
        .await
        .expect("issue token");
    assert!(issued.plaintext.starts_with("sak_"));

    let principal = token::verify(tokens.as_ref(), users.as_ref(), &issued.plaintext)
        .await
        .expect("verify token");
    assert_eq!(principal.subject, user.id);
    assert_eq!(principal.role, Role::Reader);

    token::revoke(tokens.as_ref(), &issued.id).await.unwrap();
    let err = token::verify(tokens.as_ref(), users.as_ref(), &issued.plaintext)
        .await
        .unwrap_err();
    assert!(matches!(err, token::TokenError::Revoked));
}

#[tokio::test]
async fn wrong_secret_after_correct_prefix_is_invalid() {
    let pool = fresh_pool().await;
    let (users, _sessions, tokens) = stores(pool);
    create_admin(users.as_ref(), "w@example.com", "long-enough-pw", Role::Reader)
        .await
        .unwrap();
    let user = users.find_by_email("w@example.com").await.unwrap().unwrap();
    let issued = token::issue(tokens.as_ref(), &user.id, &[], None)
        .await
        .unwrap();

    // Replace the secret half with wrong bytes.
    let (prefix_and_id, _secret) = issued.plaintext.split_once('.').unwrap();
    let tampered = format!("{prefix_and_id}.wrongsecretvalue");
    let err = token::verify(tokens.as_ref(), users.as_ref(), &tampered)
        .await
        .unwrap_err();
    assert!(matches!(err, token::TokenError::Invalid));
}

#[tokio::test]
async fn user_with_null_password_hash_round_trips() {
    // OAuth-created users have `password_hash IS NULL`. The store
    // must accept `None` on create and round-trip it as `None`.
    let pool = fresh_pool().await;
    let (users, _sessions, _tokens) = stores(pool);
    users
        .create("uid-oauth", "oauth@example.com", None, Role::Reader)
        .await
        .expect("create with NULL hash");
    let row = users
        .find_by_email("oauth@example.com")
        .await
        .unwrap()
        .unwrap();
    assert!(row.password_hash.is_none());
    assert_eq!(row.id, "uid-oauth");
}

#[tokio::test]
async fn authenticator_dispatches_by_prefix() {
    let pool = fresh_pool().await;
    let (users, sessions, tokens) = stores(pool);
    create_admin(users.as_ref(), "d@example.com", "long-enough-pw", Role::Admin)
        .await
        .unwrap();
    let user = users.find_by_email("d@example.com").await.unwrap().unwrap();
    let session = starter_auth_users::session::issue(sessions.as_ref(), &user.id)
        .await
        .unwrap();
    let token = token::issue(tokens.as_ref(), &user.id, &[], None)
        .await
        .unwrap();

    let auth = AuthAuthenticator::new(
        users.clone() as _,
        sessions.clone() as _,
        tokens.clone() as _,
    );

    // Session credential.
    let p = auth.verify(&session.cookie_value).await.unwrap();
    assert_eq!(p.subject, user.id);

    // Token credential.
    let p = auth.verify(&token.plaintext).await.unwrap();
    assert_eq!(p.subject, user.id);

    // Garbage credential — no DB hit, immediate Unauthenticated.
    let err = auth.verify("garbage").await.unwrap_err();
    assert!(matches!(err, starter_spi::Error::Unauthenticated));
}
