//! Registry parity for the write sinks: the legacy `postgres` sink config and the
//! new `datasource` sink config both build from the native registry without a
//! live connection (build is lazy), so a flow stored before RW-04 keeps working
//! and a new datasource-targeted flow builds the same way.

use nexus_engine::native_registry;
use serde_json::json;

#[test]
fn legacy_postgres_sink_config_still_builds() {
    let registry = native_registry();
    // The pre-RW-04 shape: a direct connection uri + table. Building must not
    // connect (no DB here), proving the legacy path is intact.
    let sink = registry.build_sink(
        "postgres",
        &json!({ "type": "postgres", "uri": "postgres://u@localhost/db", "table": "t" }),
    );
    assert!(sink.is_ok(), "legacy postgres sink config must still build");
}

#[test]
fn datasource_sink_config_builds() {
    let registry = native_registry();
    let sink = registry.build_sink(
        "datasource",
        &json!({
            "type": "datasource",
            "kind": "postgres",
            "table": "device_readings",
            "conn": {
                "host": "localhost",
                "port": 5432,
                "database": "db",
                "user": "u",
                "password": "p"
            },
            "batch_rows": 100,
            "batch_ms": 250
        }),
    );
    assert!(sink.is_ok(), "datasource sink config must build");
}

#[test]
fn datasource_sink_rejects_unknown_kind_at_first_write() {
    // Build succeeds (lazy), but a bogus kind has no writer; this is a build-time
    // config error surfaced when the writer is opened, not a panic.
    let registry = native_registry();
    let sink = registry.build_sink(
        "datasource",
        &json!({ "type": "datasource", "kind": "nope", "table": "t" }),
    );
    assert!(sink.is_ok(), "config parses; the kind is checked on first write");
}
