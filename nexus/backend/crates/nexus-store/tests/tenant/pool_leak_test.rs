//! R5 contract: tenant isolation cannot leak across a pooled connection.
//!
//! This is the load-bearing security test, not an optional extra. It runs under
//! the real runtime role (non-superuser, non-owner, no BYPASSRLS) against a
//! single pooled connection, and proves that serving tenant A then tenant B
//! back-to-back on that same connection never lets B read A's rows — i.e. the
//! `SET LOCAL app.tenant_id` binding is transaction-scoped and the RLS policy
//! actually enforces it.

#![cfg(feature = "testing")]

use nexus_store::{migrate, tenant_tx};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use starter_store_postgres::testing::with_database;

/// Stand up the schema, give the runtime role a login, seed one datasource per
/// tenant (as the superuser admin, which bypasses RLS for setup), and return a
/// pool connected **as the runtime role** with a single connection — so every
/// query reuses the same backend, the condition a leak would need.
async fn runtime_pool(admin: &PgPool) -> PgPool {
    migrate::run(admin).await.expect("migrations apply");

    sqlx::query("ALTER ROLE nexus_runtime LOGIN PASSWORD 'runtimepw'")
        .execute(admin)
        .await
        .expect("runtime login");
    sqlx::query("GRANT USAGE ON SCHEMA public TO nexus_runtime")
        .execute(admin)
        .await
        .expect("grant schema usage");

    seed(admin, "acme", "acme-ds").await;
    seed(admin, "globex", "globex-ds").await;

    // Reuse the admin pool's connection target (host/port from the container),
    // overriding only the credentials — so the runtime pool reaches the same
    // database without re-deriving the mapped port.
    let opts = admin
        .connect_options()
        .as_ref()
        .clone()
        .username("nexus_runtime")
        .password("runtimepw");
    PgPoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .expect("connect as runtime role")
}

async fn seed(admin: &PgPool, tenant: &str, name: &str) {
    sqlx::query(
        "INSERT INTO nexus_datasources \
         (tenant_id, name, kind, host, port, database, db_user, secret_cipher, secret_nonce) \
         VALUES ($1, $2, 'postgres', 'h', 5432, 'd', 'u', '\\x00', '\\x00')",
    )
    .bind(tenant)
    .bind(name)
    .execute(admin)
    .await
    .expect("seed datasource");
}

async fn names_for(pool: &PgPool, tenant: &str) -> Vec<String> {
    let mut tx = tenant_tx::begin(pool, tenant)
        .await
        .expect("begin tenant tx");
    let rows = sqlx::query("SELECT name FROM nexus_datasources ORDER BY name")
        .fetch_all(&mut *tx)
        .await
        .expect("select under tenant");
    tx.commit().await.expect("commit");
    rows.into_iter()
        .map(|r| r.get::<String, _>("name"))
        .collect()
}

#[tokio::test]
#[ignore = "requires docker"]
async fn tenant_cannot_read_across_a_pooled_connection() {
    let (pool, _guard) = with_database().await;
    let runtime = runtime_pool(pool.sqlx()).await;

    // Serve acme, then globex, then acme again — all on the one pooled
    // connection. Each tenant sees only its own row; the previous tenant's GUC
    // never bleeds through.
    assert_eq!(names_for(&runtime, "acme").await, ["acme-ds"]);
    assert_eq!(names_for(&runtime, "globex").await, ["globex-ds"]);
    assert_eq!(
        names_for(&runtime, "acme").await,
        ["acme-ds"],
        "acme still sees only its own row after globex used the same connection"
    );

    // A connection with no tenant bound sees nothing — a forgotten SET LOCAL
    // fails closed, not open.
    let leaked = sqlx::query("SELECT count(*)::int AS n FROM nexus_datasources")
        .fetch_one(&runtime)
        .await
        .expect("count without tenant binding")
        .get::<i32, _>("n");
    assert_eq!(leaked, 0, "no tenant bound ⇒ zero rows, never all rows");

    drop(runtime);
}
