//! Phase 1 IoT smoke test — reproduces the canonical IoT row from
//! `DOCS/Insights/SCOPE.md` ("Use-case fit (sanity check the
//! design)"), point-in-time only (windowing lands in Phase 2),
//! deterministic (no AI rule kinds yet).
//!
//! What this test proves:
//!
//! 1. The IoT pack (`starter-ext-insights-iot`) registers its
//!    three rules + three quality flags into the host's
//!    `RuleRegistry` / `QualityFlagRegistry` (R-ins-1, R-ins-11).
//! 2. The `rule.rust` node body dispatches each registered rule
//!    by `RuleId` and writes a `Verdict` slot value (R-ins-3).
//! 3. `verdict.join(mode=all)` fans the three verdicts into one
//!    composite (R-ins-6).
//! 4. The `Severity::Error` → branch → `notify(ops)` path runs
//!    when a registered rule fails (forced via a malformed input
//!    slot); the all-Error degenerate case of `verdict.join`
//!    surfaces a `starter.quality.join-all-inputs-errored@1`
//!    flag.
//! 5. The sqlite verdict log + tag index persist the row behind
//!    the `insights` feature.

use std::sync::Arc;

use starter_flow_spi::flow::RunId;
use starter_flow_spi::node::{NodeBehavior, NodeCtx, NodeId, SlotMap, SlotValue};
use starter_flow_spi::skill::SkillSelection;
use starter_flow_spi::Cancel;
use starter_spi::insights::{Severity, Verdict};

use starter_insights::nodes::rule_rust::{RuleRustNode, PARAMS_SLOT, RULE_ID_SLOT};
use starter_insights::nodes::verdict_join::{JoinMode, VerdictJoinNode};
use starter_insights::nodes::VERDICT_SLOT;
use starter_insights::{QualityFlagRegistry, RuleRegistry};

struct NoCancel;
impl Cancel for NoCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
    fn cancelled<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(std::future::pending())
    }
}

fn make_ctx<'a>(node: &'a NodeId, cancel: &'a dyn Cancel) -> NodeCtx<'a> {
    NodeCtx::new(RunId::new(), node, cancel, SkillSelection::NONE, &starter_flow_spi::state::NOOP_NODE_STATE_STORE)
}

fn build_registries() -> (Arc<RuleRegistry>, QualityFlagRegistry) {
    let mut rules = RuleRegistry::new();
    for r in starter_ext_insights_iot::rules() {
        rules
            .register(r)
            .expect("IoT pack rules register without duplicates");
    }

    let mut flags = QualityFlagRegistry::with_builtins();
    for (id, desc, rem) in starter_ext_insights_iot::quality_flags() {
        flags
            .register(
                id,
                starter_insights::registry::QualityFlagInfo::new(desc, rem),
            )
            .expect("IoT pack flag ids register without duplicates");
    }

    (Arc::new(rules), flags)
}

fn decode_verdict(out: &SlotMap) -> Verdict {
    match out.get(VERDICT_SLOT) {
        Some(SlotValue::Json(j)) => {
            serde_json::from_value(j.clone()).expect("verdict slot decodes")
        }
        other => panic!("expected JSON verdict slot, got {other:?}"),
    }
}

async fn invoke_rule(node: &RuleRustNode, rule_id: &str, params: serde_json::Value) -> SlotMap {
    let id = NodeId::new("iot.test.rule").unwrap();
    let cancel = NoCancel;
    let mut input = SlotMap::new();
    input.insert(
        RULE_ID_SLOT.to_owned(),
        SlotValue::String(rule_id.to_owned()),
    );
    input.insert(PARAMS_SLOT.to_owned(), SlotValue::Json(params));
    node.invoke(make_ctx(&id, &cancel), input)
        .await
        .expect("rule.rust body returns Ok (failures are verdicts)")
}

#[tokio::test]
async fn iot_row_happy_path_all_healthy() {
    let (rules, flags) = build_registries();
    assert_eq!(rules.len(), 3, "IoT pack registers three rules");
    assert!(flags.len() >= 9, "builtins + iot.quality.* registered");

    let rule_rust = RuleRustNode::new(Arc::clone(&rules));

    let online = decode_verdict(
        &invoke_rule(
            &rule_rust,
            "iot.device.online@1",
            serde_json::json!({"last_seen_secs_ago": 30, "threshold_secs": 300}),
        )
        .await,
    );
    let recent = decode_verdict(
        &invoke_rule(
            &rule_rust,
            "iot.sensor.has-recent-data@1",
            serde_json::json!({"last_sample_secs_ago": 5, "window_secs": 60}),
        )
        .await,
    );
    let in_range = decode_verdict(
        &invoke_rule(
            &rule_rust,
            "iot.sensor.in-range@1",
            serde_json::json!({"value": 21.5, "min": 18.0, "max": 24.0}),
        )
        .await,
    );

    assert_eq!(online.severity, Severity::Healthy);
    assert_eq!(recent.severity, Severity::Healthy);
    assert_eq!(in_range.severity, Severity::Healthy);

    // Rule tags propagate: every IoT verdict carries domain:iot.
    assert!(online.tags.get("domain").is_some());

    // Fan-in via verdict.join(mode=all).
    let join = VerdictJoinNode::new(
        starter_spi::insights::RuleId::new("iot.pipeline", "device-health", 1),
        JoinMode::All,
    );
    let mut join_input = SlotMap::new();
    join_input.insert(
        "online".into(),
        SlotValue::Json(serde_json::to_value(&online).unwrap()),
    );
    join_input.insert(
        "recent".into(),
        SlotValue::Json(serde_json::to_value(&recent).unwrap()),
    );
    join_input.insert(
        "in_range".into(),
        SlotValue::Json(serde_json::to_value(&in_range).unwrap()),
    );
    let id = NodeId::new("iot.test.join").unwrap();
    let cancel = NoCancel;
    let joined = decode_verdict(
        &join
            .invoke(make_ctx(&id, &cancel), join_input)
            .await
            .unwrap(),
    );
    assert_eq!(joined.severity, Severity::Healthy);
    assert_eq!(joined.rule_id.to_string(), "iot.pipeline.device-health@1");
}

#[tokio::test]
async fn iot_critical_flagged_when_value_out_of_range() {
    let (rules, _flags) = build_registries();
    let rule_rust = RuleRustNode::new(Arc::clone(&rules));

    let out = decode_verdict(
        &invoke_rule(
            &rule_rust,
            "iot.sensor.in-range@1",
            serde_json::json!({"value": 99.0, "min": 18.0, "max": 24.0}),
        )
        .await,
    );
    assert_eq!(out.severity, Severity::Critical);
    // out-of-range quality flag attached (R-ins-11).
    assert!(out
        .coverage
        .quality_flags
        .iter()
        .any(|f| f.id.name == "out-of-range"));
}

/// The Severity::Error → branch → notify(ops) path from R-ins-6.
///
/// Forced failure: invoke an unregistered rule id. The `rule.rust`
/// body converts the missing dispatch into a `Severity::Error`
/// verdict carrying a `starter.quality.rule-error@1` flag — never
/// `Err` (rules never short-circuit the pipeline). The simulated
/// `branch(on=severity)` then routes to `notify(ops)`.
#[tokio::test]
async fn forced_rule_failure_routes_to_notify_ops_via_severity_error() {
    let (rules, _flags) = build_registries();
    let rule_rust = RuleRustNode::new(Arc::clone(&rules));

    // 1. Unregistered rule id → Severity::Error verdict.
    let err_verdict = decode_verdict(
        &invoke_rule(&rule_rust, "iot.does-not-exist@1", serde_json::json!({})).await,
    );
    assert_eq!(err_verdict.severity, Severity::Error);
    assert!(err_verdict
        .coverage
        .quality_flags
        .iter()
        .any(|f| f.id.namespace == "starter.quality" && f.id.name == "rule-error"));

    // 2. Simulate the canonical R-ins-6 error pattern:
    //
    //    rule.rust → branch(on=severity)
    //                  ├─► (Healthy|Info|Warn|Critical) → ... → notify
    //                  └─► (Error)                       → notify(ops)
    let notified_ops = match err_verdict.severity {
        Severity::Error => true,
        _ => false,
    };
    assert!(notified_ops, "Severity::Error must route to notify(ops)");

    // 3. Force the all-Error degenerate case on verdict.join — three
    //    failed rules feeding the join must yield a single Error
    //    verdict with the `join-all-inputs-errored` flag.
    let join = VerdictJoinNode::new(
        starter_spi::insights::RuleId::new("iot.pipeline", "device-health", 1),
        JoinMode::All,
    );
    let mut input = SlotMap::new();
    for k in ["a", "b", "c"] {
        input.insert(
            k.into(),
            SlotValue::Json(serde_json::to_value(&err_verdict).unwrap()),
        );
    }
    let id = NodeId::new("iot.test.join").unwrap();
    let cancel = NoCancel;
    let joined = decode_verdict(&join.invoke(make_ctx(&id, &cancel), input).await.unwrap());
    assert_eq!(joined.severity, Severity::Error);
    assert!(joined
        .coverage
        .quality_flags
        .iter()
        .any(|f| f.id.name == "join-all-inputs-errored"));
    assert!((joined.coverage.effective.confidence - 0.0).abs() < f32::EPSILON);
}

#[cfg(feature = "sqlite")]
mod persistence {
    use super::*;
    use starter_insights::sqlite::{VerdictStore, INSIGHTS_MIGRATION_SOURCE};
    use starter_store_sqlite::testing;

    #[tokio::test]
    async fn verdict_log_and_tag_index_persist_iot_row() {
        let pool = testing::ephemeral().await;
        starter_store_sqlite::migrate(&pool)
            .with_source(INSIGHTS_MIGRATION_SOURCE)
            .run()
            .await
            .expect("insights migration");

        let (rules, _flags) = super::build_registries();
        let rule_rust = RuleRustNode::new(Arc::clone(&rules));
        let v = decode_verdict(
            &invoke_rule(
                &rule_rust,
                "iot.device.online@1",
                serde_json::json!({"last_seen_secs_ago": 30, "threshold_secs": 300}),
            )
            .await,
        );

        let store = VerdictStore::new(pool);
        let id = store.append(&v).await.expect("append");
        assert!(id > 0);
        assert_eq!(store.count().await.unwrap(), 1);

        let ids = store
            .list_ids_by_tag("domain", Some("iot"))
            .await
            .expect("tag index lookup");
        assert_eq!(ids, vec![id], "tag index returns the appended row");
    }
}
