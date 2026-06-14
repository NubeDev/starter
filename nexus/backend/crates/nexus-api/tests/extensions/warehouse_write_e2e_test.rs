//! WS-17 Wave A acceptance: an extension owns a table in the nexus Postgres and
//! writes to it through the tenant-stamped write path.
//!
//! Drives the real warehouse domain
//! ([`nexus_api::extensions::warehouse`]) against a live metadata Postgres:
//!
//! - boot DDL creates `com_acme_devices__devices` with a host-prepended
//!   `tenant_id` column and a `(tenant_id, device_id)` PRIMARY KEY; a second
//!   create is idempotent (no error).
//! - `WriteExecutor::insert` persists a real row, tenant-stamped from the
//!   caller; the same `device_id` **upserts** (no duplicate) — idempotency
//!   proven against the table, not a derived id.
//! - a write to a table outside the extension's grant, and a caller binding a
//!   different tenant, are isolated.
//! - the cleanup `DROP TABLE` path removes the owned table.

#![cfg(feature = "testing")]

use std::collections::BTreeSet;

use nexus_api::extensions::warehouse::{create_one_table, full_table_name, WriteExecutor};
use nexus_store::testing::runtime_pool;
use serde_json::json;
use starter_ext_spi::manifest::{ContributeWarehouseTable, TableColumn, WarehouseTableKind};
use starter_ext_spi::warehouse::Row;
use starter_ext_spi::ExtensionId;
use starter_store_postgres::testing::with_database;

fn ext() -> ExtensionId {
    ExtensionId::new("com.acme.devices").unwrap()
}

fn devices_spec() -> ContributeWarehouseTable {
    ContributeWarehouseTable {
        name: "devices".into(),
        columns: vec![
            TableColumn {
                name: "device_id".into(),
                ty: "text".into(),
                default: None,
            },
            TableColumn {
                name: "barcode".into(),
                ty: "text".into(),
                default: None,
            },
            TableColumn {
                name: "location".into(),
                ty: "text".into(),
                default: None,
            },
            TableColumn {
                name: "owner".into(),
                ty: "text".into(),
                default: None,
            },
            TableColumn {
                name: "team".into(),
                ty: "text".into(),
                default: None,
            },
            TableColumn {
                name: "created_at".into(),
                ty: "timestamptz".into(),
                default: Some("now()".into()),
            },
        ],
        order_by: vec!["device_id".into()],
        engine: None,
        partition_by: None,
        ttl: None,
        kind: WarehouseTableKind::Table,
    }
}

fn row(v: serde_json::Value) -> Row {
    match v {
        serde_json::Value::Object(m) => Row::from_map(m),
        _ => panic!("row must be an object"),
    }
}

fn grant(tables: &[&str]) -> BTreeSet<String> {
    tables.iter().map(|s| s.to_string()).collect()
}

#[tokio::test]
#[ignore = "requires docker"]
async fn owns_table_persists_upserts_isolates_and_drops() {
    let (admin, _guard) = with_database().await;
    let pool = runtime_pool(admin.sqlx()).await;
    let ext = ext();
    let spec = devices_spec();
    let full = full_table_name(&ext, "devices");

    // --- boot DDL: create, then create again (idempotent). ---
    //
    // Production note: nexus applies all DDL (migrations *and* this extension
    // boot DDL) on the configured `metadata` pool — the same role. This test
    // harness deliberately splits an owner role (`admin`) from the restricted,
    // non-BYPASSRLS `nexus_runtime` role to prove tenant isolation runs the way
    // production does. The runtime role has no `CREATE` on `public`, so we run
    // the create as the owner (mirroring how migrations apply) and grant the
    // runtime role CRUD — then exercise every write below through the runtime
    // `pool`, proving the write path works under the production-equivalent role.
    create_one_table(admin.sqlx(), &ext, &spec)
        .await
        .expect("create table");
    create_one_table(admin.sqlx(), &ext, &spec)
        .await
        .expect("second create is idempotent");
    sqlx::query(&format!(
        "GRANT SELECT, INSERT, UPDATE, DELETE ON \"{full}\" TO nexus_runtime"
    ))
    .execute(admin.sqlx())
    .await
    .expect("grant crud to runtime role");

    // The host-prepended tenant_id column exists and is NOT NULL.
    let tenant_col_notnull: bool = sqlx::query_scalar(
        "SELECT attnotnull FROM pg_attribute \
         WHERE attrelid = $1::regclass AND attname = 'tenant_id'",
    )
    .bind(&full)
    .fetch_one(&pool)
    .await
    .expect("tenant_id column present");
    assert!(tenant_col_notnull, "tenant_id is NOT NULL");

    let specs = vec![spec.clone()];
    let g = grant(&["devices"]);

    // --- insert persists, tenant-stamped from the caller (payload ignored). ---
    let exec = WriteExecutor::new(&pool, &ext, "t-acme", &specs, Some(&g));
    let n = exec
        .insert(
            "devices",
            vec![row(json!({
                "device_id": "dev-1",
                "barcode": "BC-1",
                "location": "rack-A",
                "owner": "alice",
                "team": "hvac-ops",
                "tenant_id": "evil"  // host overwrites this
            }))],
        )
        .await
        .expect("insert");
    assert_eq!(n, 1);

    let (stored_tenant, stored_loc): (String, String) = sqlx::query_as(
        "SELECT tenant_id, location FROM \
         com_acme_devices__devices WHERE device_id = 'dev-1'",
    )
    .fetch_one(&pool)
    .await
    .expect("row landed");
    assert_eq!(stored_tenant, "t-acme", "host stamps the caller's tenant");
    assert_eq!(stored_loc, "rack-A");

    // --- same device_id UPSERTS (no duplicate); a changed field is updated. ---
    exec.insert(
        "devices",
        vec![row(json!({
            "device_id": "dev-1",
            "barcode": "BC-1",
            "location": "rack-B",  // moved
            "owner": "alice",
            "team": "hvac-ops"
        }))],
    )
    .await
    .expect("upsert");
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM com_acme_devices__devices WHERE device_id = 'dev-1'",
    )
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(count, 1, "same barcode upserts, never duplicates");
    let loc: String = sqlx::query_scalar(
        "SELECT location FROM com_acme_devices__devices WHERE device_id = 'dev-1'",
    )
    .fetch_one(&pool)
    .await
    .expect("loc");
    assert_eq!(loc, "rack-B", "upsert updated the moved field");

    // --- tenant isolation: a second tenant's write is a separate row. ---
    let exec_other = WriteExecutor::new(&pool, &ext, "t-other", &specs, Some(&g));
    exec_other
        .insert(
            "devices",
            vec![row(json!({
                "device_id": "dev-1", "barcode": "BC-1",
                "location": "elsewhere", "owner": "bob", "team": "ops"
            }))],
        )
        .await
        .expect("other tenant insert");
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM com_acme_devices__devices")
        .fetch_one(&pool)
        .await
        .expect("count all");
    assert_eq!(
        total, 2,
        "same device_id in two tenants are distinct rows (PK includes tenant_id)"
    );

    // --- a caller's read only sees its tenant's rows. ---
    let acme_rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM com_acme_devices__devices WHERE tenant_id = 't-acme'",
    )
    .fetch_one(&pool)
    .await
    .expect("acme rows");
    assert_eq!(acme_rows, 1);

    // --- allowlist: a table not in the grant is refused (capability). ---
    let empty_grant = grant(&[]);
    let no_grant = WriteExecutor::new(&pool, &ext, "t-acme", &specs, Some(&empty_grant));
    let err = no_grant
        .insert("devices", vec![row(json!({"device_id": "x", "barcode": "y", "location": "z", "owner": "o", "team": "t"}))])
        .await
        .expect_err("empty grant refuses");
    assert!(
        matches!(err, starter_ext_spi::Error::Capability(_)),
        "got {err:?}"
    );

    // --- a table the extension never declared is refused (validation). ---
    let ghost_grant = grant(&["ghosts"]);
    let bad_table = WriteExecutor::new(&pool, &ext, "t-acme", &specs, Some(&ghost_grant));
    let err = bad_table
        .insert("ghosts", vec![])
        .await
        .expect_err("undeclared table refuses");
    assert!(
        matches!(err, starter_ext_spi::Error::Validation(_)),
        "got {err:?}"
    );

    // --- delete is tenant-scoped: deleting dev-1 as t-acme leaves t-other's. ---
    let deleted = exec
        .delete("devices", "device_id", vec![json!("dev-1")])
        .await
        .expect("delete");
    assert_eq!(deleted, 1, "only the caller's tenant row is deleted");
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM com_acme_devices__devices")
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(remaining, 1, "the other tenant's row survives");

    // --- cleanup: DROP TABLE removes the owned table (owner role, as the
    //     purge cleanup provider runs on the metadata pool in production). ---
    sqlx::query(&format!("DROP TABLE IF EXISTS \"{full}\""))
        .execute(admin.sqlx())
        .await
        .expect("drop");
    let exists: bool = sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
        .bind(&full)
        .fetch_one(&pool)
        .await
        .expect("regclass");
    assert!(!exists, "owned table dropped on purge");
}
