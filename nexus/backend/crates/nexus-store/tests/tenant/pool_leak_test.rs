//! R5 contract: tenant isolation cannot leak across a pooled connection.
//!
//! This is the load-bearing security test, not an optional extra. It runs under
//! the real runtime role (non-superuser, non-owner, no BYPASSRLS) against a
//! single pooled connection, and proves that serving tenant A then tenant B
//! back-to-back on that same connection never lets B read A's rows — i.e. the
//! `SET LOCAL app.tenant_id` binding is transaction-scoped and the RLS policy
//! actually enforces it.

#![cfg(feature = "testing")]

use nexus_store::tenant_tx;
use nexus_store::testing::runtime_pool;
use sqlx::{PgPool, Row};
use starter_store_postgres::testing::with_database;

async fn seed(admin: &PgPool, tenant: &str, name: &str) {
    sqlx::query(
        "INSERT INTO nexus_datasources \
         (tenant_id, name, kind, host, port, database, db_user, \
          secret_cipher, secret_nonce, wrapped_data_key, data_key_nonce) \
         VALUES ($1, $2, 'postgres', 'h', 5432, 'd', 'u', \
                 '\\x00', '\\x00', '\\x00', '\\x00')",
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
    // Seed as the admin (bypasses RLS for setup); the test reads as the runtime.
    seed(pool.sqlx(), "acme", "acme-ds").await;
    seed(pool.sqlx(), "globex", "globex-ds").await;

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
