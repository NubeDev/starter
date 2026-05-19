//! Smoke test: sqlite-rebuild-preserves-FKs.
//!
//! The highest-risk migration in this crate is
//! `0002_users_password_optional.sql` — the SQLite "12-step rebuild"
//! that relaxes NOT NULL on `starter_auth_users_users.password_hash`.
//! It runs with `PRAGMA foreign_keys = OFF` while it drops + renames
//! the live users table, then runs `PRAGMA foreign_key_check` before
//! `COMMIT` as the safety net.
//!
//! We replay it here against a fixture that mimics the **production
//! shape**: real users with
//!
//! - existing password hashes (the rebuild must carry them through),
//! - active sessions (FK → users via `ON DELETE CASCADE`),
//! - API tokens (FK → users via `ON DELETE CASCADE`),
//! - linked OAuth identities (FK → users via `ON DELETE CASCADE`).
//!
//! Post-rebuild we assert:
//!
//! 1. `PRAGMA foreign_key_check` returns no rows (the rebuild left
//!    the database FK-consistent),
//! 2. every fixture row still resolves through its FK (the parent
//!    user is still reachable; sessions / tokens / identities still
//!    join),
//! 3. the NOT NULL relaxation works — inserting an OAuth-only user
//!    with `password_hash = NULL` succeeds, where the same insert
//!    would have failed against the 0001 schema,
//! 4. `ON DELETE CASCADE` still fires through the renamed-into-place
//!    table — deleting a user removes their sessions, tokens, and
//!    OAuth identities atomically.
//!
//! This test sidesteps the project's migrator chain because it needs
//! to interleave fixture inserts *between* OAuth migration 0001 and
//! OAuth migration 0002. The `Migrate` runner applies every pending
//! migration in one shot, so we drive raw SQL via
//! [`sqlx::Executor::execute`] against a single-connection pool with
//! `foreign_keys = ON` from the first connection onward.

#![cfg(feature = "sqlite")]

use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow};
use sqlx::{Executor, Row, SqlitePool};

const USERS_0001: &str =
    include_str!("../../starter-auth-users/migrations/starter_auth_users/0001_users.sql");
const USERS_0002: &str =
    include_str!("../../starter-auth-users/migrations/starter_auth_users/0002_sessions.sql");
const USERS_0003: &str =
    include_str!("../../starter-auth-users/migrations/starter_auth_users/0003_tokens.sql");
const OAUTH_0001: &str =
    include_str!("../migrations/starter_auth_oauth_sqlite/0001_oauth_identities.sql");
const OAUTH_0002: &str =
    include_str!("../migrations/starter_auth_oauth_sqlite/0002_users_password_optional.sql");

async fn fresh_pool() -> SqlitePool {
    // Pin the pool to a single connection so the rebuild's
    // PRAGMA foreign_keys = OFF / ON dance applies to the same
    // connection state we later assert against. `foreign_keys(true)`
    // also wires the pragma into the initial connect handshake so
    // FK enforcement is on before any fixture row goes in.
    let opts = SqliteConnectOptions::from_str("sqlite::memory:")
        .expect("parse url")
        .foreign_keys(true);
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .expect("connect")
}

async fn count(pool: &SqlitePool, sql: &str) -> i64 {
    sqlx::query_scalar(sql)
        .fetch_one(pool)
        .await
        .expect("count")
}

#[tokio::test]
async fn rebuild_against_production_shape_fixture_keeps_every_foreign_key_intact() {
    let pool = fresh_pool().await;

    // ---- pre-0002 schema (the live one before this crate ships) ----
    pool.execute(USERS_0001).await.expect("users 0001");
    pool.execute(USERS_0002).await.expect("users 0002");
    pool.execute(USERS_0003).await.expect("users 0003");
    pool.execute(OAUTH_0001).await.expect("oauth 0001");

    // ---- fixture (production-shape) -----------------------------
    // Two users (`u-ada` and `u-grace`). Both have:
    //   - a password hash (the current schema is NOT NULL),
    //   - one active session,
    //   - one API token,
    //   - one OAuth identity row pointing at them.
    // A third row (`u-orphan-precheck`) has a hash but no
    // dependents — proves the rebuild does not silently drop
    // dependency-free users.
    for (id, email) in [
        ("u-ada", "ada@example.com"),
        ("u-grace", "grace@example.com"),
        ("u-orphan-precheck", "lone@example.com"),
    ] {
        sqlx::query(
            "INSERT INTO starter_auth_users_users \
             (id, email, password_hash, role) \
             VALUES (?1, ?2, ?3, 'reader')",
        )
        .bind(id)
        .bind(email)
        .bind(format!("$argon2id$v=19$dummy-for-{id}"))
        .execute(&pool)
        .await
        .expect("insert user");
    }

    for (sid, uid) in [("sess-ada", "u-ada"), ("sess-grace", "u-grace")] {
        sqlx::query(
            "INSERT INTO starter_auth_users_sessions \
             (id, user_id, csrf_token, expires_at) \
             VALUES (?1, ?2, 'csrf', '2099-01-01T00:00:00Z')",
        )
        .bind(sid)
        .bind(uid)
        .execute(&pool)
        .await
        .expect("insert session");
    }

    for (tid, uid) in [("tok-ada", "u-ada"), ("tok-grace", "u-grace")] {
        sqlx::query(
            "INSERT INTO starter_auth_users_tokens \
             (id, user_id, hashed_token) VALUES (?1, ?2, 'hash')",
        )
        .bind(tid)
        .bind(uid)
        .execute(&pool)
        .await
        .expect("insert token");
    }

    for (provider, sub, uid) in [
        ("github", "gh-ada", "u-ada"),
        ("google", "g-grace", "u-grace"),
    ] {
        sqlx::query(
            "INSERT INTO starter_auth_oauth_identities \
             (provider, provider_sub, user_id, email) \
             VALUES (?1, ?2, ?3, NULL)",
        )
        .bind(provider)
        .bind(sub)
        .bind(uid)
        .execute(&pool)
        .await
        .expect("insert identity");
    }

    // Sanity: fixture is what we think it is.
    assert_eq!(
        count(&pool, "SELECT COUNT(*) FROM starter_auth_users_users").await,
        3
    );
    assert_eq!(
        count(&pool, "SELECT COUNT(*) FROM starter_auth_users_sessions").await,
        2
    );
    assert_eq!(
        count(&pool, "SELECT COUNT(*) FROM starter_auth_users_tokens").await,
        2
    );
    assert_eq!(
        count(&pool, "SELECT COUNT(*) FROM starter_auth_oauth_identities").await,
        2
    );

    // Sanity: pre-0002 schema enforces NOT NULL on password_hash.
    let null_insert_err = sqlx::query(
        "INSERT INTO starter_auth_users_users \
         (id, email, password_hash, role) VALUES ('reject', 'reject@x', NULL, 'reader')",
    )
    .execute(&pool)
    .await
    .err();
    assert!(
        null_insert_err.is_some(),
        "pre-rebuild schema must reject NULL password_hash",
    );

    // ---- run the rebuild ---------------------------------------
    pool.execute(OAUTH_0002).await.expect("oauth 0002 rebuild");

    // (1) PRAGMA foreign_key_check returns no rows. Any row here
    // would be an FK pointing at a missing parent.
    let violations: Vec<SqliteRow> = sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(&pool)
        .await
        .expect("foreign_key_check");
    assert!(
        violations.is_empty(),
        "foreign_key_check returned {} violation row(s)",
        violations.len(),
    );

    // (2) Every fixture row survives via its FK.
    assert_eq!(
        count(&pool, "SELECT COUNT(*) FROM starter_auth_users_users").await,
        3
    );
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM starter_auth_users_sessions s \
             JOIN starter_auth_users_users u ON u.id = s.user_id",
        )
        .await,
        2,
        "sessions still join through to their users",
    );
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM starter_auth_users_tokens t \
             JOIN starter_auth_users_users u ON u.id = t.user_id",
        )
        .await,
        2,
        "tokens still join through to their users",
    );
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM starter_auth_oauth_identities i \
             JOIN starter_auth_users_users u ON u.id = i.user_id",
        )
        .await,
        2,
        "oauth identities still join through to their users",
    );

    // Pre-rebuild hashes carried through unchanged.
    let ada_hash: String =
        sqlx::query("SELECT password_hash FROM starter_auth_users_users WHERE id = 'u-ada'")
            .fetch_one(&pool)
            .await
            .expect("select hash")
            .get(0);
    assert_eq!(ada_hash, "$argon2id$v=19$dummy-for-u-ada");

    // (3) Post-rebuild, NULL password_hash is now legal.
    sqlx::query(
        "INSERT INTO starter_auth_users_users \
         (id, email, password_hash, role) \
         VALUES ('u-oauth-only', 'oauth-only@example.com', NULL, 'reader')",
    )
    .execute(&pool)
    .await
    .expect("post-rebuild NULL password_hash is accepted");

    // (4) ON DELETE CASCADE still fires through the renamed-into-
    // place users table.
    sqlx::query("DELETE FROM starter_auth_users_users WHERE id = 'u-ada'")
        .execute(&pool)
        .await
        .expect("delete u-ada");
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM starter_auth_users_sessions WHERE user_id = 'u-ada'",
        )
        .await,
        0,
        "session cascades on user delete",
    );
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM starter_auth_users_tokens WHERE user_id = 'u-ada'",
        )
        .await,
        0,
        "token cascades on user delete",
    );
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM starter_auth_oauth_identities WHERE user_id = 'u-ada'",
        )
        .await,
        0,
        "oauth identity cascades on user delete",
    );

    // Final FK consistency check after the cascade exercise.
    let post_delete_violations: Vec<SqliteRow> = sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(&pool)
        .await
        .expect("foreign_key_check post-delete");
    assert!(post_delete_violations.is_empty());
}
