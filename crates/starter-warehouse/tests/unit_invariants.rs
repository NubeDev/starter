//! Pure-logic integration tests — exercise the W-rule helpers that
//! do not need a live ClickHouse or Postgres. Container-backed
//! tests live in `tests/with_stack.rs` and run via
//! `cargo test --features 'warehouse testing' -- --ignored`.

#![cfg(feature = "warehouse")]

use starter_warehouse::catalog::ext::Author;
use starter_warehouse::catalog::mart_spec::{AggregationSpec, MartSpec};
use starter_warehouse::ddl;

fn fixture() -> MartSpec {
    MartSpec {
        name: "mart_energy_hourly".into(),
        description: None,
        source_table: "samples".into(),
        filter: serde_json::json!({}),
        time_bucket_secs: 3600,
        group_by: vec!["building".into(), "tenant".into()],
        aggregations: vec![AggregationSpec {
            func: "sum".into(),
            col: "value_num".into(),
            alias: "kwh".into(),
        }],
        created_by: "user:alice".into(),
        ext_manifest_hash: None,
    }
}

#[test]
fn w5_definition_hash_is_deterministic() {
    let a = fixture().definition_hash();
    let b = fixture().definition_hash();
    assert_eq!(a, b);
}

#[test]
fn w5_definition_hash_changes_with_group_by() {
    let mut s = fixture();
    s.group_by.push("floor".into());
    assert_ne!(fixture().definition_hash(), s.definition_hash());
}

#[test]
fn w5_d7_order_by_promotes_first_group_by_column() {
    let ddl = ddl::mart::build(&fixture()).unwrap();
    assert!(
        ddl.create_target.contains("ORDER BY (building, bucket, tenant)"),
        "ORDER BY must be (<first group_by>, bucket, <rest>): got {}",
        ddl.create_target
    );
}

#[test]
fn w5_promoted_columns_include_group_by_and_aliases() {
    let cols = fixture().promoted_columns();
    assert_eq!(cols, vec!["building", "tenant", "kwh"]);
}

#[test]
fn w12_author_table_user_pending() {
    let a = Author::parse("user:alice", None).unwrap();
    assert!(matches!(a, Author::User("alice")));
}

#[test]
fn w12_author_table_agent_quarantined() {
    let a = Author::parse("agent:claude", None).unwrap();
    assert!(matches!(a, Author::Agent("claude")));
}

#[test]
fn w12_author_table_ext_carries_manifest_hash() {
    let a = Author::parse("ext:com.acme.cleaner", Some("hashAAA")).unwrap();
    if let Author::Ext { id, manifest_hash } = a {
        assert_eq!(id, "com.acme.cleaner");
        assert_eq!(manifest_hash, "hashAAA");
    } else {
        panic!("expected Author::Ext");
    }
}

#[test]
fn w12_author_table_rejects_unknown_prefix() {
    let r = Author::parse("daemon:foo", None);
    assert!(r.is_err());
}

#[test]
fn ddl_validates_identifiers() {
    let mut s = fixture();
    s.group_by = vec!["BAD-NAME".into()];
    assert!(ddl::mart::build(&s).is_err());
}

#[test]
fn ddl_rejects_empty_group_by() {
    let mut s = fixture();
    s.group_by.clear();
    assert!(ddl::mart::build(&s).is_err());
}

#[test]
fn cleaner_ddl_requires_deterministic_key_for_sync_backfill() {
    use starter_warehouse::ddl::cleaner::{build, CleanerSpec};
    let s = CleanerSpec {
        name: "c".into(),
        source_table: "raw_events".into(),
        target_table: "samples".into(),
        projection: "entity_id, ts, value_num".into(),
        backfill: "sync".into(),
        deterministic_key: false,
    };
    assert!(build(&s).is_err());
}

#[test]
fn sandbox_ddl_emits_create_with_ttl() {
    use starter_warehouse::ddl::sandbox::{build, SandboxColumn, SandboxSpec};
    let s = SandboxSpec {
        name: "utility_bills_2025".into(),
        ttl_days: 60,
        columns: vec![SandboxColumn {
            name: "amount".into(),
            r#type: "Float64".into(),
        }],
    };
    let d = build(&s).unwrap();
    assert!(d.create_table.contains("TTL ts + INTERVAL 60 DAY"));
    assert!(d.create_table.contains("ENGINE = MergeTree"));
}

#[test]
fn w14_collect_keys_walks_and_or_not() {
    use starter_tags::TagQuery;
    use std::str::FromStr;
    let q = TagQuery::from_str("(building:\"a\" and not floor:\"1\") or kind").unwrap();
    let mut keys = Vec::new();
    starter_warehouse::nodes::runtime::collect_keys(&q, &mut keys);
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(sorted, vec!["building", "floor", "kind"]);
}
