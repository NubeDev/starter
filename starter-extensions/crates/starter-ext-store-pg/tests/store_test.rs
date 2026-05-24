//! Integration test for [`PgEnablementStore`] — spins up a real
//! Postgres container via `starter-store-postgres::testing::with_database`,
//! applies the crate's own `0001_extensions_enablement.sql` migration,
//! and exercises every persistence path the host + admin routes will
//! drive at runtime:
//!
//! - get / set roundtrip with both `EnablementState` variants
//! - UPSERT semantics: `set` twice for the same id with different
//!   states leaves exactly one row, last write wins
//! - `list_all` returns rows ordered by `extension_id`
//! - `set_as` records the operator id in the `updated_by` audit
//!   column (and the plain `set` records `system`)
//!
//! Gated behind the `testcontainers` feature so the default build
//! does not pull Docker-client crates. Run with:
//!
//! ```text
//! cargo test -p starter-ext-store-pg --features testcontainers
//! ```

#![cfg(feature = "testcontainers")]

use std::str::FromStr;

use starter_ext_server::{EnablementState, EnablementStore};
use starter_ext_spi::ExtensionId;
use starter_ext_store_pg::PgEnablementStore;
use starter_store_postgres::testing::with_database;

const MIGRATION_SQL: &str =
    include_str!("../src/migrations/0001_extensions_enablement.sql");

#[tokio::test]
async fn pg_enablement_store_roundtrips_upserts_lists_and_audits() {
    let (pool, _guard) = with_database().await;

    // Apply the crate's own migration. We execute the raw SQL rather
    // than wiring a `Migrator` so the test stays the single source of
    // truth for what the migration produces.
    sqlx::query(MIGRATION_SQL)
        .execute(pool.sqlx())
        .await
        .expect("apply 0001_extensions_enablement.sql");

    let store = PgEnablementStore::new(pool.sqlx().clone());

    let alpha = ExtensionId::from_str("com.acme.alpha").expect("valid id");
    let bravo = ExtensionId::from_str("com.acme.bravo").expect("valid id");
    let charlie = ExtensionId::from_str("com.acme.charlie").expect("valid id");

    // --- roundtrip: missing id reads None, then write + read back ---
    assert!(
        store.get(&alpha).await.expect("get missing ok").is_none(),
        "freshly migrated table has no rows"
    );

    store
        .set(&alpha, EnablementState::Enabled)
        .await
        .expect("set alpha enabled");
    assert_eq!(
        store.get(&alpha).await.expect("get alpha"),
        Some(EnablementState::Enabled),
    );

    // --- UPSERT: set twice for the same id, different state, ---
    // --- exactly one row remains, last write wins.             ---
    store
        .set(&alpha, EnablementState::Disabled)
        .await
        .expect("set alpha disabled (upsert path)");
    assert_eq!(
        store.get(&alpha).await.expect("get alpha after upsert"),
        Some(EnablementState::Disabled),
    );

    let row_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM extensions_enablement WHERE extension_id = $1")
            .bind(alpha.as_str())
            .fetch_one(pool.sqlx())
            .await
            .expect("count alpha rows");
    assert_eq!(row_count, 1, "UPSERT must not duplicate rows");

    // --- list_all ordering: insert out of alphabetical order, ---
    // --- expect list_all to come back sorted by extension_id. ---
    store
        .set(&charlie, EnablementState::Enabled)
        .await
        .expect("set charlie");
    store
        .set(&bravo, EnablementState::Disabled)
        .await
        .expect("set bravo");

    let listed = store.list_all().await.expect("list_all");
    let ids: Vec<&str> = listed.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["com.acme.alpha", "com.acme.bravo", "com.acme.charlie"],
        "list_all returns rows ordered by extension_id",
    );
    assert_eq!(
        listed
            .iter()
            .map(|(_, s)| *s)
            .collect::<Vec<_>>(),
        vec![
            EnablementState::Disabled,
            EnablementState::Disabled,
            EnablementState::Enabled,
        ],
    );

    // --- updated_by audit: plain `set` records "system";       ---
    // --- `set_as("operator-7", ...)` records the operator id.  ---
    let alpha_actor: String = sqlx::query_scalar(
        "SELECT updated_by FROM extensions_enablement WHERE extension_id = $1",
    )
    .bind(alpha.as_str())
    .fetch_one(pool.sqlx())
    .await
    .expect("read alpha updated_by");
    assert_eq!(alpha_actor, "system", "plain set records 'system'");

    store
        .set_as("operator-7", &bravo, EnablementState::Enabled)
        .await
        .expect("set_as bravo");

    let bravo_actor: String = sqlx::query_scalar(
        "SELECT updated_by FROM extensions_enablement WHERE extension_id = $1",
    )
    .bind(bravo.as_str())
    .fetch_one(pool.sqlx())
    .await
    .expect("read bravo updated_by");
    assert_eq!(
        bravo_actor, "operator-7",
        "set_as records the supplied actor in updated_by",
    );
    assert_eq!(
        store.get(&bravo).await.expect("get bravo"),
        Some(EnablementState::Enabled),
        "set_as also flips the state column",
    );
}
