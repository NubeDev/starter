//! Phase 1 gate test for the extension-substrate's
//! `WarehouseReadHandle` end-to-end:
//!
//! - Spin a real Timescale container via `with_timescale()`.
//! - Run `run_migrations` so the `samples` hypertable exists.
//! - Seed three tenants × elec.meter samples.
//! - Build a [`CtxInner`] wired to
//!   [`RubixWarehouseReadBackend`] and stamp a [`CallerIdentity`]
//!   whose `tenant_id` is `t-1`.
//! - Call `ctx.warehouse_read().query("meter_kwh_last_24h", {})`
//!   and assert the row reflects `t-1` only.
//! - Repeat with `tenant_id = t-2` and assert the row reflects
//!   `t-2` only.
//! - Repeat with the same backend but `caller = None` (system
//!   frame) and assert [`Error::Capability`] — the tenancy clamp
//!   is enforced by the **backend**, not by the resolver, so the
//!   refusal must surface before any SQL is issued.
//!
//! Per `rubix/docs/scope/extensions-north-star/README.md` row 5
//! this is the gate that closes the soft trust boundary
//! identified in Appendix A of the
//! extension-architecture-north-star proposal.

use std::collections::BTreeSet;
use std::sync::Arc;

use chrono::Utc;
use rubix_agent::extensions::backends::RubixWarehouseReadBackend;
use serde_json::json;
use starter_ext_host::TemplateRegistry;
use starter_ext_sdk::ctx::WarehouseReadBackend;
use starter_ext_spi::Error;
use starter_store_warehouse::store::samples::{insert_many, SampleRow};
use starter_store_warehouse::{run_migrations, testing::with_timescale};

fn sample(tenant: &str, kind: &str, value: f64) -> SampleRow {
    SampleRow {
        tenant_id: tenant.to_owned(),
        entity_id: format!("{tenant}.{kind}.meter-1"),
        ts: Utc::now(),
        value_num: Some(value),
        value_str: None,
        value_bool: None,
        quality: 0,
        tags: serde_json::json!({}),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires Docker (testcontainers Timescale); run via the integration job"]
async fn warehouse_backend_clamps_tenancy_from_caller() {
    let (client, _guard) = with_timescale().await;
    run_migrations(&client).await.expect("run timescale migrations");

    // Three tenants × elec meter readings. Each tenant's most-
    // recent value is distinct so the test can prove the backend
    // surfaces the *right* tenant's row.
    insert_many(
        &client,
        &[
            sample("t-1", "elec", 11.0),
            sample("t-2", "elec", 22.0),
            sample("t-3", "elec", 33.0),
        ],
    )
    .await
    .expect("seed samples");

    let registry = Arc::new(TemplateRegistry::builtin());
    let granted = Some(BTreeSet::from(["samples".to_owned()]));

    // --- t-1 sees only t-1's row.
    let backend_t1 = RubixWarehouseReadBackend::new(
        client.clone(),
        registry.clone(),
        Some("t-1".to_owned()),
        granted.clone(),
    );
    let rows = tokio::task::spawn_blocking(move || {
        backend_t1.query("meter_kwh_last_24h", json!({}))
    })
    .await
    .expect("join")
    .expect("query ok");
    assert_eq!(rows.len(), 1, "exactly one row for t-1: got {rows:?}");
    let kwh = rows[0]
        .as_map()
        .get("kwh")
        .and_then(|v| v.as_f64())
        .expect("kwh column present");
    assert!(
        (kwh - 11.0).abs() < f64::EPSILON,
        "t-1 must see only its own value 11.0, got {kwh}"
    );

    // --- t-2 sees only t-2's row.
    let backend_t2 = RubixWarehouseReadBackend::new(
        client.clone(),
        registry.clone(),
        Some("t-2".to_owned()),
        granted.clone(),
    );
    let rows = tokio::task::spawn_blocking(move || {
        backend_t2.query("meter_kwh_last_24h", json!({}))
    })
    .await
    .expect("join")
    .expect("query ok");
    assert_eq!(rows.len(), 1, "exactly one row for t-2");
    let kwh = rows[0]
        .as_map()
        .get("kwh")
        .and_then(|v| v.as_f64())
        .expect("kwh column present");
    assert!(
        (kwh - 22.0).abs() < f64::EPSILON,
        "t-2 must see only its own value 22.0, got {kwh}"
    );

    // --- system frame (caller = None) is refused with
    //     Error::Capability, regardless of grant.
    let backend_sys = RubixWarehouseReadBackend::new(
        client.clone(),
        registry.clone(),
        None,
        granted.clone(),
    );
    let err = tokio::task::spawn_blocking(move || {
        backend_sys.query("meter_kwh_last_24h", json!({}))
    })
    .await
    .expect("join")
    .expect_err("system frame must be refused");
    assert!(
        matches!(err, Error::Capability(_)),
        "expected Error::Capability, got {err:?}"
    );

    // --- bucketed-series template binds caller tenant too.
    // Insert a couple of historical values for t-1's water meter
    // so the bucketed query returns rows.
    insert_many(
        &client,
        &[
            sample("t-1", "water", 100.0),
            sample("t-1", "water", 101.0),
        ],
    )
    .await
    .expect("seed water samples");
    let backend_t1_bucketed = RubixWarehouseReadBackend::new(
        client.clone(),
        registry.clone(),
        Some("t-1".to_owned()),
        granted.clone(),
    );
    let rows = tokio::task::spawn_blocking(move || {
        backend_t1_bucketed.query(
            "meter_value_24h_1m",
            json!({ "meter_id": "t-1.water.meter-1" }),
        )
    })
    .await
    .expect("join")
    .expect("query ok");
    assert!(
        !rows.is_empty(),
        "bucketed query must return at least one row for t-1's water meter"
    );

    // --- count() honours the same gate.
    let backend_for_count = RubixWarehouseReadBackend::new(
        client.clone(),
        registry,
        Some("t-3".to_owned()),
        granted,
    );
    let n = tokio::task::spawn_blocking(move || {
        backend_for_count.count("meter_kwh_last_24h", json!({}))
    })
    .await
    .expect("join")
    .expect("count ok");
    assert_eq!(n, 1, "t-3 sees one elec row");
}
