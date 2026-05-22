//! S6 — agent rule/verdict/pipeline tool bridge.
//!
//! Asserts:
//! 1. `synthesize_insights_tools` returns the 5 rule defs when the
//!    agent declares `insights:rule.*` (wildcard), and an empty vec
//!    when the agent declares no insights tools.
//! 2. `dispatch_insights_tool` round-trips `rule.read` against
//!    seeded fixtures.
//! 3. `rule.propose` returns a proposal object without writing;
//!    `rule.apply` does write through to disk. The propose-vs-apply
//!    contract is load-bearing per spec §Agent tools.

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{json, Value};
use starter_ai::Registry as AiRegistry;
use starter_spi::ai::ToolUse;
use starter_store_sqlite::{migrate, pool};

use flow_agent::ai_runtime::AiRuntime;
use flow_agent::flow_engine::FlowEngine;
use flow_agent::insights_mock::{InsightsFixtures, InsightsState};
use flow_agent::migrations;
use flow_agent::sse::EventHub;
use flow_agent::store::{FlowStore, RunStore};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/insights")
}

async fn make_runtime() -> (AiRuntime, tempfile::TempDir) {
    let tmp = tempfile::tempdir().expect("tempdir");
    for name in [
        "rules.json",
        "verdicts.json",
        "pipelines.json",
        "coverage.json",
        "tags-index.json",
    ] {
        std::fs::copy(fixtures_dir().join(name), tmp.path().join(name)).expect("copy fixture");
    }
    let data = InsightsFixtures::load(tmp.path()).expect("load fixtures");
    let state = InsightsState::new(data);

    let pool = pool::connect("sqlite::memory:").await.expect("connect");
    let mut chain = migrate(&pool);
    for src in migrations::sources() {
        chain = chain.with_source(src);
    }
    chain.run().await.expect("migrate");
    let sqlx = pool.sqlx().clone();
    let flows = Arc::new(FlowStore::new(sqlx.clone()));
    let runs = Arc::new(RunStore::new(sqlx));
    let hub = Arc::new(EventHub::new());

    let ai = AiRuntime::with_registry(
        Arc::new(AiRegistry::default()),
        flows,
        FlowEngine::new(),
        runs,
        hub,
    )
    .with_insights(state);
    (ai, tmp)
}

#[tokio::test]
async fn synth_rule_tools_lights_under_wildcard() {
    let (ai, _tmp) = make_runtime().await;
    let tools = ai.synthesize_insights_tools(&["insights:rule.*".into()]);
    assert_eq!(tools.len(), 5, "expected 5 rule tools, got {}", tools.len());
    assert!(tools.iter().any(|t| t.name == "insights:rule.propose"));
    assert!(tools.iter().any(|t| t.name == "insights:rule.dry-run"));

    let empty = ai.synthesize_insights_tools(&["flow:*".into()]);
    assert!(empty.is_empty());
}

#[tokio::test]
async fn dispatch_rule_read_returns_fixture_row() {
    let (ai, _tmp) = make_runtime().await;
    let tu = ToolUse {
        id: "t1".into(),
        name: "insights:rule.read".into(),
        input: json!({ "id": "device.online@1" }),
    };
    let out = ai.dispatch_insights_tool(&tu).await;
    let parsed: Value = serde_json::from_str(&out).expect("json");
    assert_eq!(parsed["id"], "device.online@1");
}

#[tokio::test]
async fn propose_does_not_mutate_apply_does() {
    let (ai, tmp) = make_runtime().await;

    let propose = ai
        .dispatch_insights_tool(&ToolUse {
            id: "p1".into(),
            name: "insights:rule.propose".into(),
            input: json!({ "id": "new.rule@1", "kind": "rule.rhai" }),
        })
        .await;
    let parsed: Value = serde_json::from_str(&propose).unwrap();
    assert_eq!(parsed["needs_approval"], true);
    let on_disk_before: Value =
        serde_json::from_slice(&std::fs::read(tmp.path().join("rules.json")).unwrap()).unwrap();
    assert!(!on_disk_before
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"] == "new.rule@1"));

    let apply = ai
        .dispatch_insights_tool(&ToolUse {
            id: "a1".into(),
            name: "insights:rule.apply".into(),
            input: json!({
                "id": "new.rule@1",
                "kind": "rule.rhai",
                "namespace": "test",
                "severity_default": "Warn",
                "tags": [],
                "summary": "",
                "body": "",
                "schema": {},
                "created_at": "2026-05-22T00:00:00Z",
                "updated_at": "2026-05-22T00:00:00Z"
            }),
        })
        .await;
    let apply_p: Value = serde_json::from_str(&apply).unwrap();
    assert_eq!(apply_p["ok"], true);
    let on_disk_after: Value =
        serde_json::from_slice(&std::fs::read(tmp.path().join("rules.json")).unwrap()).unwrap();
    assert!(on_disk_after
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"] == "new.rule@1"));
}
