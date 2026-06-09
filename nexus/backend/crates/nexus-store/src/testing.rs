//! Test-only helpers for exercising the store under the real runtime role.
//!
//! RLS is bypassed by superusers, so tenant-isolation tests must connect as the
//! non-superuser `nexus_runtime` role the migration creates — proving isolation
//! the way production runs, not under the testcontainers `postgres` superuser.

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::migrate;

/// Run the migrations as `admin`, give `nexus_runtime` a login, and return a
/// single-connection pool connected as that role. The single connection forces
/// connection reuse, the condition tenant-leak tests need; the runtime role has
/// no BYPASSRLS, so RLS policies are actually enforced.
pub async fn runtime_pool(admin: &PgPool) -> PgPool {
    migrate::run(admin).await.expect("migrations apply");
    sqlx::query("ALTER ROLE nexus_runtime LOGIN PASSWORD 'runtimepw'")
        .execute(admin)
        .await
        .expect("runtime login");
    sqlx::query("GRANT USAGE ON SCHEMA public TO nexus_runtime")
        .execute(admin)
        .await
        .expect("grant schema usage");

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
